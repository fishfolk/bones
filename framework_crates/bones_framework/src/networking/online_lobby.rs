#![allow(missing_docs)]

use bones_matchmaker_proto::{LobbyId, LobbyInfo, GameID, MatchmakerRequest, MatchmakerResponse, MATCH_ALPN};
use iroh_net::NodeId;
use crate::{
    networking::{get_network_endpoint, socket::establish_peer_connections, NetworkMatchSocket},
    prelude::*,
    utils::BiChannelServer,
};
use std::sync::Arc;
use super::online::{OnlineMatchmakerResponse, OnlineMatchmakerRequest};

async fn connect_to_matchmaker(id: NodeId) -> anyhow::Result<iroh_quinn::Connection> {
    let ep = get_network_endpoint().await;
    Ok(ep.connect(id.into(), MATCH_ALPN).await?)
}

pub async fn list_lobbies(
    matchmaker_channel: &BiChannelServer<OnlineMatchmakerRequest, OnlineMatchmakerResponse>,
    id: NodeId,
    game_id: GameID,
) -> anyhow::Result<()> {
    let conn = connect_to_matchmaker(id).await?;

    let (mut send, mut recv) = conn.open_bi().await?;

    let message = MatchmakerRequest::ListLobbies(game_id);
    let message = postcard::to_allocvec(&message)?;
    send.write_all(&message).await?;
    send.finish().await?;

    let response = recv.read_to_end(5 * 1024).await?;
    let message: MatchmakerResponse = postcard::from_bytes(&response)?;

    match message {
        MatchmakerResponse::LobbiesList(lobbies) => {
            matchmaker_channel.try_send(OnlineMatchmakerResponse::LobbiesList(lobbies))?;
        }
        other => anyhow::bail!("Unexpected message from matchmaker: {other:?}"),
    }

    Ok(())
}

pub async fn create_lobby(
    matchmaker_channel: &BiChannelServer<OnlineMatchmakerRequest, OnlineMatchmakerResponse>,
    id: NodeId,
    lobby_info: LobbyInfo,
) -> anyhow::Result<()> {
    let conn = connect_to_matchmaker(id).await?;

    let (mut send, mut recv) = conn.open_bi().await?;

    let message = MatchmakerRequest::CreateLobby(lobby_info);
    let message = postcard::to_allocvec(&message)?;
    send.write_all(&message).await?;
    send.finish().await?;

    let response = recv.read_to_end(256).await?;
    let message: MatchmakerResponse = postcard::from_bytes(&response)?;

    match message {
        MatchmakerResponse::LobbyCreated(lobby_id) => {
            matchmaker_channel.try_send(OnlineMatchmakerResponse::LobbyCreated(lobby_id))?;
        }
        MatchmakerResponse::Error(err) => {
            matchmaker_channel.try_send(OnlineMatchmakerResponse::Error(err))?;
        }
        other => anyhow::bail!("Unexpected message from matchmaker: {other:?}"),
    }

    Ok(())
}

pub async fn join_lobby(
    matchmaker_channel: &BiChannelServer<OnlineMatchmakerRequest, OnlineMatchmakerResponse>,
    id: NodeId,
    game_id: GameID,
    lobby_id: LobbyId,
    password: Option<String>,
) -> anyhow::Result<()> {
    let conn = connect_to_matchmaker(id).await?;

    let (mut send, mut recv) = conn.open_bi().await?;

    let message = MatchmakerRequest::JoinLobby(game_id, lobby_id.clone(), password);
    let message = postcard::to_allocvec(&message)?;
    send.write_all(&message).await?;
    send.finish().await?;

    let response = recv.read_to_end(256).await?;
    let message: MatchmakerResponse = postcard::from_bytes(&response)?;

    match message {
        MatchmakerResponse::LobbyJoined(joined_lobby_id) => {
            matchmaker_channel.try_send(OnlineMatchmakerResponse::LobbyJoined {
                lobby_id: joined_lobby_id,
                player_count: 0, // We don't have this information yet
            })?;

            // Wait for further messages (player count updates or game start)
            while let Ok(recv) = conn.accept_uni().await {
                let mut recv = recv;
                let message = recv.read_to_end(5 * 1024).await?;
                let message: MatchmakerResponse = postcard::from_bytes(&message)?;

                match message {
                    MatchmakerResponse::ClientCount(count) => {
                        matchmaker_channel.try_send(OnlineMatchmakerResponse::PlayerCount(count as _))?;
                    }
                    MatchmakerResponse::Success { random_seed, player_idx, client_count, player_ids } => {
                        let peer_connections = establish_peer_connections(
                            player_idx,
                            client_count,
                            player_ids,
                            None,
                        ).await?;

                        let socket = super::socket::Socket::new(player_idx, peer_connections);

                        matchmaker_channel.try_send(OnlineMatchmakerResponse::GameStarting {
                            socket: NetworkMatchSocket(Arc::new(socket)),
                            player_idx: player_idx as _,
                            player_count: client_count as _,
                            random_seed
                        })?;
                        break;
                    }
                    MatchmakerResponse::Error(err) => {
                        matchmaker_channel.try_send(OnlineMatchmakerResponse::Error(err))?;
                        break;
                    }
                    other => anyhow::bail!("Unexpected message from matchmaker: {other:?}"),
                }
            }
        }
        MatchmakerResponse::Error(err) => {
            matchmaker_channel.try_send(OnlineMatchmakerResponse::Error(err))?;
        }
        other => anyhow::bail!("Unexpected message from matchmaker: {other:?}"),
    }

    Ok(())
}

// Public interface functions
pub fn list_lobbies_request(matchmaking_server: NodeId, game_id: GameID) -> Result<(), async_channel::TrySendError<OnlineMatchmakerRequest>> {
    super::online::ONLINE_MATCHMAKER.try_send(OnlineMatchmakerRequest::ListLobbies { id: matchmaking_server, game_id })
}

pub fn create_lobby_request(matchmaking_server: NodeId, lobby_info: LobbyInfo) -> Result<(), async_channel::TrySendError<OnlineMatchmakerRequest>> {
    super::online::ONLINE_MATCHMAKER.try_send(OnlineMatchmakerRequest::CreateLobby { id: matchmaking_server, lobby_info })
}

pub fn join_lobby_request(matchmaking_server: NodeId,game_id: GameID, lobby_id: LobbyId, password: Option<String>) -> Result<(), async_channel::TrySendError<OnlineMatchmakerRequest>> {
    super::online::ONLINE_MATCHMAKER.try_send(OnlineMatchmakerRequest::JoinLobby { id: matchmaking_server , game_id, lobby_id, password })
}