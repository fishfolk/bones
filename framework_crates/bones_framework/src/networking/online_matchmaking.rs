#![allow(missing_docs)]

use super::online::{
    MatchmakerConnectionState, OnlineMatchmakerRequest, OnlineMatchmakerResponse,
    READ_TO_END_BYTE_COUNT,
};
use crate::{
    networking::{socket::establish_peer_connections, NetworkMatchSocket},
    prelude::*,
    utils::BiChannelServer,
};
use bones_matchmaker_proto::{MatchInfo, MatchmakerRequest, MatchmakerResponse};
use std::sync::Arc;
use tracing::{info, warn};

/// Resolves the search for a match by sending a matchmaking request and handling responses.
pub(crate) async fn resolve_search_for_match(
    matchmaker_channel: &BiChannelServer<OnlineMatchmakerRequest, OnlineMatchmakerResponse>,
    matchmaker_connection_state: &mut MatchmakerConnectionState,
    match_info: MatchInfo,
) -> anyhow::Result<()> {
    let conn = matchmaker_connection_state.acquire_connection().await?;
    let (mut send, mut recv) = conn.open_bi().await?;

    // Send a matchmaking request to the server
    let message = MatchmakerRequest::RequestMatchmaking(match_info.clone());
    info!(request=?message, "Resolve: Sending match request");
    let message = postcard::to_allocvec(&message)?;
    send.write_all(&message).await?;
    send.finish()?;
    send.stopped().await?;

    let res = recv.read_to_end(READ_TO_END_BYTE_COUNT).await?;
    let _response: MatchmakerResponse = postcard::from_bytes(&res)?;

    loop {
        tokio::select! {
            message = matchmaker_channel.recv() => {
                match message {
                    Ok(OnlineMatchmakerRequest::StopSearch { .. }) => {
                        if let Err(err) = resolve_stop_search_for_match(
                            matchmaker_channel,
                            matchmaker_connection_state,
                            match_info.clone(),
                        ).await {
                            warn!("Error stopping matchmaking: {:?}", err);
                        }
                        return Ok(());
                    }
                    Ok(other) => {
                        warn!("Unexpected request during matchmaking: {:?}", other);
                    }
                    Err(err) => {
                        anyhow::bail!("Failed to recv from match maker channel: {err:?}");
                    }
                }
            }
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

                        matchmaker_connection_state.close_connection();
                        return Ok(());
                    }
                    other => anyhow::bail!("Unexpected message from matchmaker: {other:?}"),
                }
            }
        }
    }
}

/// Resolves the stop search for a match by sending a stop matchmaking request and handling responses.
pub(crate) async fn resolve_stop_search_for_match(
    matchmaker_channel: &BiChannelServer<OnlineMatchmakerRequest, OnlineMatchmakerResponse>,
    matchmaker_connection_state: &mut MatchmakerConnectionState,
    match_info: MatchInfo,
) -> anyhow::Result<()> {
    let conn = matchmaker_connection_state.acquire_connection().await?;
    // Use the existing connection to send the stop request
    let (mut send, mut recv) = conn.open_bi().await?;

    let message = MatchmakerRequest::StopMatchmaking(match_info);
    info!(request=?message, "Sending stop matchmaking request");

    let message = postcard::to_allocvec(&message)?;
    send.write_all(&message).await?;
    send.finish()?;
    send.stopped().await?;

    let res = recv.read_to_end(READ_TO_END_BYTE_COUNT).await?;
    let response: MatchmakerResponse = postcard::from_bytes(&res)?;

    match response {
        MatchmakerResponse::Accepted => {
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
