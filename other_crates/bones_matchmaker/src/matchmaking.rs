use super::matchmaker::{MATCHMAKER_STATE, start_game};
use anyhow::Result;
use bones_matchmaker_proto::{
 MatchInfo,  MatchmakerResponse,
};
use tokio::time::{sleep, Duration};
use iroh_net::Endpoint;
use quinn::Connection;

pub async fn handle_request_match(
    ep: Endpoint,
    conn: Connection,
    match_info: MatchInfo,
    send: &mut quinn::SendStream,
) -> Result<()> {
    println!("Entering handle_request_match");

    // Check if the matchmaking room is full
    for _ in 0..100 { // 10 seconds total (100 * 100ms)
        let room_is_full = {
            let state = MATCHMAKER_STATE.lock().await;
            state.matchmaking_rooms.get(&match_info)
                .map(|room| room.get().len() >= match_info.max_players as usize)
                .unwrap_or(false)
        };

        if !room_is_full {
            break;
        }

        // Wait for 100ms before checking again
        sleep(Duration::from_millis(100)).await;
    }

    // Check one last time if the room is full
    let can_join = {
        let state = MATCHMAKER_STATE.lock().await;
        state.matchmaking_rooms.get(&match_info)
            .map(|room| room.get().len() < match_info.max_players as usize)
            .unwrap_or(true)
    };

    if !can_join {
        // Send an error message if the room is still full after waiting
        let error_message = postcard::to_allocvec(&MatchmakerResponse::Error(
            "Matchmaking room is full. Please try again later.".to_string(),
        ))?;
        send.write_all(&error_message).await?;
        send.finish().await?;
        return Ok(());
    }

    // Accept the matchmaking request
    let message = postcard::to_allocvec(&MatchmakerResponse::Accepted)?;
    send.write_all(&message).await?;
    send.finish().await?;
    println!("Sent Accepted response to client");
    println!("Handling join matchmaking. Match Info: {:?}", match_info);

    // Add the connection to the matchmaking room
    let new_player_count = {
        let state = MATCHMAKER_STATE.lock().await;
        println!("Acquired MATCHMAKER_STATE lock");

        let count = state.matchmaking_rooms.update(&match_info, |_, members| {
            members.push(conn.clone());
            members.len() as u32
        }).unwrap_or_else(|| {
            let members = vec![conn.clone()];
            state.matchmaking_rooms.insert(match_info.clone(), members).unwrap();
            1 as u32
        });

        println!("Added new connection to matchmaking room. Total members: {:?}", count);
        count
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
        let state = MATCHMAKER_STATE.lock().await;
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
        let state = MATCHMAKER_STATE.lock().await;
        state.matchmaking_rooms.remove(&match_info).unwrap();
        state.matchmaking_rooms.insert(match_info.clone(), active_connections.clone()).unwrap();
    }

    Ok(active_connections)
}

async fn start_matchmaked_game_if_ready(ep: Endpoint, match_info: &MatchInfo) -> Result<()> {
    let members = {
        let state = MATCHMAKER_STATE.lock().await;
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
