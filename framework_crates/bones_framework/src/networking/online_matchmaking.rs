#![allow(missing_docs)]

use super::online::{OnlineMatchmaker, OnlineMatchmakerRequest, OnlineMatchmakerResponse};
use crate::{
    networking::{socket::establish_peer_connections, NetworkMatchSocket},
    prelude::*,
    utils::BiChannelServer,
};
use bones_matchmaker_proto::{
    GameID, MatchInfo, MatchmakerRequest, MatchmakerResponse, PlayerIdxAssignment,
};
use iroh_net::NodeId;
use iroh_quinn::Connection;
use std::sync::Arc;
use tracing::info;

pub async fn _resolve_stop_search_for_match(
    matchmaker_channel: &BiChannelServer<OnlineMatchmakerRequest, OnlineMatchmakerResponse>,
    conn: Connection,
    match_info: MatchInfo,
) -> anyhow::Result<()> {
    println!("Resolve: Stopping matchmaking");

    // Use the existing connection to send the stop request
    let (mut send, mut recv) = conn.open_bi().await?;

    let message = MatchmakerRequest::StopMatchmaking(match_info);
    info!(request=?message, "Sending stop matchmaking request");
    println!("Sending stop matchmaking request");

    let message = postcard::to_allocvec(&message)?;
    send.write_all(&message).await?;
    send.finish().await?;

    let res = recv.read_to_end(256).await?;
    let response: MatchmakerResponse = postcard::from_bytes(&res)?;

    match response {
        MatchmakerResponse::Accepted => {
            println!("Stop matchmaking request accepted");
            matchmaker_channel
                .send(OnlineMatchmakerResponse::Error(
                    "Matchmaking stopped by user".to_string(),
                ))
                .await?;
        }
        MatchmakerResponse::Error(error) => {
            anyhow::bail!("Failed to stop matchmaking: {}", error);
        }
        _ => {
            anyhow::bail!("Unexpected response from matchmaker: {:?}", response);
        }
    }

    Ok(())
}

pub async fn _resolve_search_for_match(
    matchmaker_channel: &BiChannelServer<OnlineMatchmakerRequest, OnlineMatchmakerResponse>,
    conn: Connection,
    match_info: MatchInfo,
) -> anyhow::Result<()> {
    matchmaker_channel
        .send(OnlineMatchmakerResponse::Searching)
        .await?;

    // Send a match request to the server
    let (mut send, mut recv) = conn.open_bi().await?;

    let message = MatchmakerRequest::RequestMatchmaking(match_info.clone());
    info!(request=?message, "Resolve: Sending match request");

    let message = postcard::to_allocvec(&message)?;
    send.write_all(&message).await?;
    send.finish().await?;

    let res = recv.read_to_end(256).await?;
    let _response: MatchmakerResponse = postcard::from_bytes(&res)?;

    loop {
        tokio::select! {
            // UI message
            // message = matchmaker_channel.recv() => {
            //     match message {
            //         Ok(OnlineMatchmakerRequest::SearchForGame { .. }) => {
            //             anyhow::bail!("Unexpected message from UI");
            //         }
            //         Ok(OnlineMatchmakerRequest::StopSearch { .. }) => {
            //             anyhow::bail!("Unexpected stop search");
            //         }
            //         Ok(OnlineMatchmakerRequest::ListLobbies { .. }) |
            //         Ok(OnlineMatchmakerRequest::CreateLobby { .. }) |
            //         Ok(OnlineMatchmakerRequest::JoinLobby { .. }) => {
            //             anyhow::bail!("Unexpected lobby-related message during matchmaking");
            //         }
            //         Err(err) => {
            //             anyhow::bail!("Failed to recv from match maker channel: {err:?}");
            //         }
            //     }
            // }
            // Matchmaker message
            recv = conn.accept_uni() => {
                let mut recv = recv?;
                let message = recv.read_to_end(5 * 1024).await?;
                let message: MatchmakerResponse = postcard::from_bytes(&message)?;

                match message {
                    MatchmakerResponse::MatchmakingUpdate{ player_count } => {
                        info!("Online matchmaking updated player count: {player_count}");
                        matchmaker_channel.try_send(OnlineMatchmakerResponse::MatchmakingUpdate{ player_count })?;
                    }
                    MatchmakerResponse::Success {
                        random_seed,
                        player_idx,
                        player_count,
                        player_ids,
                    } => {
                        info!(%random_seed, %player_idx, player_count=%player_count, "Online match starting");

                        let peer_connections = establish_peer_connections(
                            player_idx,
                            player_count,
                            player_ids,
                            None,
                        )
                        .await?;

                        let socket = super::socket::Socket::new(player_idx, peer_connections);

                        matchmaker_channel.try_send(OnlineMatchmakerResponse::GameStarting {
                            socket: NetworkMatchSocket(Arc::new(socket)),
                            player_idx: player_idx as _,
                            player_count: player_count as _,
                            random_seed
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

impl OnlineMatchmaker {
    /// Sends a request to the matchmaking server to start searching for a match. Response is read via `read_matchmaker_response()`.
    pub fn start_search_for_match(
        matchmaking_server: NodeId,
        game_id: GameID,
        player_count: u32,
        match_data: Vec<u8>,
        player_idx_assignment: PlayerIdxAssignment,
    ) -> Result<(), async_channel::TrySendError<OnlineMatchmakerRequest>> {
        super::online::ONLINE_MATCHMAKER.try_send(OnlineMatchmakerRequest::SearchForGame {
            id: matchmaking_server,
            player_count,
            game_id,
            match_data,
            player_idx_assignment,
        })
    }

    /// Stops searching for a match.
    pub fn stop_search_for_match(
        matchmaking_server: NodeId,
    ) -> Result<(), async_channel::TrySendError<OnlineMatchmakerRequest>> {
        super::online::ONLINE_MATCHMAKER.try_send(OnlineMatchmakerRequest::StopSearch {
            id: matchmaking_server,
        })
    }
}
