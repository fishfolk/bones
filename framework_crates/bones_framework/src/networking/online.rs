#![doc = include_str!("./online.md")]
// TODO
#![allow(missing_docs)]

use std::sync::Arc;

use bones_matchmaker_proto::{MatchInfo, MatchmakerRequest, MatchmakerResponse, MATCH_ALPN};
use iroh_net::NodeId;
use once_cell::sync::Lazy;
use tracing::{info, warn};

use crate::{
    networking::{get_network_endpoint, socket::establish_peer_connections, NetworkMatchSocket},
    prelude::*,
};

use super::socket::Socket;

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

    RUNTIME.spawn(online_matchmaker(server));

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
        socket: Socket,
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
                let conn = get_network_endpoint()
                    .await
                    .connect(id.into(), MATCH_ALPN)
                    .await
                    .unwrap();
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
                            let message = recv.read_to_end(5 * 1024).await.unwrap();
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
                                    player_ids,
                                } => {
                                    info!(%random_seed, %player_idx, player_count=%client_count, "Online match complete");

                                    let peer_connections = establish_peer_connections(
                                        player_idx as usize,
                                        client_count as usize,
                                        player_ids,
                                        None,
                                    )
                                    .await;

                                    let socket = Socket::new(player_idx as usize, peer_connections);

                                    matchmaker_channel
                                        .try_send(OnlineMatchmakerResponse::GameStarting {
                                            socket,
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
                socket,
                player_idx,
                player_count: _,
            } => {
                info!(?player_idx, "Starting network game");

                *search_state = default();

                return Some(NetworkMatchSocket(Arc::new(socket)));
            }
        }
    }

    None
}
