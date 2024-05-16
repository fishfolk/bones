// TODO
#![allow(missing_docs)]

use bones_matchmaker_proto::PLAY_ALPN;
use bytes::Bytes;
use iroh_net::NodeAddr;
use tracing::{info, warn};

use crate::networking::get_network_endpoint;

use super::{
    BoxedNonBlockingSocket, GameMessage, NetworkSocket, SocketTarget, MAX_PLAYERS, RUNTIME,
};

/// The [`NetworkSocket`] implementation.
#[derive(Debug, Clone)]
pub struct Socket {
    ///
    pub connections: [Option<iroh_quinn::Connection>; MAX_PLAYERS],
    pub ggrs_receiver: async_channel::Receiver<(usize, GameMessage)>,
    pub reliable_receiver: async_channel::Receiver<(usize, Vec<u8>)>,
    pub player_idx: usize,
    pub player_count: usize,
    /// ID for current match, messages received that do not match ID are dropped.
    pub match_id: u8,
}

impl Socket {
    pub fn new(
        player_idx: usize,
        connections: [Option<iroh_quinn::Connection>; MAX_PLAYERS],
    ) -> Self {
        let (ggrs_sender, ggrs_receiver) = async_channel::unbounded();
        let (reliable_sender, reliable_receiver) = async_channel::unbounded();

        // Spawn tasks to receive network messages from each peer
        #[allow(clippy::needless_range_loop)]
        for i in 0..MAX_PLAYERS {
            if let Some(conn) = connections[i].clone() {
                let ggrs_sender = ggrs_sender.clone();

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
        }

        Self {
            player_idx,
            player_count: connections.iter().flatten().count() + 1,
            connections,
            ggrs_receiver,
            reliable_receiver,
            match_id: 0,
        }
    }
}

impl NetworkSocket for Socket {
    fn send_reliable(&self, target: SocketTarget, message: &[u8]) {
        let message = Bytes::copy_from_slice(message);

        match target {
            SocketTarget::Player(i) => {
                let conn = self.connections[i].as_ref().unwrap().clone();

                RUNTIME.spawn(async move {
                    let mut stream = conn.open_uni().await.unwrap();
                    stream.write_chunk(message).await.unwrap();
                    stream.finish().await.unwrap();
                });
            }
            SocketTarget::All => {
                for conn in &self.connections {
                    if let Some(conn) = conn.clone() {
                        let message = message.clone();
                        RUNTIME.spawn(async move {
                            let mut stream = conn.open_uni().await.unwrap();
                            stream.write_chunk(message).await.unwrap();
                            stream.finish().await.unwrap();
                        });
                    }
                }
            }
        }
    }

    fn recv_reliable(&self) -> Vec<(usize, Vec<u8>)> {
        let mut messages = Vec::new();
        while let Ok(message) = self.reliable_receiver.try_recv() {
            messages.push(message);
        }
        messages
    }

    fn ggrs_socket(&self) -> BoxedNonBlockingSocket {
        BoxedNonBlockingSocket(Box::new(self.clone()))
    }

    fn close(&self) {
        for conn in self.connections.iter().flatten() {
            conn.close(0u8.into(), &[]);
        }
    }

    fn player_idx(&self) -> usize {
        self.player_idx
    }

    fn player_count(&self) -> usize {
        self.player_count
    }

    fn player_is_local(&self) -> [bool; MAX_PLAYERS] {
        std::array::from_fn(|i| self.connections[i].is_none() && i < self.player_count)
    }

    fn increment_match_id(&mut self) {
        self.match_id = self.match_id.wrapping_add(1);
    }
}

pub(super) async fn establish_peer_connections(
    player_idx: usize,
    player_count: usize,
    peer_addrs: [Option<NodeAddr>; MAX_PLAYERS],
    conn: Option<iroh_quinn::Connection>,
) -> [Option<iroh_quinn::Connection>; MAX_PLAYERS] {
    let mut peer_connections = std::array::from_fn(|_| None);

    if let Some(conn) = conn {
        // Set the connection to the matchmaker for player 0
        peer_connections[0] = Some(conn);
    }

    let ep = get_network_endpoint().await;

    // For every peer with a player index that is higher than ours, wait for
    // them to connect to us.
    let range = (player_idx + 1)..player_count;
    info!(players=?range, "Waiting for {} peer connections", range.len());
    for _ in range {
        // Wait for connection
        let mut conn = ep.accept().await.unwrap();
        let alpn = conn.alpn().await.unwrap();
        if alpn.as_bytes() != PLAY_ALPN {
            panic!("invalid ALPN: {}", alpn);
        }
        let conn = conn.await.expect("Could not accept incomming connection");

        // Receive the player index
        let idx = {
            let mut buf = [0; 1];
            let mut channel = conn.accept_uni().await.unwrap();
            channel.read_exact(&mut buf).await.unwrap();

            buf[0] as usize
        };
        assert!(idx < MAX_PLAYERS, "Invalid player index");

        peer_connections[idx] = Some(conn);
    }

    // For every peer with a player index lower than ours, connect to them.
    let start_range = if peer_connections[0].is_some() { 1 } else { 0 };
    let range = start_range..player_idx;
    info!(players=?range, "Connecting to {} peers", range.len());
    for i in range {
        let addr = peer_addrs[i].as_ref().unwrap();
        let conn = ep
            .connect(addr.clone(), PLAY_ALPN)
            .await
            .expect("Could not connect to peer");

        // Send player index
        let mut channel = conn.open_uni().await.unwrap();
        channel.write(&[player_idx as u8]).await.unwrap();
        channel.finish().await.unwrap();

        peer_connections[i] = Some(conn);
    }

    peer_connections
}

impl ggrs::NonBlockingSocket<usize> for Socket {
    fn send_to(&mut self, msg: &ggrs::Message, addr: &usize) {
        let msg = GameMessage {
            // Consider a way we can send message by reference and avoid clone?
            message: msg.clone(),
            match_id: self.match_id,
        };
        let conn = self.connections[*addr].as_ref().unwrap();

        let msg_bytes = postcard::to_allocvec(&msg).unwrap();
        conn.send_datagram(Bytes::copy_from_slice(&msg_bytes[..]))
            .ok();
    }

    fn receive_all_messages(&mut self) -> Vec<(usize, ggrs::Message)> {
        let mut messages = Vec::new();
        while let Ok(message) = self.ggrs_receiver.try_recv() {
            if message.1.match_id == self.match_id {
                messages.push((message.0, message.1.message));
            }
        }
        messages
    }
}
