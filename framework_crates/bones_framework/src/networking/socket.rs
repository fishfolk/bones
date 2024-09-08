// TODO
#![allow(missing_docs)]

use bones_matchmaker_proto::PLAY_ALPN;
use bytes::Bytes;
use iroh_net::NodeAddr;
use tracing::{info, warn};

use crate::networking::get_network_endpoint;

use super::{GameMessage, NetworkSocket, SocketTarget, RUNTIME};

/// The [`NetworkSocket`] implementation.
#[derive(Debug, Clone)]
pub struct Socket {
    pub connections: Vec<(u32, iroh_quinn::Connection)>,
    pub ggrs_receiver: async_channel::Receiver<(u32, GameMessage)>,
    pub reliable_receiver: async_channel::Receiver<(u32, Vec<u8>)>,
    pub player_idx: u32,
    pub player_count: u32,
    /// ID for current match, messages received that do not match ID are dropped.
    pub match_id: u8,
}

impl Socket {
    pub fn new(player_idx: u32, connections: Vec<(u32, iroh_quinn::Connection)>) -> Self {
        let (ggrs_sender, ggrs_receiver) = async_channel::unbounded();
        let (reliable_sender, reliable_receiver) = async_channel::unbounded();

        // Spawn tasks to receive network messages from each peer
        for (i, conn) in &connections {
            let ggrs_sender = ggrs_sender.clone();
            let i = *i;

            // Unreliable message receiver
            let conn_ = conn.clone();
            RUNTIME.spawn(async move {
                let conn = conn_;

                #[cfg(feature = "debug-network-slowdown")]
                use turborand::prelude::*;
                #[cfg(feature = "debug-network-slowdown")]
                let rng = AtomicRng::new();

                loop {
                    tokio::select! {
                        closed = conn.closed() => {
                            warn!("Connection error: {closed}");
                            break;
                        }
                        datagram_result = conn.read_datagram() => match datagram_result {
                            Ok(data) => {
                                let message: GameMessage = postcard::from_bytes(&data)
                                .expect("Could not deserialize net message");

                                // Debugging code to introduce artificial latency
                                #[cfg(feature = "debug-network-slowdown")]
                                {
                                    use async_timer::Oneshot;
                                    async_timer::oneshot::Timer::new(
                                        std::time::Duration::from_millis(
                                            (rng.f32_normalized() * 100.0) as u64 + 1,
                                        ),
                                    )
                                    .await;
                                }
                                if ggrs_sender.send((i, message)).await.is_err() {
                                    break;
                                }
                            }
                            Err(e) => {
                                warn!("Connection error: {e}");
                            }
                        }
                    }
                }
            });

            // Reliable message receiver
            let reliable_sender = reliable_sender.clone();
            let conn = conn.clone();
            RUNTIME.spawn(async move {
                #[cfg(feature = "debug-network-slowdown")]
                use turborand::prelude::*;
                #[cfg(feature = "debug-network-slowdown")]
                let rng = AtomicRng::new();

                loop {
                    tokio::select! {
                        closed = conn.closed() => {
                            warn!("Connection error: {closed}");
                            break;
                        }
                        result = conn.accept_uni() => match result {
                            Ok(mut stream) => {
                                let data = stream.read_to_end(4096).await.expect("Network read error");

                                // Debugging code to introduce artificial latency
                                #[cfg(feature = "debug-network-slowdown")]
                                {
                                    use async_timer::Oneshot;
                                    async_timer::oneshot::Timer::new(
                                        std::time::Duration::from_millis(
                                            (rng.f32_normalized() * 100.0) as u64 + 1,
                                        ),
                                    )
                                    .await;
                                }
                                if reliable_sender.send((i, data)).await.is_err() {
                                    break;
                                }
                            }
                            Err(e) => {
                                warn!("Connection error: {e}");
                            }
                        },
                    }
                }
            });
        }

        Self {
            player_idx,
            player_count: (connections.len() + 1).try_into().unwrap(),
            connections,
            ggrs_receiver,
            reliable_receiver,
            match_id: 0,
        }
    }

    fn get_connection(&self, idx: u32) -> &iroh_quinn::Connection {
        debug_assert!(idx < self.player_count);
        // TODO: if this is too slow, optimize storage
        self.connections
            .iter()
            .find(|(i, _)| *i == idx)
            .map(|(_, c)| c)
            .unwrap()
    }
}

impl NetworkSocket for Socket {
    fn send_reliable(&self, target: SocketTarget, message: &[u8]) {
        let message = Bytes::copy_from_slice(message);

        match target {
            SocketTarget::Player(i) => {
                let conn = self.get_connection(i).clone();

                RUNTIME.spawn(async move {
                    let result = async move {
                        let mut stream = conn.open_uni().await?;
                        stream.write_chunk(message).await?;
                        stream.finish().await?;
                        anyhow::Ok(())
                    };
                    if let Err(err) = result.await {
                        warn!("send reliable to {i} failed: {err:?}");
                    }
                });
            }
            SocketTarget::All => {
                for (_, conn) in &self.connections {
                    let message = message.clone();
                    let conn = conn.clone();
                    RUNTIME.spawn(async move {
                        let result = async move {
                            let mut stream = conn.open_uni().await?;
                            stream.write_chunk(message).await?;
                            stream.finish().await?;
                            anyhow::Ok(())
                        };
                        if let Err(err) = result.await {
                            warn!("send reliable all failed: {err:?}");
                        }
                    });
                }
            }
        }
    }

    fn recv_reliable(&self) -> Vec<(u32, Vec<u8>)> {
        let mut messages = Vec::new();
        while let Ok(message) = self.reliable_receiver.try_recv() {
            messages.push(message);
        }
        messages
    }

    fn ggrs_socket(&self) -> Self {
        self.clone()
    }

    fn close(&self) {
        for (_, conn) in &self.connections {
            conn.close(0u8.into(), &[]);
        }
    }

    fn player_idx(&self) -> u32 {
        self.player_idx
    }

    fn player_count(&self) -> u32 {
        self.player_count
    }

    fn increment_match_id(&mut self) {
        self.match_id = self.match_id.wrapping_add(1);
    }
}

pub(super) async fn establish_peer_connections(
    player_idx: u32,
    player_count: u32,
    peer_addrs: Vec<(u32, NodeAddr)>,
    conn: Option<iroh_quinn::Connection>,
) -> anyhow::Result<Vec<(u32, iroh_quinn::Connection)>> {
    let mut peer_connections = Vec::new();
    let had_og_conn = conn.is_some();
    if let Some(conn) = conn {
        // Set the connection to the matchmaker for player 0
        peer_connections.push((0, conn));
    }

    let ep = get_network_endpoint().await;

    // For every peer with a player index that is higher than ours, wait for
    // them to connect to us.
    let mut in_connections = Vec::new();
    let range = (player_idx + 1)..player_count;
    info!(players=?range, "Waiting for {} peer connections", range.len());
    for i in range {
        // Wait for connection
        let mut conn = ep
            .accept()
            .await
            .ok_or_else(|| anyhow::anyhow!("no connection for {}", i))?;
        let alpn = conn.alpn().await?;
        anyhow::ensure!(
            alpn == PLAY_ALPN,
            "invalid ALPN: {:?}",
            std::str::from_utf8(&alpn).unwrap_or("<bytes>")
        );

        let conn = conn.await?;

        // Receive the player index
        let idx = {
            let mut buf = [0; 4];
            let mut channel = conn.accept_uni().await?;
            channel.read_exact(&mut buf).await?;

            u32::from_le_bytes(buf)
        };

        in_connections.push((idx, conn));
    }

    // For every peer with a player index lower than ours, connect to them.
    let start_range = if had_og_conn { 1 } else { 0 };
    let range = start_range..player_idx;
    info!(players=?range, "Connecting to {} peers", range.len());

    let mut out_connections = Vec::new();
    for i in range {
        let (_, addr) = peer_addrs.iter().find(|(idx, _)| *idx == i).unwrap();
        let conn = ep.connect(addr.clone(), PLAY_ALPN).await?;

        // Send player index
        let mut channel = conn.open_uni().await?;
        channel.write(&player_idx.to_le_bytes()).await?;
        channel.finish().await?;

        out_connections.push((i, conn));
    }

    peer_connections.extend(out_connections);
    peer_connections.extend(in_connections);

    Ok(peer_connections)
}

impl ggrs::NonBlockingSocket<usize> for Socket {
    fn send_to(&mut self, msg: &ggrs::Message, addr: &usize) {
        let msg = GameMessage {
            // Consider a way we can send message by reference and avoid clone?
            message: msg.clone(),
            match_id: self.match_id,
        };
        let conn = self.get_connection((*addr).try_into().unwrap());

        let msg_bytes = postcard::to_allocvec(&msg).unwrap();
        conn.send_datagram(Bytes::copy_from_slice(&msg_bytes[..]))
            .ok();
    }

    fn receive_all_messages(&mut self) -> Vec<(usize, ggrs::Message)> {
        let mut messages = Vec::new();
        while let Ok(message) = self.ggrs_receiver.try_recv() {
            if message.1.match_id == self.match_id {
                messages.push((message.0 as usize, message.1.message));
            }
        }
        messages
    }
}
