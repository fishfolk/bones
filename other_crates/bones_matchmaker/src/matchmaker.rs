use crate::helpers::{generate_random_seed, generate_unique_id, hash_password};
use anyhow::Result;
use bones_matchmaker_proto::{
    GameID, LobbyId, LobbyInfo, LobbyListItem, MatchInfo, MatchmakerRequest, MatchmakerResponse,
    PlayerIdxAssignment,
};
use tokio::time::{timeout, Duration};
use iroh_net::{Endpoint, NodeAddr};
use once_cell::sync::Lazy;
use quinn::Connection;
use rand::{prelude::SliceRandom, SeedableRng};
use scc::HashMap as SccHashMap;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Represents the lobbies for a specific game
struct GameLobbies {
    #[allow(dead_code)]
    game_id: GameID,
    lobbies: HashMap<LobbyId, LobbyInfo>,
}

/// Represents the global state of the matchmaker
#[derive(Default)]
struct State {
    game_lobbies: HashMap<GameID, GameLobbies>,
    lobby_connections: SccHashMap<(GameID, LobbyId), Vec<Connection>>,
    matchmaking_rooms: SccHashMap<MatchInfo, Vec<Connection>>,
}

/// Global state of the matchmaker
static STATE: Lazy<Arc<Mutex<State>>> = Lazy::new(|| Arc::new(Mutex::new(State::default())));

/// Handles incoming connections and routes requests to appropriate handlers
pub async fn handle_connection(ep: Endpoint, conn: Connection) -> Result<()> {
    let connection_id = conn.stable_id();
    println!( "Accepted matchmaker connection: {:?}", connection_id);

    loop {
        tokio::select! {
            close = conn.closed() => {
                println!("Connection closed {close:?}");
                return Ok(());
            }
            bi = conn.accept_bi() => {
                let (mut send, mut recv) = bi?;
                // Parse the incoming request
                let request: MatchmakerRequest = postcard::from_bytes(&recv.read_to_end(256).await?)?;

                // Route the request to the appropriate handler
                match request {
                    MatchmakerRequest::RequestMatch(match_info) => {
                        handle_request_match(ep.clone(), conn.clone(), match_info, &mut send).await?;
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

/// Handles a request to list lobbies for a specific game
async fn handle_list_lobbies(game_id: GameID, send: &mut quinn::SendStream) -> Result<()> {
    let state = STATE.lock().await;
    // Retrieve and format lobby information for the specified game
    let lobbies = state
        .game_lobbies
        .get(&game_id)
        .map(|game_lobbies| {
            game_lobbies
                .lobbies
                .iter()
                .map(|(id, lobby_info)| {
                    let current_players = state
                        .lobby_connections
                        .get(&(game_id.clone(), id.clone()))
                        .map(|entry| entry.get().len() as u32)
                        .unwrap_or(0);
                    LobbyListItem {
                        id: id.clone(),
                        name: lobby_info.name.clone(),
                        current_players,
                        max_players: lobby_info.max_players,
                        has_password: lobby_info.password_hash.is_some(),
                        game_id: game_id.clone(),
                    }
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    // Send the lobby list back to the client
    let message = postcard::to_allocvec(&MatchmakerResponse::LobbiesList(lobbies))?;
    send.write_all(&message).await?;
    send.finish().await?;

    Ok(())
}

/// Handles a request to create a new lobby
async fn handle_create_lobby(
    conn: Connection,
    lobby_info: LobbyInfo,
    send: &mut quinn::SendStream,
) -> Result<()> {
    let lobby_id = LobbyId(generate_unique_id());
    let mut state = STATE.lock().await;

    // Create or update the game lobbies and insert the new lobby
    state
        .game_lobbies
        .entry(lobby_info.game_id.clone())
        .or_insert_with(|| GameLobbies {
            game_id: lobby_info.game_id.clone(),
            lobbies: HashMap::new(),
        })
        .lobbies
        .insert(lobby_id.clone(), lobby_info.clone());

    // Add the connection to the lobby
        if let Err(e) = state
        .lobby_connections
        .insert((lobby_info.game_id.clone(), lobby_id.clone()), vec![conn]) {
            error!("Failed to inserting lobby during creation: {:?}", e);
        }

    // Send confirmation to the client
    let message = postcard::to_allocvec(&MatchmakerResponse::LobbyCreated(lobby_id))?;
    send.write_all(&message).await?;
    send.finish().await?;

    Ok(())
}

/// Handles a request to join an existing lobby
async fn handle_join_lobby(
    ep: Endpoint,
    conn: Connection,
    game_id: GameID,
    lobby_id: LobbyId,
    password: Option<String>,
    send: &mut quinn::SendStream,
) -> Result<()> {
    let mut state = STATE.lock().await;

    if let Some(game_lobbies) = state.game_lobbies.get_mut(&game_id) {
        if let Some(lobby_info) = game_lobbies.lobbies.get(&lobby_id) {
            // Check password if the lobby is password-protected
            if let Some(hash) = &lobby_info.password_hash {
                if password.as_ref().map(|p| hash_password(p)) != Some(hash.clone()) {
                    let message = postcard::to_allocvec(&MatchmakerResponse::Error(
                        "Incorrect password".to_string(),
                    ))?;
                    send.write_all(&message).await?;
                    send.finish().await?;
                    return Ok(());
                }
            }

            let max_players = lobby_info.max_players;
            let match_data = lobby_info.match_data.clone();
            let player_idx_assignment = lobby_info.player_idx_assignment.clone();

            // Try to add the player to the lobby
            let join_result = state.lobby_connections.update(
                &(game_id.clone(), lobby_id.clone()),
                |_exists, connections| {
                    if connections.len() < max_players as usize {
                        connections.push(conn.clone());
                        Some(connections.len())
                    } else {
                        None
                    }
                },
            );

            match join_result {
                Some(Some(count)) => {
                    // Successfully joined the lobby
                    let message =
                        postcard::to_allocvec(&MatchmakerResponse::LobbyJoined(lobby_id.clone()))?;
                    send.write_all(&message).await?;
                    send.finish().await?;

                    // Always notify all players in the lobby about the update
                    let lobby_update_message =
                        postcard::to_allocvec(&MatchmakerResponse::LobbyUpdate{player_count: count as u32})?;
                    if let Some(connections) = state
                        .lobby_connections
                        .get(&(game_id.clone(), lobby_id.clone()))
                    {
                        for connection in connections.get().iter() {
                            let mut send = connection.open_uni().await?;
                            send.write_all(&lobby_update_message).await?;
                            send.finish().await?;
                        }
                    }

                    // Check if the lobby is full and start the match if it is
                    if count == max_players as usize {
                        let match_info = MatchInfo {
                            max_players,
                            match_data,
                            game_id: game_id.clone(),
                            player_idx_assignment,
                        };
                        if let Some(connections) = state
                            .lobby_connections
                            .remove(&(game_id.clone(), lobby_id.clone()))
                        {
                            let members = connections.1;
                            drop(state);
                            tokio::spawn(async move {
                                if let Err(e) = start_game(ep, members, &match_info).await {
                                    error!("Error starting match from full lobby: {:?}", e);
                                }
                            });
                        }
                    }
                }
                _ => {
                    // Lobby is full
                    let message = postcard::to_allocvec(&MatchmakerResponse::Error(
                        "Lobby is full".to_string(),
                    ))?;
                    send.write_all(&message).await?;
                    send.finish().await?;
                }
            }
        } else {
            let message =
                postcard::to_allocvec(&MatchmakerResponse::Error("Lobby not found".to_string()))?;
            send.write_all(&message).await?;
            send.finish().await?;
        }
    } else {
        let message =
            postcard::to_allocvec(&MatchmakerResponse::Error("Game not found".to_string()))?;
        send.write_all(&message).await?;
        send.finish().await?;
    }

    Ok(())
}

/// Handles a request to join a matchmaking queue
async fn handle_request_match(
    ep: Endpoint,
    conn: Connection,
    match_info: MatchInfo,
    send: &mut quinn::SendStream,
) -> Result<()> {
    println!("Entering handle_request_match");

    // Accept the matchmaking request
    let message = postcard::to_allocvec(&MatchmakerResponse::Accepted)?;
    send.write_all(&message).await?;
    send.finish().await?;
    println!("Sent Accepted response to client");
    println!("Handling join matchmaking. Match Info: {:?}", match_info);

    // Add the connection to the matchmaking room
    let new_player_count = {
        let mut state = STATE.lock().await;
        println!("Acquired STATE lock");

        let mut room_entry = state.matchmaking_rooms.entry(match_info.clone()).or_default();
        let members = room_entry.get_mut();
        members.push(conn.clone());
        println!("Added new connection to matchmaking room. Total members: {}", members.len());
        members.len() as u32
    }; // Release the lock here

    // Send MatchmakingUpdate to all players in the room and get active connections
    let active_connections = send_matchmaking_updates(&match_info, new_player_count).await?;

    let player_count = active_connections.len();
    println!("Room now has {}/{} active players", player_count, match_info.max_players);

    // Check if the room is full and start the game if it is
    if player_count >= match_info.max_players as usize {
        println!("Room is full. Starting the game.");
        start_matchmaked_game_if_ready(ep, &match_info).await?;
    }

    println!("Exiting handle_request_match");
    Ok(())
}


async fn send_matchmaking_updates(match_info: &MatchInfo, new_player_count: u32) -> Result<Vec<Connection>> {
    let connections = {
        let state = STATE.lock().await;
        state.matchmaking_rooms.get(match_info)
            .map(|room| room.get().clone())
            .unwrap_or_default()
    };

    let current_count = connections.len() as u32;
    let mut active_connections = Vec::new();

    let first_update_message = postcard::to_allocvec(&MatchmakerResponse::MatchmakingUpdate {
        player_count: current_count
    })?;

    // Send first update and check which connections are still active
    for (index, conn) in connections.into_iter().enumerate() {
        if let Ok(mut send) = conn.open_uni().await {
            if send.write_all(&first_update_message).await.is_ok() && send.finish().await.is_ok() {
                println!("Successfully sent first update to member {}", index);
                active_connections.push(conn);
            } else {
                println!("Failed to send first update to member {}", index);
            }
        } else {
            println!("Failed to open uni stream for member {}", index);
        }
    }

    // If the number of active connections is different from what we expected, send a second update
    if active_connections.len() as u32 != new_player_count {
        let second_update_message = postcard::to_allocvec(&MatchmakerResponse::MatchmakingUpdate {
            player_count: active_connections.len() as u32
        })?;

        for (index, member) in active_connections.iter().enumerate() {
            if let Ok(mut send) = member.open_uni().await {
                if let Err(e) = send.write_all(&second_update_message).await {
                    println!("Failed to send second update to member {}: {:?}", index, e);
                } else if let Err(e) = send.finish().await {
                    println!("Failed to finish sending second update to member {}: {:?}", index, e);
                } else {
                    println!("Successfully sent second update to member {}", index);
                }
            } else {
                println!("Failed to open uni stream for second update to member {}", index);
            }
        }
    }

    // Update the stored connections
    {
        let state = STATE.lock().await;
        state.matchmaking_rooms.remove(&match_info).unwrap();
        state.matchmaking_rooms.insert(match_info.clone(), active_connections.clone()).unwrap();
    }

    Ok(active_connections)
}

async fn start_matchmaked_game_if_ready(ep: Endpoint, match_info: &MatchInfo) -> Result<()> {
    let members = {
        let state = STATE.lock().await;
        state.matchmaking_rooms.remove(match_info).map(|(_, connections)| connections)
    };

    if let Some(members) = members {
        let cloned_match_info = match_info.clone();
        println!("Starting game with {} members", members.len());
        tokio::spawn(async move {
            match start_game(ep, members, &cloned_match_info).await {
                Ok(_) => println!("Game started successfully"),
                Err(e) => error!("Error starting match: {:?}", e),
            }
        });
    } else {
        warn!("Failed to remove matchmaking room when starting game");
    }

    Ok(())
}


/// Starts a match/lobby with the given members
async fn start_game(ep: Endpoint, members: Vec<Connection>, match_info: &MatchInfo) -> Result<()> {
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
            if indices.len() < player_count {
                indices.extend(indices.len()..player_count);
            } else if indices.len() > player_count {
                indices.truncate(player_count);
            }
            indices
        }
    };

    // Collect player IDs and addresses
    for (conn_idx, conn) in members.iter().enumerate() {
        let id = iroh_net::endpoint::get_remote_node_id(&conn)?;
        let mut addr = NodeAddr::new(id);
        if let Some(info) = ep.connection_info(id) {
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
        send.finish().await?;
        conn.close(0u32.into(), b"done");
    }

    Ok(())
}
