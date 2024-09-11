#![allow(missing_docs)]

use std::sync::Arc;
use bones_matchmaker_proto::{MatchInfo, MatchmakerRequest, MatchmakerResponse, MATCH_ALPN, GameID, PlayerIdxAssignment};
use iroh_net::NodeId;
use tracing::info;
use crate::{
    networking::{get_network_endpoint, socket::establish_peer_connections, NetworkMatchSocket},
    prelude::*,
    utils::BiChannelServer,
};
use super::online::{OnlineMatchmakerResponse, OnlineMatchmakerRequest, OnlineMatchmaker};

pub async fn _resolve_search_for_match(
    matchmaker_channel: &BiChannelServer<OnlineMatchmakerRequest, OnlineMatchmakerResponse>,
    id: NodeId,
    game_id: GameID,
    player_count: u32,
    match_data: Vec<u8>,
    player_idx_assignment: PlayerIdxAssignment
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
        max_players: player_count,
        match_data,
        game_id,
player_idx_assignment
    });
    info!(request=?message, "Sending match request");

    let message = postcard::to_allocvec(&message)?;
    send.write_all(&message).await?;
    send.finish().await?;

    let res = recv.read_to_end(256).await?;
    let response: MatchmakerResponse = postcard::from_bytes(&res)?;

    if let MatchmakerResponse::Accepted = response {
        info!("Matchmaking request accepted. Waiting for match...");
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
    pub fn start_search_for_match(matchmaking_server: NodeId, game_id: GameID, player_count: u32, match_data: Vec<u8>, player_idx_assignment: PlayerIdxAssignment) -> Result<(), async_channel::TrySendError<OnlineMatchmakerRequest>> {
        super::online::ONLINE_MATCHMAKER
            .try_send(OnlineMatchmakerRequest::SearchForGame {
                id: matchmaking_server,
                player_count,
                game_id,
                match_data,
                player_idx_assignment,
            })
    }

    /// Stops searching for a match.
    pub fn stop_search_for_match() -> Result<(), async_channel::TrySendError<OnlineMatchmakerRequest>> {
        super::online::ONLINE_MATCHMAKER.try_send(OnlineMatchmakerRequest::StopSearch)
    }
}