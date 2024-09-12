#![doc = include_str!("./online.md")]
// TODO
#![allow(missing_docs)]

pub use super::online_lobby::*;
pub use super::online_matchmaking::*;
use crate::{
    networking::{get_network_endpoint, NetworkMatchSocket},
    prelude::*,
};
pub use bones_matchmaker_proto::{
    GameID, LobbyId, LobbyInfo, LobbyListItem, MatchInfo, PlayerIdxAssignment, MATCH_ALPN,
};
use iroh_net::Endpoint;
use iroh_net::NodeId;
use iroh_quinn::Connection;
use once_cell::sync::Lazy;
use tracing::{info, warn};

/// The number of bytes to use for read_to_end()
pub const READ_TO_END_BYTE_COUNT: usize = 256;

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
        if let Err(err) = process_matchmaker_requests(server).await {
            warn!("online matchmaker failed: {err:?}");
        }
    });

    OnlineMatchmaker(client)
});

/// Internal struct used to keep track of the connection with the matchmaker
pub struct MatchmakerConnectionState {
    ep: Option<Endpoint>,
    conn: Option<Connection>,
    node_id: Option<NodeId>,
}

impl MatchmakerConnectionState {
    /// Initialize a new MatchmakerConnectionState
    pub fn new() -> Self {
        Self {
            ep: None,
            conn: None,
            node_id: None,
        }
    }

    /// Acquires the matchmaker connection, either establishing from scratch if none exists
    /// or fetching the currently held connection.
    pub async fn acquire_connection(&mut self) -> anyhow::Result<&Connection> {
        if let Some(id) = self.node_id {
            if self.conn.is_none() {
                info!("Connecting to online matchmaker");
                println!("Connecting to online matchmaker");
                let ep = get_network_endpoint().await;
                let conn = ep.connect(id.into(), MATCH_ALPN).await?;
                self.ep = Some(ep.clone());
                self.conn = Some(conn);
                info!("Connected to online matchmaker");
                println!("Connected to online matchmaker");
            }

            println!("Acquired online matchmaker connection");
            self.conn.as_ref().ok_or_else(|| anyhow::anyhow!("Failed to establish connection"))
        } else {
            Err(anyhow::anyhow!("NodeId not set"))
        }
    }

    /// Closes the connection with the matchmaker, and removes the conn/ep from self.
    pub fn close_connection(&mut self) {
        if let Some(conn) = self.conn.take() {
            conn.close(0u32.into(), b"Closing matchmaker connection");
            println!("Closed matchmaker connection");
        }
        self.ep = None;
    }

    /// Returns true if a connection with the matchmaker currently exists
    pub fn is_connected(&self) -> bool {
        self.conn.is_some()
    }

    /// Sets the iroh NodeId that will be used to establish connection with the matchmaker
    pub fn set_node_id(&mut self, id: NodeId) {
        self.node_id = Some(id);
    }
}

/// Core communication processing for the matchmaker
async fn process_matchmaker_requests(
    user_channel: BiChannelServer<OnlineMatchmakerRequest, OnlineMatchmakerResponse>,
) -> anyhow::Result<()> {
    let mut matchmaker_connection_state = MatchmakerConnectionState::new();

    while let Ok(message) = user_channel.recv().await {
        match message {
            OnlineMatchmakerRequest::SearchForGame {
                id,
                player_count,
                game_id,
                match_data,
                player_idx_assignment,
            } => {
                matchmaker_connection_state.set_node_id(id);
                let match_info = MatchInfo {
                    max_players: player_count,
                    match_data,
                    game_id,
                    player_idx_assignment,
                };

                if let Err(err) = crate::networking::online_matchmaking::_resolve_search_for_match(
                    &user_channel,
                    &mut matchmaker_connection_state,
                    match_info.clone(),
                )
                .await
                {
                    warn!("Online Matchmaking Search failed: {err:?}");
                }
                matchmaker_connection_state.close_connection();
            }
            OnlineMatchmakerRequest::ListLobbies { id, game_id } => {
                matchmaker_connection_state.set_node_id(id);
                if let Err(err) = crate::networking::online_lobby::_resolve_list_lobbies(
                    &user_channel,
                    &mut matchmaker_connection_state,
                    game_id,
                )
                .await
                {
                    warn!("Listing lobbies failed: {err:?}");
                }
            }
            OnlineMatchmakerRequest::CreateLobby { id, lobby_info } => {
                matchmaker_connection_state.set_node_id(id);
                if let Err(err) = crate::networking::online_lobby::_resolve_create_lobby(
                    &user_channel,
                    &mut matchmaker_connection_state,
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
                matchmaker_connection_state.set_node_id(id);
                if let Err(err) = crate::networking::online_lobby::_resolve_join_lobby(
                    &user_channel,
                    &mut matchmaker_connection_state,
                    game_id,
                    lobby_id,
                    password,
                )
                .await
                {
                    warn!("Joining lobby failed: {err:?}");
                }
            }
            OnlineMatchmakerRequest::StopSearch { id } => {
                matchmaker_connection_state.set_node_id(id);
                matchmaker_connection_state.close_connection();
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
