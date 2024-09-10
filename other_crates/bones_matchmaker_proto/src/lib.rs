#![doc = include_str!("../README.md")]
// This cfg_attr is needed because `rustdoc::all` includes lints not supported on stable
#![cfg_attr(doc, allow(unknown_lints))]
#![deny(rustdoc::all)]

use serde::{Deserialize, Serialize};

/// ALPN used for the matchmaking protocol.
pub const MATCH_ALPN: &[u8] = b"/bones/match/0";

/// ALPN used for the direct connections between players.
pub const PLAY_ALPN: &[u8] = b"/bones/play/0";

//
// === Matchmaking Mode ===
//
// These are messages sent when first establishing a connection to the matchmaker and waiting for a
// match.
//
/// Requests that may be made in matchmaking mode
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum MatchmakerRequest {
    /// Request a match from the server
    RequestMatch(MatchInfo),
    /// Request a list of lobbies for a specific game
    ListLobbies(String),
    /// Request to create a new lobby
    CreateLobby(LobbyInfo),
    /// Request to join an existing lobby for a specific gameid, optionally providing a password
    JoinLobby(GameID, LobbyId, Option<String>),
}

/// Information about a match that is being requested
#[derive(Serialize, Deserialize, Debug, Clone, Hash, Eq, PartialEq)]
pub struct MatchInfo {
    /// The number of clients to have in a match.
    pub player_count: u32,
    /// This is an arbitrary set of bytes that must match exactly for clients to end up in the same match.
    /// This allows us to support matchmaking for different modes or game versions with the same matchmaking server.
    pub match_data: Vec<u8>,
    /// The unique identifier for the game
    pub game_id: String,
    /// Enables choosing how player_idx should be assigned to each player who joins the match.
    pub player_idx_assignment: PlayerIdxAssignment,
}

/// Information about a lobby
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LobbyInfo {
    /// The name of the lobby
    pub name: String,
    /// The maximum number of players allowed in the lobby
    pub max_players: u32,
    /// The hashed password for the lobby, if any
    pub password_hash: Option<String>,
    /// This is an arbitrary set of bytes that the lobby creator specifies and which other clients must read/accept (ie. settings, version number, etc.)
    pub match_data: Vec<u8>,
    /// The unique identifier for the game
    pub game_id: String,
    /// Enables choosing how player_idx should be assigned to each player who joins the match.
    pub player_idx_assignment: PlayerIdxAssignment,
}

/// Choose how player_idx should be assigned to each player who joins a match/lobby.
#[derive(Serialize, Deserialize, Debug, Clone, Hash, Eq, PartialEq, Default)]
pub enum PlayerIdxAssignment {
    /// The players will be assigned an idx based on the order that they join the match/lobby.
    #[default]
    Ordered,
    /// The players will randomly be assigned an idx
    Random,
    /// The order specified in the Vec will be assigned to the players based on the order they join the match/lobby.
    /// Ie. If the Vec contains `0,2,1,3` then the first player will get player_idx 0, second 2, third 1, and fourth 3.
    SpecifiedOrder(Vec<usize>),
}

/// A unique identifier for a game
pub type GameID = String;

/// A unique identifier for a lobby
#[derive(Serialize, Deserialize, Debug, Clone, Hash, Eq, PartialEq)]
pub struct LobbyId(pub String);

/// Responses that may be returned in matchmaking mode
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum MatchmakerResponse {
    /// The connection has been accepted
    Accepted,
    /// Response that specifies updates about the current matchmaking (ie. player count updates)
    MatchmakingUpdate { player_count: u32 },
    /// The desired client count has been reached, and the match may start.
    Success {
        /// The random seed that each client should use.
        random_seed: u64,
        /// The client idx of the current client
        player_idx: u32,
        /// The number of connected clients in the match
        player_count: u32,
        /// The node ids of all players.
        player_ids: Vec<(u32, iroh_net::NodeAddr)>,
    },
    /// Response that specifies updates about the current lobby (ie. player count updates)
    LobbyUpdate { player_count: u32 },
    /// A list of available lobbies
    LobbiesList(Vec<LobbyListItem>),
    /// Confirmation that a lobby has been created
    LobbyCreated(LobbyId),
    /// Confirmation that a client has joined a lobby
    LobbyJoined(LobbyId),
    /// An error message
    Error(String),
}

/// Information about a lobby for the lobby list
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LobbyListItem {
    /// The unique identifier of the lobby
    pub id: LobbyId,
    /// The name of the lobby
    pub name: String,
    /// The current number of players in the lobby
    pub current_players: u32,
    /// The maximum number of players allowed in the lobby
    pub max_players: u32,
    /// Whether the lobby is password protected
    pub has_password: bool,
    /// The unique identifier for the game this lobby belongs to
    pub game_id: String,
}

/// The format of a message sent by a client to the proxy, so the proxy can send it to another client.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SendProxyMessage {
    /// The client that the message should go to.
    pub target_client: TargetClient,
    /// The message data.
    pub message: Vec<u8>,
}

/// The client to send a network message to.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum TargetClient {
    /// Send the message to all connected clients.
    All,
    /// Send the message to the client with the specified index.
    One(u8),
}

/// The format of a message forwarded by the proxy to a client.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RecvProxyMessage {
    /// The client that the message came from.
    pub from_client: u8,
    /// The message data.
    pub message: Vec<u8>,
}
