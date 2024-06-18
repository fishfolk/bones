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
// These are messages sent when first establishing a connecting to the matchmaker and waiting for a
// match.
//

/// Requests that may be made in matchmaking mode
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum MatchmakerRequest {
    /// Request a match ID from the server
    RequestMatch(MatchInfo),
}

/// Information about a match that is being requested
#[derive(Serialize, Deserialize, Debug, Clone, Hash, Eq, PartialEq)]
pub struct MatchInfo {
    /// The number of clients to have in a match.
    pub client_count: u32,
    /// This is an arbitrary set of bytes that must match exactly for clients to end up in the same
    /// match.
    ///
    /// This allows us to support matchmaking for different games/modes with the same matchmaking
    /// server.
    pub match_data: Vec<u8>,
}

/// Responses that may be returned in matchmaking mode
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum MatchmakerResponse {
    /// The conneciton has been accepted
    Accepted,
    /// This is the current number of connected clients
    ClientCount(u32),
    /// The desired client count has been reached, and you may start the match.
    Success {
        /// The random seed that each client should use.
        random_seed: u64,
        /// The client idx of the current client
        player_idx: u32,
        /// The number of connected clients in the match
        client_count: u32,
        /// The node ids of all players.
        player_ids: Vec<(u32, iroh_net::NodeAddr)>,
    },
}

//
// === Proxy mode ===
//
// These are messages sent after the match has been made and the clients are sending messages to
// each-other.
//

/// The format of a message sent by a client to the proxy, so the proxy can send it to another
/// client.
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
