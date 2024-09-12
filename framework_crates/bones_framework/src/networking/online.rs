#![doc = include_str!("./online.md")]
// TODO
#![allow(missing_docs)]

pub use super::online_lobby::*;
pub use super::online_matchmaking::*;
use crate::{networking::NetworkMatchSocket, prelude::*};
pub use bones_matchmaker_proto::{GameID, LobbyId, LobbyInfo, LobbyListItem, PlayerIdxAssignment, MatchInfo};
use iroh_net::NodeId;
use once_cell::sync::Lazy;
use tracing::warn;

/// Struct that holds a channel which exchange messages with the matchmaking server.
#[derive(DerefMut, Deref)]
pub struct OnlineMatchmaker(BiChannelClient<OnlineMatchmakerRequest, OnlineMatchmakerResponse>);

/// Online matchmaker request
#[derive(Debug)]
pub enum OnlineMatchmakerRequest {
    SearchForGame {
        id: NodeId,
        player_count: u32,
        game_id: GameID,
        match_data: Vec<u8>,
        player_idx_assignment: PlayerIdxAssignment,
    },
    StopSearch {
        id: NodeId,
    },
    ListLobbies {
        id: NodeId,
        game_id: GameID,
    },
    CreateLobby {
        id: NodeId,
        lobby_info: LobbyInfo,
    },
    JoinLobby {
        id: NodeId,
        game_id: GameID,
        lobby_id: LobbyId,
        password: Option<String>,
    },
}

/// Online matchmaker response
#[derive(Serialize, Clone)]
pub enum OnlineMatchmakerResponse {
    /// Searching for matchmaking in progress
    Searching,
    /// Response that specifies updates about the current matchmaking (ie. player count updates)
    MatchmakingUpdate { player_count: u32 },
    /// The desired client count has been reached, and the match may start.
    GameStarting {
        #[serde(skip_serializing, skip_deserializing)]
        socket: NetworkMatchSocket,
        player_idx: usize,
        player_count: usize,
        random_seed: u64,
    },
    /// Response that specifies updates about the current lobby (ie. player count updates)
    LobbyUpdate { player_count: u32 },
    /// A list of available lobbies
    LobbiesList(Vec<LobbyListItem>),
    /// Confirmation that a lobby has been created
    LobbyCreated(LobbyId),
    /// Confirmation that a client has joined a lobby
    LobbyJoined {
        lobby_id: LobbyId,
        player_count: usize,
    },
    /// An error message response
    Error(String),
}

impl std::fmt::Debug for OnlineMatchmakerResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let serialized =
            serde_yaml::to_string(self).expect("Failed to serialize OnlineMatchmakerResponse");
        write!(f, "{:?}", serialized)
    }
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

    let mut current_matchmaking_info = None;

    while let Ok(message) = matchmaker_channel.recv().await {
        match message {
            OnlineMatchmakerRequest::SearchForGame {
                id,
                player_count,
                game_id,
                match_data,
                player_idx_assignment,
            } => {
                let match_info = MatchInfo {
                    max_players: player_count,
                    match_data,
                    game_id,
                    player_idx_assignment,
                };
                current_matchmaking_info = Some(match_info.clone());

                if let Err(err) = crate::networking::online_matchmaking::_resolve_search_for_match(
                    &matchmaker_channel,
                    id,
                    match_info,
                )
                .await
                {
                    warn!("Online Game Search failed: {err:?}");
                }
            }
            OnlineMatchmakerRequest::StopSearch
            {
                id,
            }
             => {
                if let Some(match_info) = current_matchmaking_info.clone() {
                    current_matchmaking_info = None;
                    if let Err(err) = crate::networking::online_matchmaking::_resolve_stop_search_for_match(
                        &matchmaker_channel,
                        id,
                        match_info
                    )
                    .await
                    {
                        warn!("Stopping search failed: {err:?}");
                    }
                }
                else {
                    matchmaker_channel
                    .send(OnlineMatchmakerResponse::Error("Matchmaking must be started first before trying to stop.".to_string()))
                    .await?;
                }
            }
            OnlineMatchmakerRequest::ListLobbies { id, game_id } => {
                if let Err(err) = crate::networking::online_lobby::_resolve_list_lobbies(
                    &matchmaker_channel,
                    id,
                    game_id,
                )
                .await
                {
                    warn!("Listing lobbies failed: {err:?}");
                }
            }
            OnlineMatchmakerRequest::CreateLobby { id, lobby_info } => {
                if let Err(err) = crate::networking::online_lobby::_resolve_create_lobby(
                    &matchmaker_channel,
                    id,
                    lobby_info,
                )
                .await
                {
                    warn!("Creating lobby failed: {err:?}");
                }
            }
            OnlineMatchmakerRequest::JoinLobby {
                id,
                game_id,
                lobby_id,
                password,
            } => {
                if let Err(err) = crate::networking::online_lobby::_resolve_join_lobby(
                    &matchmaker_channel,
                    id,
                    game_id,
                    lobby_id,
                    password,
                )
                .await
                {
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
