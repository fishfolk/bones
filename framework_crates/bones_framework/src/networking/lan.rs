//! LAN matchmaking and socket implementation.
//!
//! ## Matchmaking
//!
//! The LAN matchmaker works by allowing the player to start a match and wait for people to join it,
//! or to join player's started match.
//!
//! Communication happens directly between LAN peers over the QUIC protocol.

// TODO
#![allow(missing_docs)]

use std::{net::IpAddr, time::Duration};

use iroh::{endpoint::get_remote_node_id, NodeAddr};
use mdns_sd::{ServiceDaemon, ServiceInfo};
use smallvec::SmallVec;
use tracing::warn;

use crate::networking::socket::establish_peer_connections;
use crate::utils::BiChannelServer;

use super::socket::Socket;
use super::*;

/// Service discover info and ping.
#[derive(Clone)]
pub struct ServerInfo {
    /// mutli-cast dns service discover info.
    pub service: ServiceInfo,
    /// The ping in milliseconds
    pub ping: Option<u16>,
}

/// Receiver for LAN service discovery channel.
#[derive(Clone)]
pub struct ServiceDiscoveryReceiver(mdns_sd::Receiver<mdns_sd::ServiceEvent>);

/// Channel used to do matchmaking over LAN.
///
/// Spawns a task to handle the actual matchmaking.
static LAN_MATCHMAKER: Lazy<LanMatchmaker> = Lazy::new(|| {
    let (client, server) = bi_channel();

    RUNTIME.spawn(async move {
        if let Err(err) = lan_matchmaker(server).await {
            warn!("lan matchmaker failed: {err:?}");
        }
    });

    LanMatchmaker(client)
});

static MDNS: Lazy<ServiceDaemon> =
    Lazy::new(|| ServiceDaemon::new().expect("Couldn't start MDNS service discovery thread."));

const MDNS_SERVICE_TYPE: &str = "_jumpy._udp.local.";

#[derive(DerefMut, Deref)]
struct Pinger(BiChannelClient<PingerRequest, PingerResponse>);

type PingerRequest = SmallVec<[IpAddr; 10]>;
type PingerResponse = SmallVec<[(IpAddr, Option<u16>); 10]>;

static PINGER: Lazy<Pinger> = Lazy::new(|| {
    let (client, server) = bi_channel();

    std::thread::spawn(move || pinger(server));

    Pinger(client)
});

/// Host a server.
///
/// The number of players is limited to `u32::MAX`.
pub fn start_server(server: ServerInfo, player_count: u32) {
    MDNS.register(server.service)
        .expect("Could not register MDNS service.");
    LAN_MATCHMAKER
        .try_send(LanMatchmakerRequest::StartServer { player_count })
        .unwrap();
}

/// Stop hosting a server.
pub fn stop_server(server: &ServerInfo) {
    if let Err(err) = stop_server_by_name(server.service.get_fullname()) {
        warn!("Lan: failed to stop server: {err:?}");
    }
}

/// Stop hosting a server specified by name. (Use [`ServiceInfo::get_fullname()`].)
fn stop_server_by_name(name: &str) -> anyhow::Result<()> {
    loop {
        match MDNS.unregister(name) {
            Ok(_) => break,
            Err(mdns_sd::Error::Again) => (),
            Err(e) => {
                anyhow::bail!("Error unregistering MDNS service: {e}")
            }
        }
    }
    Ok(())
}

/// Wait for players to join a hosted server.
pub fn wait_players(joined_players: &mut usize, server: &ServerInfo) -> Option<NetworkMatchSocket> {
    while let Ok(response) = LAN_MATCHMAKER.try_recv() {
        match response {
            LanMatchmakerResponse::ServerStarted => {}
            LanMatchmakerResponse::PlayerCount(count) => {
                *joined_players = count;
            }
            LanMatchmakerResponse::GameStarting {
                socket,
                player_idx,
                player_count: _,
            } => {
                info!(?player_idx, "Starting network game");
                if let Err(err) = stop_server_by_name(server.service.get_fullname()) {
                    warn!("Lan: failed to stop server: {err:?}");
                }
                return Some(NetworkMatchSocket(Arc::new(socket)));
            }
        }
    }
    None
}

/// Join a server hosted by someone else.
pub fn join_server(server: &ServerInfo) -> anyhow::Result<()> {
    let addr_raw = server
        .service
        .get_properties()
        .get_property_val_str("node-addr")
        .ok_or_else(|| anyhow::anyhow!("missing node-addr property from discovery"))?;
    let addr_raw = hex::decode(addr_raw)?;
    let addr: NodeAddr = postcard::from_bytes(&addr_raw)?;
    LAN_MATCHMAKER
        .try_send(lan::LanMatchmakerRequest::JoinServer { addr })
        .unwrap();
    Ok(())
}

/// Leave a joined server.
pub fn leave_server() {
    LAN_MATCHMAKER
        .try_send(LanMatchmakerRequest::StopJoin)
        .unwrap();
}

/// Wait for a joined game to start.
pub fn wait_game_start() -> Option<NetworkMatchSocket> {
    while let Ok(message) = LAN_MATCHMAKER.try_recv() {
        match message {
            LanMatchmakerResponse::ServerStarted | LanMatchmakerResponse::PlayerCount(_) => {}
            LanMatchmakerResponse::GameStarting {
                socket,
                player_idx,
                player_count: _,
            } => {
                info!(?player_idx, "Starting network game");
                return Some(NetworkMatchSocket(Arc::new(socket)));
            }
        }
    }
    None
}

/// Update server pings and turn on service discovery.
pub fn prepare_to_join(
    servers: &mut Vec<ServerInfo>,
    service_discovery_recv: &mut Option<ServiceDiscoveryReceiver>,
    ping_update_timer: &Timer,
) {
    // Update server pings
    if ping_update_timer.finished() {
        PINGER
            .try_send(
                servers
                    .iter()
                    .map(|x| *x.service.get_addresses().iter().next().unwrap())
                    .collect(),
            )
            .ok();
    }
    if let Ok(pings) = PINGER.try_recv() {
        for (server, ping) in pings {
            for info in servers.iter_mut() {
                if info.service.get_addresses().contains(&server) {
                    info.ping = ping;
                }
            }
        }
    }

    let events = service_discovery_recv.get_or_insert_with(|| {
        ServiceDiscoveryReceiver(
            MDNS.browse(MDNS_SERVICE_TYPE)
                .expect("Couldn't start service discovery"),
        )
    });

    while let Ok(event) = events.0.try_recv() {
        match event {
            mdns_sd::ServiceEvent::ServiceResolved(info) => {
                info!("Found lan service!");
                servers.push(lan::ServerInfo {
                    service: info,
                    ping: None,
                })
            }
            mdns_sd::ServiceEvent::ServiceRemoved(_, full_name) => {
                servers.retain(|server| server.service.get_fullname() != full_name);
            }
            _ => (),
        }
    }
}

/// Get the current host info or create a new one. When there's an existing
/// service but its `service_name` is different, the service is recreated and
/// only then the returned `bool` is `true`.
pub async fn prepare_to_host<'a>(
    host_info: &'a mut Option<ServerInfo>,
    service_name: &str,
) -> (bool, &'a mut ServerInfo) {
    let create_service_info = || async {
        info!("New service hosting");
        let ep = get_network_endpoint().await;
        let mut my_addr = ep.node_addr().await.expect("network endpoint dead");
        my_addr
            .info
            .direct_addresses
            .retain(std::net::SocketAddr::is_ipv4);
        let port = my_addr.info.direct_addresses.first().unwrap().port();
        let mut props = std::collections::HashMap::default();
        let addr_encoded = hex::encode(postcard::to_stdvec(&my_addr).unwrap());
        props.insert("node-addr".to_string(), addr_encoded);
        let service = mdns_sd::ServiceInfo::new(
            MDNS_SERVICE_TYPE,
            service_name,
            service_name,
            "",
            port,
            props,
        )
        .unwrap()
        .enable_addr_auto();
        ServerInfo {
            service,
            ping: None,
        }
    };

    if host_info.is_none() {
        let info = create_service_info().await;
        host_info.replace(info);
    }
    let service_info = host_info.as_mut().unwrap();

    let mut is_recreated = false;
    if service_info.service.get_hostname() != service_name {
        stop_server_by_name(service_info.service.get_fullname()).unwrap();
        is_recreated = true;
        *service_info = create_service_info().await;
    }
    (is_recreated, service_info)
}

/// Implementation of the lan matchmaker task.
///
/// This is a long-running tasks that listens for messages sent through the `LAN_MATCHMAKER`
/// channel.
async fn lan_matchmaker(
    matchmaker_channel: BiChannelServer<LanMatchmakerRequest, LanMatchmakerResponse>,
) -> anyhow::Result<()> {
    while let Ok(request) = matchmaker_channel.recv().await {
        match request {
            // Start server
            LanMatchmakerRequest::StartServer { player_count } => {
                if let Err(err) = lan_start_server(&matchmaker_channel, player_count).await {
                    warn!("lan server failed: {err:?}");
                }
                // Once we are done with server matchmaking
            }
            // Server not running or joining so do nothing
            LanMatchmakerRequest::StopServer => (),
            LanMatchmakerRequest::StopJoin => (),

            // Join a hosted match
            LanMatchmakerRequest::JoinServer { addr } => {
                if let Err(err) = lan_join_server(&matchmaker_channel, addr).await {
                    warn!("failed to join server: {err:?}");
                }
            }
        }
    }

    Ok(())
}

async fn lan_start_server(
    matchmaker_channel: &BiChannelServer<LanMatchmakerRequest, LanMatchmakerResponse>,
    mut player_count: u32,
) -> anyhow::Result<()> {
    info!("Starting LAN server");
    matchmaker_channel
        .send(LanMatchmakerResponse::ServerStarted)
        .await?;

    let mut connections = Vec::new();
    let ep = get_network_endpoint().await;

    loop {
        tokio::select! {
            next_request = matchmaker_channel.recv() => {
                match next_request? {
                    LanMatchmakerRequest::StartServer {
                        player_count: new_player_count,
                    } => {
                        connections.clear();
                        player_count = new_player_count;
                    }
                    LanMatchmakerRequest::StopServer => {
                        break;
                    }
                    LanMatchmakerRequest::StopJoin => {} // Not joining, so don't do anything
                    LanMatchmakerRequest::JoinServer { .. } => {
                        anyhow::bail!("Cannot join server while hosting server");
                    }
                }
            }

            // Handle new connections
            incomming = ep.accept() => {
                let Some(incomming) = incomming else {
                    anyhow::bail!("unable to accept new connections");
                };
                let result = async move {
                    let mut connecting = incomming.accept()?;
                    let alpn = connecting.alpn().await?;
                    anyhow::ensure!(alpn == PLAY_ALPN, "unexpected ALPN");
                    let conn = connecting.await?;
                    anyhow::Ok(conn)
                };

                match result.await {
                    Ok(conn) => {
                        connections.push(conn);
                        let current_players = connections.len() + 1;
                        info!(%current_players, "New player connection");
                    }
                    Err(err) => {
                        warn!("failed to accept connection: {:?}", err);
                        continue;
                    }
                }
            }
        }

        // Discard closed connections
        connections.retain(|conn| {
            if conn.close_reason().is_some() {
                info!("Player closed connection");
                false
            } else {
                true
            }
        });

        let current_players = connections.len();
        let target_players = player_count;
        info!(%current_players, %target_players);

        // If we're ready to start a match
        if connections.len() == (player_count - 1) as usize {
            info!("All players joined.");

            let endpoint = get_network_endpoint().await;

            // Tell all clients we're ready
            for (i, conn) in connections.iter().enumerate() {
                let mut peers = Vec::new();
                connections
                    .iter()
                    .enumerate()
                    .filter(|x| x.0 != i)
                    .for_each(|(i, conn)| {
                        let id = get_remote_node_id(conn).expect("invalid connection");
                        let mut addr = NodeAddr::new(id);
                        if let Some(info) = endpoint.remote_info(id) {
                            if let Some(relay_url) = info.relay_url {
                                addr = addr.with_relay_url(relay_url.relay_url);
                            }
                            addr = addr.with_direct_addresses(
                                info.addrs.into_iter().map(|addr| addr.addr),
                            );
                        }

                        peers.push((u32::try_from(i + 1).unwrap(), addr));
                    });

                let mut uni = conn.open_uni().await?;
                uni.write_all(&postcard::to_vec::<_, 20>(&MatchmakerNetMsg::MatchReady {
                    player_idx: (i + 1).try_into()?,
                    peers,
                    player_count,
                })?)
                .await?;
                uni.finish()?;
                uni.stopped().await?;
            }

            let connections = connections
                .into_iter()
                .enumerate()
                .map(|(i, c)| (u32::try_from(i + 1).unwrap(), c))
                .collect();

            // Send the connections to the game so that it can start the network match.
            matchmaker_channel
                .send(LanMatchmakerResponse::GameStarting {
                    socket: Socket::new(0, connections),
                    player_idx: 0,
                    player_count,
                })
                .await?;
            info!(player_idx=0, %player_count, "Matchmaking finished");

            // Break out of the server loop
            break;

            // If we don't have enough players yet, send the updated player count to the game.
        } else {
            matchmaker_channel
                .send(LanMatchmakerResponse::PlayerCount(current_players))
                .await?;
        }
    }

    Ok(())
}

async fn lan_join_server(
    matchmaker_channel: &BiChannelServer<LanMatchmakerRequest, LanMatchmakerResponse>,
    addr: NodeAddr,
) -> anyhow::Result<()> {
    let ep = get_network_endpoint().await;
    let conn = ep.connect(addr, PLAY_ALPN).await?;

    // Wait for match to start
    let mut uni = conn.accept_uni().await?;
    let bytes = uni.read_to_end(20).await?;
    let message: MatchmakerNetMsg = postcard::from_bytes(&bytes)?;

    match message {
        MatchmakerNetMsg::MatchReady {
            peers: peer_addrs,
            player_idx,
            player_count,
        } => {
            info!(%player_count, %player_idx, ?peer_addrs, "Matchmaking finished");

            let peer_connections =
                establish_peer_connections(player_idx, player_count, peer_addrs, Some(conn))
                    .await?;

            let socket = Socket::new(player_idx, peer_connections);
            info!("Connections established.");

            matchmaker_channel
                .send(LanMatchmakerResponse::GameStarting {
                    socket,
                    player_idx,
                    player_count,
                })
                .await?;
        }
    }
    Ok(())
}

#[derive(Serialize, Deserialize)]
enum MatchmakerNetMsg {
    MatchReady {
        /// The peers they have for the match, with the index in the array being the player index of the peer.
        peers: Vec<(u32, NodeAddr)>,
        /// The player index of the player getting the message.
        player_idx: u32,
        player_count: u32,
    },
}

/// The type of the `LAN_MATCHMAKER` channel.
#[derive(DerefMut, Deref)]
pub struct LanMatchmaker(BiChannelClient<LanMatchmakerRequest, LanMatchmakerResponse>);

/// A request that may be sent to the `LAN_MATCHMAKER`.
#[derive(Debug)]
pub enum LanMatchmakerRequest {
    /// Start matchmaker server
    StartServer {
        /// match player count
        player_count: u32,
    },
    /// Join server
    JoinServer {
        /// Node Addr
        addr: NodeAddr,
    },
    /// Stop matchmaking server
    StopServer,
    /// Stop joining match
    StopJoin,
}

/// A response that may come from the `LAN_MATCHMAKER`.
pub enum LanMatchmakerResponse {
    /// Server started
    ServerStarted,
    /// Server player count
    PlayerCount(usize),
    /// Game is starting
    GameStarting {
        /// Lan socket to game
        socket: Socket,
        /// Local player index
        player_idx: u32,
        /// Game player count
        player_count: u32,
    },
}

fn pinger(server: BiChannelServer<PingerRequest, PingerResponse>) {
    while let Ok(servers) = server.recv_blocking() {
        let mut pings = SmallVec::new();
        for server in servers {
            let start = Instant::now();
            let ping_result =
                ping_rs::send_ping(&server, Duration::from_secs(2), &[1, 2, 3, 4], None);

            let ping = if let Err(e) = ping_result {
                warn!("Error pinging {server}: {e:?}");
                None
            } else {
                Some((Instant::now() - start).as_millis() as u16)
            };

            pings.push((server, ping));
        }
        if server.send_blocking(pings).is_err() {
            break;
        }
    }
}
