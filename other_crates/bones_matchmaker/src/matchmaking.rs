use super::matchmaker::{start_game, MATCHMAKER_STATE};
use anyhow::Result;
use bones_matchmaker_proto::{MatchInfo, MatchmakerResponse};
use iroh_net::Endpoint;
use quinn::Connection;
use tokio::time::{sleep, Duration};

/// Handles a stop matchmaking request from a client
pub async fn handle_stop_matchmaking(
    conn: Connection,
    match_info: MatchInfo,
    send: &mut quinn::SendStream,
) -> Result<()> {
    let state = MATCHMAKER_STATE.lock().await;

    let removed = state
        .matchmaking_rooms
        .update(&match_info, |_, members| {
            if let Some(pos) = members
                .iter()
                .position(|member| member.stable_id() == conn.stable_id())
            {
                members.remove(pos);
                true
            } else {
                false
            }
        })
        .unwrap_or(false);

    let response = if removed {
        MatchmakerResponse::Accepted
    } else {
        MatchmakerResponse::Error("Not found in matchmaking queue".to_string())
    };

    let message = postcard::to_allocvec(&response)?;
    send.write_all(&message).await?;
    send.finish().await?;

    // If we removed a player, update the other players in the room
    if removed {
        drop(state); // Release the lock before calling send_matchmaking_updates
        if let Ok(active_connections) = send_matchmaking_updates(&match_info, 0).await {
            let player_count = active_connections.len();
            println!(
                "Updated matchmaking room. Current player count: {}",
                player_count
            );
        }
    }

    Ok(())
}

/// Handles a new matchmaking request from a client
pub async fn handle_request_matchaking(
    ep: Endpoint,
    conn: Connection,
    match_info: MatchInfo,
    send: &mut quinn::SendStream,
) -> Result<()> {
    let mut state = MATCHMAKER_STATE.lock().await;

    // Wait for up to 20 seconds if the matchmaking room is full
    for _ in 0..200 {
        let room_is_full = state
            .matchmaking_rooms
            .get(&match_info)
            .map(|room| room.get().len() >= match_info.max_players as usize)
            .unwrap_or(false);

        if !room_is_full {
            break;
        }

        // Temporarily release the lock while waiting
        drop(state);
        sleep(Duration::from_millis(100)).await;
        state = MATCHMAKER_STATE.lock().await;
    }

    // Final check if the room can be joined
    let can_join = state
        .matchmaking_rooms
        .get(&match_info)
        .map(|room| room.get().len() < match_info.max_players as usize)
        .unwrap_or(true);

    // Send error if room is still full
    // TODO: If this occurs often enough under heavy load, rework matchmakng to allow for multiple rooms
    if !can_join {
        let error_message = postcard::to_allocvec(&MatchmakerResponse::Error(
            "Matchmaking room is full. Please try matchmaking again shortly.".to_string(),
        ))?;
        send.write_all(&error_message).await?;
        send.finish().await?;
        return Ok(());
    }

    // Accept the matchmaking request
    let message = postcard::to_allocvec(&MatchmakerResponse::Accepted)?;
    send.write_all(&message).await?;
    send.finish().await?;

    // Add the connection to the matchmaking room
    let new_player_count = state
        .matchmaking_rooms
        .update(&match_info, |_, members| {
            members.push(conn.clone());
            members.len() as u32
        })
        .unwrap_or_else(|| {
            let members = vec![conn.clone()];
            if let Err(e) = state.matchmaking_rooms.insert(match_info.clone(), members) {
                warn!("Failed to insert new matchmaking room: {:?}", e);
            }
            1 as u32
        });

    // Release the lock after adding the new player
    drop(state);

    // Update all players and get active connections
    let active_connections = send_matchmaking_updates(&match_info, new_player_count).await?;

    let player_count = active_connections.len();

    // Start the game if room is full
    if player_count >= match_info.max_players as usize {
        start_matchmaked_game_if_ready(ep, &match_info).await?;
    }

    Ok(())
}

/// Sends matchmaking updates to all players in a room.
/// Actively checks if all connections are still alive before sending out new_player_count.
/// Returns the list of active connections.
async fn send_matchmaking_updates(
    match_info: &MatchInfo,
    new_player_count: u32,
) -> Result<Vec<Connection>> {
    let connections = {
        let state = MATCHMAKER_STATE.lock().await;
        state
            .matchmaking_rooms
            .get(match_info)
            .map(|room| room.get().clone())
            .unwrap_or_default()
    };

    let current_count = connections.len() as u32;
    let mut active_connections = Vec::new();

    // Prepare first update message
    let first_update_message = postcard::to_allocvec(&MatchmakerResponse::MatchmakingUpdate {
        player_count: current_count,
    })?;

    // Send first update and check active connections
    for (_index, conn) in connections.into_iter().enumerate() {
        if let Ok(mut send) = conn.open_uni().await {
            if send.write_all(&first_update_message).await.is_ok() && send.finish().await.is_ok() {
                active_connections.push(conn);
            }
        }
    }

    // Send second update if active connections count changed
    if active_connections.len() as u32 != new_player_count {
        let second_update_message =
            postcard::to_allocvec(&MatchmakerResponse::MatchmakingUpdate {
                player_count: active_connections.len() as u32,
            })?;

        for (index, member) in active_connections.iter().enumerate() {
            if let Ok(mut send) = member.open_uni().await {
                if let Err(e) = send.write_all(&second_update_message).await {
                    warn!("Connection to client {} has closed. {:?}", index, e);
                } else if let Err(e) = send.finish().await {
                    warn!("Connection to client {} has closed. {:?}", index, e);
                }
            }
        }
    }

    // Update stored connections
    {
        let state = MATCHMAKER_STATE.lock().await;
        if let None = state.matchmaking_rooms.remove(&match_info) {
            warn!("Failed to remove matchmaking room: {:?}", &match_info);
        }
        if let Err(e) = state
            .matchmaking_rooms
            .insert(match_info.clone(), active_connections.clone())
        {
            warn!(
                "Failed to insert updated matchmaking room: {:?}. Error: {:?}",
                &match_info, e
            );
        }
    }

    Ok(active_connections)
}

/// Starts a matchmade game if the room is ready with sufficient players
async fn start_matchmaked_game_if_ready(ep: Endpoint, match_info: &MatchInfo) -> Result<()> {
    let members = {
        let state = MATCHMAKER_STATE.lock().await;
        state
            .matchmaking_rooms
            .remove(match_info)
            .map(|(_, connections)| connections)
    };

    if let Some(members) = members {
        let cloned_match_info = match_info.clone();
        let players_len = members.len();
        tokio::spawn(async move {
            match start_game(ep, members, &cloned_match_info).await {
                Ok(_) => info!("Starting matchmaked game with {} players", players_len),
                Err(e) => error!("Error starting match: {:?}", e),
            }
        });
    } else {
        warn!("Failed to remove matchmaking room when starting game");
    }

    Ok(())
}
