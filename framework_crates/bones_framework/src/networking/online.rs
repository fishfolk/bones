#![doc = include_str!("./online.md")]
// TODO
#![allow(missing_docs)]

pub use super::online_lobby::*;
pub use super::online_matchmaking::*;
use crate::{networking::{NetworkMatchSocket, get_network_endpoint }, prelude::*};
pub use bones_matchmaker_proto::{GameID, LobbyId, LobbyInfo, LobbyListItem, PlayerIdxAssignment, MatchInfo, MATCH_ALPN};
use iroh_net::NodeId;
use once_cell::sync::Lazy;
use tracing::{warn, info};
use iroh_quinn::Connection;
use iroh_net::Endpoint;

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

impl OnlineMatchmakerRequest {
    /// Returns the NodeId associated with the request.
    pub fn node_id(&self) -> NodeId {
        match self {
            OnlineMatchmakerRequest::SearchForGame { id, .. } => *id,
            OnlineMatchmakerRequest::StopSearch { id } => *id,
            OnlineMatchmakerRequest::ListLobbies { id, .. } => *id,
            OnlineMatchmakerRequest::CreateLobby { id, .. } => *id,
            OnlineMatchmakerRequest::JoinLobby { id, .. } => *id,
        }
    }
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
    let mut current_connection: Option<(Endpoint, Connection)> = None;
    let mut current_match_info: Option<MatchInfo> = None;

    while let Ok(message) = matchmaker_channel.recv().await {
        match message {
            OnlineMatchmakerRequest::SearchForGame {
                id,
                player_count,
                game_id,
                match_data,
                player_idx_assignment,
            } => {
                let (_ep, conn) = acquire_matchmaker_connection(id, &mut current_connection).await?;
                let match_info = MatchInfo {
                    max_players: player_count,
                    match_data,
                    game_id,
                    player_idx_assignment,
                };

                if let Err(err) = crate::networking::online_matchmaking::_resolve_search_for_match(
                    &matchmaker_channel,
                    conn.clone(),
                    match_info.clone(),
                )
                .await
                {
                    warn!("Online Game Search failed: {err:?}");
                    current_connection = None;
                    current_match_info = None;
                } else {
                    current_match_info = Some(match_info);
                }
            }
            OnlineMatchmakerRequest::StopSearch { id } => {
                let (_, conn) = acquire_matchmaker_connection(id, &mut current_connection).await?;
                if let Some(match_info) = current_match_info.take() {
                    if let Err(err) = crate::networking::online_matchmaking::_resolve_stop_search_for_match(
                        &matchmaker_channel,
                        conn.clone(),
                        match_info,
                    )
                    .await
                    {
                        warn!("Stopping search failed: {err:?}");
                    }
                } else {
                    matchmaker_channel
                        .send(OnlineMatchmakerResponse::Error("No active matchmaking to stop".to_string()))
                        .await?;
                }
            }
            OnlineMatchmakerRequest::ListLobbies { id, game_id } => {
                let (_, conn) = acquire_matchmaker_connection(id, &mut current_connection).await?;
                if let Err(err) = crate::networking::online_lobby::_resolve_list_lobbies(
                    &matchmaker_channel,
                    conn.clone(),
                    game_id,
                )
                .await
                {
                    warn!("Listing lobbies failed: {err:?}");
                }
            }
            OnlineMatchmakerRequest::CreateLobby { id, lobby_info } => {
                let (_, conn) = acquire_matchmaker_connection(id, &mut current_connection).await?;
                if let Err(err) = crate::networking::online_lobby::_resolve_create_lobby(
                    &matchmaker_channel,
                    conn.clone(),
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
                let (_, conn) = acquire_matchmaker_connection(id, &mut current_connection).await?;
                if let Err(err) = crate::networking::online_lobby::_resolve_join_lobby(
                    &matchmaker_channel,
                    conn.clone(),
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

/// Acquires the matchmaker connection, either establishing from scratch if none exists,
/// or fetching and returning the current connection.
pub async fn acquire_matchmaker_connection(
    id: NodeId,
    current_connection: &mut Option<(Endpoint, Connection)>,
) -> anyhow::Result<(&Endpoint, &Connection)> {
    if current_connection.is_none() {
        info!("Connecting to online matchmaker");
        let ep = get_network_endpoint().await;
        let conn = ep.connect(id.into(), MATCH_ALPN).await?;
        *current_connection = Some((ep.clone(), conn));
        info!("Connected to online matchmaker");
    }

    current_connection.as_ref()
        .map(|(ep, conn)| (ep, conn))
        .ok_or_else(|| anyhow::anyhow!("Failed to establish connection"))
}

impl OnlineMatchmaker {
    /// Read and return the latest matchmaker response, if one exists.
    pub fn read_matchmaker_response() -> Option<OnlineMatchmakerResponse> {
        ONLINE_MATCHMAKER.try_recv().ok()
    }
}
