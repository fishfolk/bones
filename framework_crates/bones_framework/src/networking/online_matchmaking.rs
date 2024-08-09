#![allow(missing_docs)]

use std::sync::Arc;
use bones_matchmaker_proto::{MatchInfo, MatchmakerRequest, MatchmakerResponse, MATCH_ALPN, GameID};
use iroh_net::{NodeId, NodeAddr};
use tracing::info;
use crate::{
    networking::{get_network_endpoint, socket::establish_peer_connections, NetworkMatchSocket},
    prelude::*,
    utils::BiChannelServer,
};
use super::online::{OnlineMatchmakerResponse, OnlineMatchmakerRequest};

pub async fn search_for_game(
    matchmaker_channel: &BiChannelServer<OnlineMatchmakerRequest, OnlineMatchmakerResponse>,
    id: NodeId,
    game_id: GameID,
    player_count: u32,
    match_data: Vec<u8>
) -> anyhow::Result<()> {
    info!("Connecting to online matchmaker");
    let ep = get_network_endpoint().await;
    let conn = ep.connect(id.into(), MATCH_ALPN).await?;
    info!("Connected to online matchmaker");

    matchmaker_channel
        .send(OnlineMatchmakerResponse::Searching)
        .await?;

    // Send a match request to the server
    let (mut send, mut recv) = conn.open_bi().await?;

    let message = MatchmakerRequest::RequestMatch(MatchInfo {
        client_count: player_count,
        match_data,
        game_id,
    });
    info!(request=?message, "Sending match request");

    let message = postcard::to_allocvec(&message)?;
    send.write_all(&message).await?;
    send.finish().await?;

    let response = recv.read_to_end(256).await?;
    let message: MatchmakerResponse = postcard::from_bytes(&response)?;

    if let MatchmakerResponse::Accepted = message {
        info!("Waiting for match...");
    } else {
        anyhow::bail!("Invalid response from matchmaker");
    }

    loop {
        tokio::select! {
            // UI message
            message = matchmaker_channel.recv() => {
                match message {
                    Ok(OnlineMatchmakerRequest::SearchForGame { .. }) => {
                        anyhow::bail!("Unexpected message from UI");
                    }
                    Ok(OnlineMatchmakerRequest::StopSearch) => {
                        info!("Canceling online search");
                        break;
                    }
                    Ok(OnlineMatchmakerRequest::ListLobbies { .. }) |
                    Ok(OnlineMatchmakerRequest::CreateLobby { .. }) |
                    Ok(OnlineMatchmakerRequest::JoinLobby { .. }) => {
                        anyhow::bail!("Unexpected lobby-related message during matchmaking");
                    }
                    Err(err) => {
                        anyhow::bail!("Failed to recv from match maker channel: {err:?}");
                    }
                }
            }
            // Matchmaker message
            recv = conn.accept_uni() => {
                let mut recv = recv?;
                let message = recv.read_to_end(5 * 1024).await?;
                let message: MatchmakerResponse = postcard::from_bytes(&message)?;

                match message {
                    MatchmakerResponse::ClientCount(count) => {
                        info!("Online match player count: {count}");
                        matchmaker_channel.try_send(OnlineMatchmakerResponse::PlayerCount(count as _))?;
                    }
                    MatchmakerResponse::Success {
                        random_seed,
                        player_idx,
                        client_count,
                        player_ids,
                    } => {
                        info!(%random_seed, %player_idx, player_count=%client_count, "Online match complete");

                        let peer_connections = establish_peer_connections(
                            player_idx,
                            client_count,
                            player_ids,
                            None,
                        )
                        .await?;

                        let socket = super::socket::Socket::new(player_idx, peer_connections);

                        matchmaker_channel.try_send(OnlineMatchmakerResponse::GameStarting {
                            socket: NetworkMatchSocket(Arc::new(socket)),
                            player_idx: player_idx as _,
                            player_count: client_count as _,
                        })?;
                        break;
                    }
                    other => anyhow::bail!("Unexpected message from matchmaker: {other:?}"),
                }
            }
        }
    }

    Ok(())
}

/// Search for a game via the matchmaker
pub fn start_search_for_matchmaked_game(matchmaking_server: NodeId, game_id: GameID, player_count: u32, match_data: Vec<u8>) {
    super::online::ONLINE_MATCHMAKER
        .try_send(OnlineMatchmakerRequest::SearchForGame {
            id: matchmaking_server,
            player_count,
            game_id,
            match_data
        })
        .unwrap()
}

/// Stop searching for game
pub fn stop_search_for_matchmaked_game() -> Result<(), async_channel::TrySendError<OnlineMatchmakerRequest>> {
    super::online::ONLINE_MATCHMAKER.try_send(OnlineMatchmakerRequest::StopSearch)
}