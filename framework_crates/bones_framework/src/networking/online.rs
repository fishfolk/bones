#![doc = include_str!("./online.md")]
// TODO
#![allow(missing_docs)]

use std::sync::Arc;

use bevy_tasks::IoTaskPool;
use bones_matchmaker_proto::{MatchInfo, MatchmakerRequest, MatchmakerResponse, ALPN};
use bytes::Bytes;
use futures_lite::future;
use iroh_net::NodeId;
use iroh_quinn::Connection;
use once_cell::sync::Lazy;
use tracing::{info, warn};

use crate::{networking::NetworkMatchSocket, prelude::*};

use super::{
    BoxedNonBlockingSocket, GameMessage, NetworkSocket, SocketTarget, MAX_PLAYERS, NETWORK_ENDPOINT,
};

#[derive(Default, PartialEq, Eq, Clone, Copy)]
pub enum SearchState {
    #[default]
    Connecting,
    Searching,
    WaitingForPlayers(usize),
}

/// Online matchmaker channel
pub static ONLINE_MATCHMAKER: Lazy<OnlineMatchmaker> = Lazy::new(|| {
    let (client, server) = bi_channel();

    IoTaskPool::get().spawn(online_matchmaker(server)).detach();

    OnlineMatchmaker(client)
});

/// Channel to exchagne messages with matchmaking server
#[derive(DerefMut, Deref)]
pub struct OnlineMatchmaker(BiChannelClient<OnlineMatchmakerRequest, OnlineMatchmakerResponse>);

/// Online matchmaker request
#[derive(Debug)]
pub enum OnlineMatchmakerRequest {
    SearchForGame { id: NodeId, player_count: usize },
    StopSearch,
}

/// Online matchmaker response
#[derive(Debug)]
pub enum OnlineMatchmakerResponse {
    Searching,
    PlayerCount(usize),
    GameStarting {
        online_socket: OnlineSocket,
        player_idx: usize,
        player_count: usize,
    },
}

async fn online_matchmaker(
    matchmaker_channel: BiChannelServer<OnlineMatchmakerRequest, OnlineMatchmakerResponse>,
) {
    while let Ok(message) = matchmaker_channel.recv().await {
        match message {
            OnlineMatchmakerRequest::SearchForGame { id, player_count } => {
                info!("Connecting to online matchmaker");
                let conn = NETWORK_ENDPOINT.connect(id.into(), ALPN).await.unwrap();
                info!("Connected to online matchmaker");

                matchmaker_channel
                    .try_send(OnlineMatchmakerResponse::Searching)
                    .unwrap();

                // Send a match request to the server
                let (mut send, mut recv) = conn.open_bi().await.unwrap();

                let message = MatchmakerRequest::RequestMatch(MatchInfo {
                    client_count: player_count.try_into().unwrap(),
                    match_data: b"jumpy_default_game".to_vec(),
                });
                info!(request=?message, "Sending match request");
                let message = postcard::to_allocvec(&message).unwrap();
                send.write_all(&message).await.unwrap();
                send.finish().await.unwrap();

                let response = recv.read_to_end(256).await.unwrap();
                let message: MatchmakerResponse = postcard::from_bytes(&response).unwrap();

                if let MatchmakerResponse::Accepted = message {
                    info!("Waiting for match...");
                } else {
                    panic!("Invalid response from matchmaker");
                }

                loop {
                    let recv_ui_message = matchmaker_channel.recv();
                    let recv_online_matchmaker = conn.accept_uni();

                    let next_message = futures_lite::future::or(
                        async move { either::Left(recv_ui_message.await) },
                        async move { either::Right(recv_online_matchmaker.await) },
                    )
                    .await;

                    match next_message {
                        // UI message
                        either::Either::Left(message) => {
                            let message = message.unwrap();

                            match message {
                                OnlineMatchmakerRequest::SearchForGame { .. } => {
                                    panic!("Unexpected message from UI");
                                }
                                OnlineMatchmakerRequest::StopSearch => {
                                    info!("Canceling online search");
                                    break;
                                }
                            }
                        }

                        // Matchmaker message
                        either::Either::Right(recv) => {
                            let mut recv = recv.unwrap();
                            let message = recv.read_to_end(256).await.unwrap();
                            let message: MatchmakerResponse =
                                postcard::from_bytes(&message).unwrap();

                            match message {
                                MatchmakerResponse::ClientCount(count) => {
                                    info!("Online match player count: {count}");
                                    matchmaker_channel
                                        .try_send(OnlineMatchmakerResponse::PlayerCount(count as _))
                                        .unwrap();
                                }
                                MatchmakerResponse::Success {
                                    random_seed,
                                    player_idx,
                                    client_count,
                                } => {
                                    info!(%random_seed, %player_idx, player_count=%client_count, "Online match complete");
                                    let online_socket = OnlineSocket::new(
                                        player_idx as usize,
                                        client_count as usize,
                                        conn,
                                    );

                                    matchmaker_channel
                                        .try_send(OnlineMatchmakerResponse::GameStarting {
                                            online_socket,
                                            player_idx: player_idx as _,
                                            player_count: client_count as _,
                                        })
                                        .unwrap();
                                    break;
                                }
                                _ => panic!("Unexpected message from matchmaker"),
                            }
                        }
                    }
                }
            }
            OnlineMatchmakerRequest::StopSearch => (), // Not searching, don't do anything
        }
    }
}

#[derive(Debug, Clone)]
pub struct OnlineSocket {
    pub conn: Connection,
    pub ggrs_receiver: async_channel::Receiver<(usize, GameMessage)>,
    pub reliable_receiver: async_channel::Receiver<(usize, Vec<u8>)>,
    pub player_idx: usize,
    pub player_count: usize,
    /// ID for current match, messages received that do not match ID are dropped.
    pub match_id: u8,
}

impl OnlineSocket {
    pub fn new(player_idx: usize, player_count: usize, conn: Connection) -> Self {
        let (ggrs_sender, ggrs_receiver) = async_channel::unbounded();
        let (reliable_sender, reliable_receiver) = async_channel::unbounded();

        let task_pool = IoTaskPool::get();

        let conn_ = conn.clone();
        task_pool
            .spawn(async move {
                let conn = conn_;
                loop {
                    let event = future::or(async { either::Left(conn.closed().await) }, async {
                        either::Right(conn.read_datagram().await)
                    })
                    .await;

                    match event {
                        either::Either::Left(closed) => {
                            warn!("Connection error: {closed}");
                            break;
                        }
                        either::Either::Right(datagram_result) => match datagram_result {
                            Ok(data) => {
                                let message: bones_matchmaker_proto::RecvProxyMessage =
                                    postcard::from_bytes(&data)
                                        .expect("Could not deserialize net message");
                                let player = message.from_client;
                                let message = postcard::from_bytes(&message.message).unwrap();

                                if ggrs_sender.send((player as _, message)).await.is_err() {
                                    break;
                                }
                            }
                            Err(e) => {
                                warn!("Connection error: {e}");
                            }
                        },
                    }
                }
            })
            .detach();

        let conn_ = conn.clone();
        task_pool
            .spawn(async move {
                let conn = conn_;
                loop {
                    let event = future::or(async { either::Left(conn.closed().await) }, async {
                        either::Right(conn.accept_uni().await)
                    })
                    .await;

                    match event {
                        either::Either::Left(closed) => {
                            warn!("Connection error: {closed}");
                            break;
                        }
                        either::Either::Right(result) => match result {
                            Ok(mut stream) => {
                                let data =
                                    stream.read_to_end(4096).await.expect("Network read error");
                                let message: bones_matchmaker_proto::RecvProxyMessage =
                                    postcard::from_bytes(&data).unwrap();

                                if reliable_sender
                                    .send((message.from_client as usize, message.message))
                                    .await
                                    .is_err()
                                {
                                    break;
                                }
                            }
                            Err(e) => {
                                warn!("Connection error: {e}");
                            }
                        },
                    }
                }
            })
            .detach();

        Self {
            conn,
            ggrs_receiver,
            reliable_receiver,
            player_idx,
            player_count,
            match_id: 0,
        }
    }
}

impl NetworkSocket for OnlineSocket {
    fn ggrs_socket(&self) -> BoxedNonBlockingSocket {
        BoxedNonBlockingSocket(Box::new(self.clone()))
    }

    fn send_reliable(&self, target: SocketTarget, message: &[u8]) {
        let task_pool = IoTaskPool::get();
        let target_client = match target {
            SocketTarget::Player(player) => bones_matchmaker_proto::TargetClient::One(player as _),
            SocketTarget::All => bones_matchmaker_proto::TargetClient::All,
        };
        let message = bones_matchmaker_proto::SendProxyMessage {
            target_client,
            message: message.into(),
        };

        let conn = self.conn.clone();
        task_pool
            .spawn(async move {
                let mut send = conn.open_uni().await.unwrap();

                send.write_all(&postcard::to_allocvec(&message).unwrap())
                    .await
                    .unwrap();
                send.finish().await.unwrap();
            })
            .detach();
    }

    fn recv_reliable(&self) -> Vec<(usize, Vec<u8>)> {
        let mut messages = Vec::new();
        while let Ok(message) = self.reliable_receiver.try_recv() {
            messages.push(message);
        }
        messages
    }

    fn close(&self) {
        self.conn.close(0u8.into(), &[]);
    }

    fn player_idx(&self) -> usize {
        self.player_idx
    }

    fn player_is_local(&self) -> [bool; MAX_PLAYERS] {
        std::array::from_fn(|i| i == self.player_idx)
    }

    fn player_count(&self) -> usize {
        self.player_count
    }

    fn increment_match_id(&mut self) {
        // This is wrapping addition
        self.match_id = self.match_id.wrapping_add(1);
    }
}

impl ggrs::NonBlockingSocket<usize> for OnlineSocket {
    fn send_to(&mut self, msg: &ggrs::Message, addr: &usize) {
        let msg = GameMessage {
            message: msg.clone(),
            match_id: self.match_id,
        };
        let message = bones_matchmaker_proto::SendProxyMessage {
            target_client: bones_matchmaker_proto::TargetClient::One(*addr as u8),
            message: postcard::to_allocvec(&msg).unwrap(),
        };
        let msg_bytes = postcard::to_allocvec(&message).unwrap();
        self.conn
            .send_datagram(Bytes::copy_from_slice(&msg_bytes[..]))
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

/// Search for game with `matchmaking_server` and `player_count`
pub fn start_search_for_game(matchmaking_server: NodeId, player_count: usize) {
    // TODO remove
    info!("Starting search for online game with player count {player_count}");
    ONLINE_MATCHMAKER
        .try_send(OnlineMatchmakerRequest::SearchForGame {
            id: matchmaking_server,
            player_count,
        })
        .unwrap()
}

/// Stop searching for game
pub fn stop_search_for_game() -> Result<(), async_channel::TrySendError<OnlineMatchmakerRequest>> {
    ONLINE_MATCHMAKER.try_send(OnlineMatchmakerRequest::StopSearch)
}

/// Update state of game matchmaking, update `search_state`, return [`NetworkMatchSocket`] once connected.
pub fn update_search_for_game(search_state: &mut SearchState) -> Option<NetworkMatchSocket> {
    while let Ok(message) = ONLINE_MATCHMAKER.try_recv() {
        match message {
            OnlineMatchmakerResponse::Searching => *search_state = SearchState::Searching,
            OnlineMatchmakerResponse::PlayerCount(count) => {
                warn!("Waiting for players: {count}");
                *search_state = SearchState::WaitingForPlayers(count)
            }
            OnlineMatchmakerResponse::GameStarting {
                online_socket,
                player_idx,
                player_count: _,
            } => {
                info!(?player_idx, "Starting network game");

                *search_state = default();

                return Some(NetworkMatchSocket(Arc::new(online_socket)));
            }
        }
    }

    None
}
