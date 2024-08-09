#![doc = include_str!("./online.md")]
// TODO
#![allow(missing_docs)]

pub use bones_matchmaker_proto::{LobbyId, LobbyInfo, LobbyListItem, GameID};
pub use super::online_matchmaking::*;
pub use super::online_lobby::*;
use iroh_net::NodeId;
use once_cell::sync::Lazy;
use tracing::warn;
use crate::{
    networking::NetworkMatchSocket,
    prelude::*,
};

/// Struct that holds a channel which exchange messages with the matchmaking server.
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
pub enum OnlineMatchmakerResponse {
    Searching,
    PlayerCount(usize),
    GameStarting {
        socket: NetworkMatchSocket,
        player_idx: usize,
        player_count: usize,
        random_seed: u64,
    },
    LobbiesList(Vec<LobbyListItem>),
    LobbyCreated(LobbyId),
    LobbyJoined {
        lobby_id: LobbyId,
        player_count: usize,
    },
    Error(String),
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

async fn online_matchmaker(
    matchmaker_channel: BiChannelServer<OnlineMatchmakerRequest, OnlineMatchmakerResponse>,
) -> anyhow::Result<()> {
    while let Ok(message) = matchmaker_channel.recv().await {
        match message {
            OnlineMatchmakerRequest::SearchForGame { id, player_count, game_id, match_data } => {
                if let Err(err) = crate::networking::online_matchmaking::_resolve_search_for_game(&matchmaker_channel, id, game_id, player_count, match_data).await {
                    warn!("Online Game Search failed: {err:?}");
                }
            }
            OnlineMatchmakerRequest::StopSearch => (), // Not searching, don't do anything
            OnlineMatchmakerRequest::ListLobbies { id, game_id } => {
                if let Err(err) = crate::networking::online_lobby::_resolve_list_lobbies(&matchmaker_channel, id, game_id).await {
                    warn!("Listing lobbies failed: {err:?}");
                }
            }
            OnlineMatchmakerRequest::CreateLobby { id, lobby_info } => {
                if let Err(err) = crate::networking::online_lobby::_resolve_create_lobby(&matchmaker_channel, id, lobby_info).await {
                    warn!("Creating lobby failed: {err:?}");
                }
            }
            OnlineMatchmakerRequest::JoinLobby { id, game_id, lobby_id, password } => {
                if let Err(err) = crate::networking::online_lobby::_resolve_join_lobby(&matchmaker_channel, id, game_id, lobby_id, password).await {
                    warn!("Joining lobby failed: {err:?}");
                }
            }
        }
    }

    Ok(())
}


impl OnlineMatchmaker {
    /// Read and return the latest matchmaker response, if one exists.
    pub fn read_matchmaker_response() -> Option<OnlineMatchmakerResponse> {
        ONLINE_MATCHMAKER.try_recv().ok()
    }
}

