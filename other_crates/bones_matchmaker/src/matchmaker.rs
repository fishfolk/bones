use super::lobbies::{handle_create_lobby, handle_join_lobby, handle_list_lobbies};
use super::matchmaking::{handle_request_matchaking, handle_stop_matchmaking};
use crate::helpers::generate_random_seed;
use anyhow::Result;
use bones_matchmaker_proto::{
    GameID, LobbyId, LobbyInfo, MatchInfo, MatchmakerRequest, MatchmakerResponse,
    PlayerIdxAssignment,
};
use iroh::{endpoint::Connection, Endpoint, NodeAddr};
use once_cell::sync::Lazy;
use rand::{prelude::SliceRandom, SeedableRng};
use scc::HashMap as SccHashMap;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Represents the lobbies for a specific game
pub struct GameLobbies {
    #[allow(dead_code)]
    pub game_id: GameID,
    pub lobbies: HashMap<LobbyId, LobbyInfo>,
}

/// Represents the global state of the matchmaker
#[derive(Default)]
pub struct State {
    pub game_lobbies: HashMap<GameID, GameLobbies>,
    pub lobby_connections: SccHashMap<(GameID, LobbyId), Vec<Connection>>,
    pub matchmaking_rooms: SccHashMap<MatchInfo, Vec<Connection>>,
}

/// Global state of the matchmaker
pub static MATCHMAKER_STATE: Lazy<Arc<Mutex<State>>> =
    Lazy::new(|| Arc::new(Mutex::new(State::default())));

/// Handles incoming connections and routes requests to appropriate handlers
pub async fn handle_connection(ep: Endpoint, conn: Connection) -> Result<()> {
    let connection_id = conn.stable_id();
    loop {
        tokio::select! {
            _ = conn.closed() => {
                info!("[{}] Client closed connection.", connection_id);
                return Ok(());
            }
            bi = conn.accept_bi() => {
                let (mut send, mut recv) = bi?;
                // Parse the incoming request
                let request: MatchmakerRequest = postcard::from_bytes(&recv.read_to_end(256).await?)?;

                // Route the request to the appropriate handler
                match request {
                    MatchmakerRequest::RequestMatchmaking(match_info) => {
                        handle_request_matchaking(ep.clone(), conn.clone(), match_info, &mut send).await?;
                        send.finish()?;
                        send.stopped().await?;
                    }
                    MatchmakerRequest::StopMatchmaking(match_info) => {
                        handle_stop_matchmaking(conn.clone(), match_info, &mut send).await?;
                    }
                    MatchmakerRequest::ListLobbies(game_id) => {
                        handle_list_lobbies(game_id, &mut send).await?;
                    }
                    MatchmakerRequest::CreateLobby(lobby_info) => {
                        handle_create_lobby(conn.clone(), lobby_info, &mut send).await?;
                    }
                    MatchmakerRequest::JoinLobby(game_id, lobby_id, password) => {
                        handle_join_lobby(ep.clone(), conn.clone(), game_id, lobby_id, password, &mut send).await?;
                    }
                }
            }
        }
    }
}

/// Starts a match/lobby with the given members
pub async fn start_game(
    ep: Endpoint,
    members: Vec<Connection>,
    match_info: &MatchInfo,
) -> Result<()> {
    let random_seed = generate_random_seed();
    let mut player_ids = Vec::new();
    let player_count = members.len();

    // Generate player indices based on the PlayerIdxAssignment
    let player_indices = match &match_info.player_idx_assignment {
        PlayerIdxAssignment::Ordered => (0..player_count).collect::<Vec<_>>(),
        PlayerIdxAssignment::Random => {
            let mut indices: Vec<_> = (0..player_count).collect();
            let mut rng = rand::rngs::StdRng::seed_from_u64(random_seed);
            indices.shuffle(&mut rng);
            indices
        }
        PlayerIdxAssignment::SpecifiedOrder(order) => {
            let mut indices = order.clone();
            match indices.len().cmp(&player_count) {
                Ordering::Less => {
                    indices.extend(indices.len()..player_count);
                }
                Ordering::Greater => {
                    indices.truncate(player_count);
                }
                _ => (),
            }
            indices
        }
    };

    // Collect player IDs and addresses
    for (conn_idx, conn) in members.iter().enumerate() {
        let id = iroh::endpoint::get_remote_node_id(conn)?;
        let mut addr = NodeAddr::new(id);
        if let Some(info) = ep.remote_info(id) {
            if let Some(relay_url) = info.relay_url {
                addr = addr.with_relay_url(relay_url.relay_url);
            }
            addr = addr.with_direct_addresses(info.addrs.into_iter().map(|addr| addr.addr));
        }
        let player_idx = player_indices[conn_idx];
        player_ids.push((player_idx as u32, addr));
    }

    // Sort player_ids by the assigned player index
    player_ids.sort_by_key(|&(idx, _)| idx);

    // Send match information to each player
    for (conn_idx, conn) in members.into_iter().enumerate() {
        let player_idx = player_indices[conn_idx];
        let message = postcard::to_allocvec(&MatchmakerResponse::Success {
            random_seed,
            player_count: player_ids.len() as u32,
            player_idx: player_idx as u32,
            player_ids: player_ids.clone(),
        })?;
        let mut send = conn.open_uni().await?;
        send.write_all(&message).await?;
        send.finish()?;
        send.stopped().await?;
        conn.close(0u32.into(), b"done");
    }

    Ok(())
}
