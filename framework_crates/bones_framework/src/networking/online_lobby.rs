#![allow(missing_docs)]

use super::online::{
    MatchmakerConnectionState,OnlineMatchmakerRequest, OnlineMatchmakerResponse,
    READ_TO_END_BYTE_COUNT,
};
use crate::{
    networking::{socket::establish_peer_connections, NetworkMatchSocket},
    prelude::*,
    utils::BiChannelServer,
};
use bones_matchmaker_proto::{GameID, LobbyId, LobbyInfo, MatchmakerRequest, MatchmakerResponse};
use std::sync::Arc;
use tracing::info;

pub(crate) async fn resolve_list_lobbies(
    user_channel: &BiChannelServer<OnlineMatchmakerRequest, OnlineMatchmakerResponse>,
    matchmaker_connection_state: &mut MatchmakerConnectionState,
    game_id: GameID,
) -> anyhow::Result<()> {
    let conn = matchmaker_connection_state.acquire_connection().await?;
    let (mut send, mut recv) = conn.open_bi().await?;

    let message = MatchmakerRequest::ListLobbies(game_id);
    let message = postcard::to_allocvec(&message)?;
    send.write_all(&message).await?;
    send.finish().await?;

    let response = recv.read_to_end(5 * 1024).await?;
    let message: MatchmakerResponse = postcard::from_bytes(&response)?;

    match message {
        MatchmakerResponse::LobbiesList(lobbies) => {
            user_channel.try_send(OnlineMatchmakerResponse::LobbiesList(lobbies))?;
        }
        other => anyhow::bail!("Unexpected message from matchmaker: {other:?}"),
    }

    Ok(())
}

pub(crate) async fn resolve_create_lobby(
    user_channel: &BiChannelServer<OnlineMatchmakerRequest, OnlineMatchmakerResponse>,
    matchmaker_connection_state: &mut MatchmakerConnectionState,
    lobby_info: LobbyInfo,
) -> anyhow::Result<()> {
    let conn = matchmaker_connection_state.acquire_connection().await?;
    let (mut send, mut recv) = conn.open_bi().await?;

    let message = MatchmakerRequest::CreateLobby(lobby_info);
    let message = postcard::to_allocvec(&message)?;
    send.write_all(&message).await?;
    send.finish().await?;

    let response = recv.read_to_end(READ_TO_END_BYTE_COUNT).await?;
    let message: MatchmakerResponse = postcard::from_bytes(&response)?;

    match message {
        MatchmakerResponse::LobbyCreated(lobby_id) => {
            user_channel.try_send(OnlineMatchmakerResponse::LobbyCreated(lobby_id))?;
        }
        MatchmakerResponse::Error(err) => {
            user_channel.try_send(OnlineMatchmakerResponse::Error(err))?;
        }
        other => anyhow::bail!("Unexpected message from matchmaker: {other:?}"),
    }

    Ok(())
}

pub(crate) async fn resolve_join_lobby(
    user_channel: &BiChannelServer<OnlineMatchmakerRequest, OnlineMatchmakerResponse>,
    matchmaker_connection_state: &mut MatchmakerConnectionState,
    game_id: GameID,
    lobby_id: LobbyId,
    password: Option<String>,
) -> anyhow::Result<()> {
    let conn = matchmaker_connection_state.acquire_connection().await?;
    let (mut send, mut recv) = conn.open_bi().await?;

    let message = MatchmakerRequest::JoinLobby(game_id, lobby_id.clone(), password);
    let message = postcard::to_allocvec(&message)?;
    send.write_all(&message).await?;
    send.finish().await?;

    let response = recv.read_to_end(READ_TO_END_BYTE_COUNT).await?;
    let message: MatchmakerResponse = postcard::from_bytes(&response)?;

    match message {
        MatchmakerResponse::LobbyJoined(joined_lobby_id) => {
            user_channel.try_send(OnlineMatchmakerResponse::LobbyJoined {
                lobby_id: joined_lobby_id,
                player_count: 0, // We don't have this information yet
            })?;

            // Wait for further messages (updates or game start)
            while let Ok(recv) = conn.accept_uni().await {
                let mut recv = recv;
                let message = recv.read_to_end(5 * 1024).await?;
                let message: MatchmakerResponse = postcard::from_bytes(&message)?;

                match message {
                    MatchmakerResponse::LobbyUpdate { player_count } => {
                        info!("Online lobby updated player count: {player_count}");
                        user_channel
                            .try_send(OnlineMatchmakerResponse::LobbyUpdate { player_count })?;
                    }
                    MatchmakerResponse::Success {
                        random_seed,
                        player_idx,
                        player_count,
                        player_ids,
                    } => {
                        let peer_connections =
                            establish_peer_connections(player_idx, player_count, player_ids, None)
                                .await?;

                        let socket = super::socket::Socket::new(player_idx, peer_connections);

                        user_channel.try_send(OnlineMatchmakerResponse::GameStarting {
                            socket: NetworkMatchSocket(Arc::new(socket)),
                            player_idx: player_idx as _,
                            player_count: player_count as _,
                            random_seed,
                        })?;
                        break;
                    }
                    MatchmakerResponse::Error(err) => {
                        user_channel.try_send(OnlineMatchmakerResponse::Error(err))?;
                        break;
                    }
                    other => anyhow::bail!("Unexpected message from matchmaker: {other:?}"),
                }
            }
        }
        MatchmakerResponse::Error(err) => {
            user_channel.try_send(OnlineMatchmakerResponse::Error(err))?;
        }
        other => anyhow::bail!("Unexpected message from matchmaker: {other:?}"),
    }

    Ok(())
}