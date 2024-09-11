use std::collections::HashMap;
use super::matchmaker::{MATCHMAKER_STATE, start_game,  GameLobbies};
use anyhow::Result;
use bones_matchmaker_proto::{
 MatchInfo,  MatchmakerResponse, GameID, LobbyListItem, LobbyInfo, LobbyId,
};
use iroh_net::Endpoint;
use quinn::Connection;
use crate::helpers::{generate_unique_id, hash_password};


/// Handles a request to list lobbies for a specific game
pub async fn handle_list_lobbies(game_id: GameID, send: &mut quinn::SendStream) -> Result<()> {
    let state = MATCHMAKER_STATE.lock().await;
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
pub async fn handle_create_lobby(
    conn: Connection,
    lobby_info: LobbyInfo,
    send: &mut quinn::SendStream,
) -> Result<()> {
    let lobby_id = LobbyId(generate_unique_id());
    let mut state = MATCHMAKER_STATE.lock().await;

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
pub async fn handle_join_lobby(
    ep: Endpoint,
    conn: Connection,
    game_id: GameID,
    lobby_id: LobbyId,
    password: Option<String>,
    send: &mut quinn::SendStream,
) -> Result<()> {
    let mut state = MATCHMAKER_STATE.lock().await;

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
