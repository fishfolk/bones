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

use iroh_net::{magic_endpoint::get_remote_node_id, NodeAddr};
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

    RUNTIME.spawn(lan_matchmaker(server));
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
pub fn start_server(server: ServerInfo, player_count: usize) {
    MDNS.register(server.service)
        .expect("Could not register MDNS service.");
    LAN_MATCHMAKER
        .try_send(LanMatchmakerRequest::StartServer { player_count })
        .unwrap();
}

/// Stop hosting a server.
pub fn stop_server(server: &ServerInfo) {
    stop_server_by_name(server.service.get_fullname())
}

/// Stop hosting a server specified by name. (Use [`ServiceInfo::get_fullname()`].)
fn stop_server_by_name(name: &str) {
    loop {
        match MDNS.unregister(name) {
            Ok(_) => break,
            Err(mdns_sd::Error::Again) => (),
            Err(e) => {
                panic!("Error unregistering MDNS service: {e}")
            }
        }
    }
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
                loop {
                    match MDNS.unregister(server.service.get_fullname()) {
                        Ok(_) => break,
                        Err(mdns_sd::Error::Again) => (),
                        Err(e) => panic!("Error unregistering MDNS service: {e}"),
                    }
                }
                return Some(NetworkMatchSocket(Arc::new(socket)));
            }
        }
    }
    None
}

/// Join a server hosted by someone else.
pub fn join_server(server: &ServerInfo) {
    let addr_raw = server
        .service
        .get_properties()
        .get_property_val_str("node-addr")
        .unwrap();
    let addr_raw = hex::decode(addr_raw).unwrap();
    let addr: NodeAddr = postcard::from_bytes(&addr_raw).unwrap();
    LAN_MATCHMAKER
        .try_send(lan::LanMatchmakerRequest::JoinServer { addr })
        .unwrap();
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
        let my_addr = ep.my_addr().await.expect("network endpoint dead");
        let port = ep.local_addr().0.port();
        let mut props = std::collections::HashMap::default();
        let addr_encoded = hex::encode(&postcard::to_stdvec(&my_addr).unwrap());
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
        stop_server_by_name(service_info.service.get_fullname());
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
) {
    #[derive(Serialize, Deserialize)]
    enum MatchmakerNetMsg {
        MatchReady {
            /// The peers they have for the match, with the index in the array being the player index of the peer.
            peers: [Option<NodeAddr>; MAX_PLAYERS],
            /// The player index of the player getting the message.
            player_idx: usize,
            player_count: usize,
        },
    }

    while let Ok(request) = matchmaker_channel.recv().await {
        match request {
            // Start server
            LanMatchmakerRequest::StartServer { mut player_count } => {
                info!("Starting LAN server");
                matchmaker_channel
                    .send(LanMatchmakerResponse::ServerStarted)
                    .await
                    .unwrap();

                let mut connections = Vec::new();
                let ep = get_network_endpoint().await;

                loop {
                    tokio::select! {
                        next_request = matchmaker_channel.recv() => {
                            let Ok(next_request) = next_request else {
                                break;
                            };

                            match next_request {
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
                                    error!("Cannot join server while hosting server");
                                }
                            }
                        }

                        // Handle new connections
                        new_connection = ep.accept() => {
                            let Some(mut new_connection) = new_connection else {
                                break;
                            };
                            let Ok(alpn) = new_connection.alpn().await else {
                                continue;
                            };
                            if alpn.as_bytes() != PLAY_ALPN {
                                warn!("unexpected ALPN: {alpn}");
                                continue;
                            }
                            let Some(conn) = new_connection.await.ok() else {
                                continue;
                            };
                            connections.push(conn);
                            let current_players = connections.len() + 1;
                            info!(%current_players, "New player connection");
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
                    if connections.len() == player_count - 1 {
                        info!("All players joined.");

                        let endpoint = get_network_endpoint().await;

                        // Tell all clients we're ready
                        for (i, conn) in connections.iter().enumerate() {
                            let mut peers = std::array::from_fn(|_| None);
                            connections
                                .iter()
                                .enumerate()
                                .filter(|x| x.0 != i)
                                .for_each(|(i, conn)| {
                                    let id = get_remote_node_id(&conn).expect("invalid connection");
                                    let mut addr = NodeAddr::new(id);
                                    if let Some(info) = endpoint.connection_info(id) {
                                        if let Some(relay_url) = info.relay_url {
                                            addr = addr.with_relay_url(relay_url.relay_url);
                                        }
                                        addr = addr.with_direct_addresses(
                                            info.addrs.into_iter().map(|addr| addr.addr),
                                        );
                                    }

                                    peers[i + 1] = Some(addr);
                                });

                            let mut uni = conn.open_uni().await.unwrap();
                            uni.write_all(
                                &postcard::to_vec::<_, 20>(&MatchmakerNetMsg::MatchReady {
                                    player_idx: i + 1,
                                    peers,
                                    player_count,
                                })
                                .unwrap(),
                            )
                            .await
                            .unwrap();
                            uni.finish().await.unwrap();
                        }

                        // Collect the list of client connections
                        let connections = std::array::from_fn(|i| {
                            if i == 0 {
                                None
                            } else {
                                connections.get(i - 1).cloned()
                            }
                        });

                        // Send the connections to the game so that it can start the network match.
                        matchmaker_channel
                            .try_send(LanMatchmakerResponse::GameStarting {
                                socket: Socket::new(0, connections),
                                player_idx: 0,
                                player_count,
                            })
                            .ok();
                        info!(player_idx=0, %player_count, "Matchmaking finished");

                        // Break out of the server loop
                        break;

                    // If we don't have enough players yet, send the updated player count to the game.
                    } else if matchmaker_channel
                        .try_send(LanMatchmakerResponse::PlayerCount(current_players))
                        .is_err()
                    {
                        break;
                    }
                }

                // Once we are done with server matchmaking
            }
            // Server not running or joining so do nothing
            LanMatchmakerRequest::StopServer => (),
            LanMatchmakerRequest::StopJoin => (),

            // Join a hosted match
            LanMatchmakerRequest::JoinServer { addr } => {
                let conn = get_network_endpoint()
                    .await
                    .connect(addr, PLAY_ALPN)
                    .await
                    .expect("Could not connect to server");

                // Wait for match to start
                let mut uni = conn.accept_uni().await.unwrap();
                let bytes = uni.read_to_end(20).await.unwrap();
                let message: MatchmakerNetMsg = postcard::from_bytes(&bytes).unwrap();

                match message {
                    MatchmakerNetMsg::MatchReady {
                        peers: peer_addrs,
                        player_idx,
                        player_count,
                    } => {
                        info!(%player_count, %player_idx, ?peer_addrs, "Matchmaking finished");

                        let peer_connections = establish_peer_connections(
                            player_idx,
                            player_count,
                            peer_addrs,
                            Some(conn),
                        )
                        .await;

                        let socket = Socket::new(player_idx, peer_connections);
                        info!("Connections established.");

                        matchmaker_channel
                            .try_send(LanMatchmakerResponse::GameStarting {
                                socket,
                                player_idx,
                                player_count,
                            })
                            .ok();
                    }
                }
            }
        }
    }
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
        player_count: usize,
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
        player_idx: usize,
        /// Game player count
        player_count: usize,
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
