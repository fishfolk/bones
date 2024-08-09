#![doc = include_str!("./online.md")]
// TODO
#![allow(missing_docs)]

use std::sync::Arc;
use bones_matchmaker_proto::{LobbyId, LobbyInfo, LobbyListItem, GameID, MatchInfo, MATCH_ALPN};
use iroh_net::NodeId;
use once_cell::sync::Lazy;
use tracing::{info, warn};
use crate::{
    networking::{get_network_endpoint, NetworkMatchSocket},
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

    RUNTIME.spawn(async move {
        if let Err(err) = online_matchmaker(server).await {
            warn!("online matchmaker failed: {err:?}");
        }
    });

    OnlineMatchmaker(client)
});

/// Channel to exchange messages with matchmaking server
#[derive(DerefMut, Deref)]
pub struct OnlineMatchmaker(BiChannelClient<OnlineMatchmakerRequest, OnlineMatchmakerResponse>);

/// Online matchmaker request
#[derive(Debug)]
pub enum OnlineMatchmakerRequest {
    SearchForGame { id: NodeId, player_count: u32, game_id: GameID, match_data: Vec<u8> },
    StopSearch,
    ListLobbies { id: NodeId, game_id: GameID },
    CreateLobby { id: NodeId, lobby_info: LobbyInfo },
    JoinLobby { id: NodeId, game_id: GameID, lobby_id: LobbyId, password: Option<String> },
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
    LobbiesList(Vec<LobbyListItem>),
    LobbyCreated(LobbyId),
    LobbyJoined {
        lobby_id: LobbyId,
        player_count: usize,
    },
    Error(String),
}

async fn online_matchmaker(
    matchmaker_channel: BiChannelServer<OnlineMatchmakerRequest, OnlineMatchmakerResponse>,
) -> anyhow::Result<()> {
    while let Ok(message) = matchmaker_channel.recv().await {
        match message {
            OnlineMatchmakerRequest::SearchForGame { id, player_count, game_id, match_data } => {
                if let Err(err) = crate::networking::online_matchmaking::search_for_game(&matchmaker_channel, id, game_id, player_count, match_data).await {
                    warn!("Online Game Search failed: {err:?}");
                }
            }
            OnlineMatchmakerRequest::StopSearch => (), // Not searching, don't do anything
            OnlineMatchmakerRequest::ListLobbies { id, game_id } => {
                if let Err(err) = crate::networking::online_lobby::list_lobbies(&matchmaker_channel, id, game_id).await {
                    warn!("Listing lobbies failed: {err:?}");
                }
            }
            OnlineMatchmakerRequest::CreateLobby { id, lobby_info } => {
                if let Err(err) = crate::networking::online_lobby::create_lobby(&matchmaker_channel, id, lobby_info).await {
                    warn!("Creating lobby failed: {err:?}");
                }
            }
            OnlineMatchmakerRequest::JoinLobby { id, game_id, lobby_id, password } => {
                if let Err(err) = crate::networking::online_lobby::join_lobby(&matchmaker_channel, id, game_id, lobby_id, password).await {
                    warn!("Joining lobby failed: {err:?}");
                }
            }
        }
    }

    Ok(())
}

/// Update state of game matchmaking or lobby, update `search_state`, return [`NetworkMatchSocket`] once connected.
pub fn update_online_state(search_state: &mut SearchState) -> Option<NetworkMatchSocket> {
    while let Ok(message) = ONLINE_MATCHMAKER.try_recv() {
        match message {
            OnlineMatchmakerResponse::Searching => *search_state = SearchState::Searching,
            OnlineMatchmakerResponse::PlayerCount(count) => {
                warn!("Waiting for players: {count}");
                *search_state = SearchState::WaitingForPlayers(count)
            }
            OnlineMatchmakerResponse::GameStarting { socket, player_idx, player_count: _ } => {
                info!(?player_idx, "Starting network game");
                *search_state = default();
                return Some(NetworkMatchSocket(Arc::new(socket)));
            }
            OnlineMatchmakerResponse::LobbiesList(lobbies) => {
                info!("Received lobbies list: {:?}", lobbies);
                // Handle the lobbies list (e.g., update UI)
            }
            OnlineMatchmakerResponse::LobbyCreated(lobby_id) => {
                info!("Lobby created: {:?}", lobby_id);
                // Handle lobby creation (e.g., update UI, join the created lobby)
            }
            OnlineMatchmakerResponse::LobbyJoined { lobby_id, player_count } => {
                info!("Joined lobby: {:?}, player count: {}", lobby_id, player_count);
                *search_state = SearchState::WaitingForPlayers(player_count);
            }
            OnlineMatchmakerResponse::Error(err) => {
                warn!("Online matchmaker error: {}", err);
                // Handle error (e.g., show error message to user)
            }
        }
    }

    None
}
