use anyhow::Result;
use bones_matchmaker_proto::{LobbyId, LobbyInfo, LobbyListItem, MatchInfo, MatchmakerRequest, MatchmakerResponse};
use iroh_net::{Endpoint,NodeAddr};
use once_cell::sync::Lazy;
use quinn::Connection;
use scc::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::helpers::{generate_random_seed, generate_unique_id, hash_password};
use scc::HashMap as SccHashMap;




struct GameLobbies {
    game_id: String,
    lobbies: HashMap<LobbyId, LobbyInfo>,
}

#[derive(Default)]
struct State {
    game_lobbies: HashMap<String, GameLobbies>,
    lobby_connections: SccHashMap<(String, LobbyId), Vec<Connection>>,
    matchmaking_rooms: SccHashMap<MatchInfo, Vec<Connection>>,
}

static STATE: Lazy<Arc<Mutex<State>>> = Lazy::new(|| Arc::new(Mutex::new(State::default())));

pub async fn handle_connection(ep: Endpoint, conn: Connection) -> Result<()> {
    let connection_id = conn.stable_id();
    debug!(connection_id, "Accepted matchmaker connection");

    loop {
        tokio::select! {
            close = conn.closed() => {
                debug!("Connection closed {close:?}");
                return Ok(());
            }
            bi = conn.accept_bi() => {
                let (mut send, mut recv) = bi?;
                let request: MatchmakerRequest = postcard::from_bytes(&recv.read_to_end(256).await?)?;

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
                    MatchmakerRequest::JoinLobby(lobby_id, password) => {
                        handle_join_lobby(conn.clone(), lobby_id, password, &mut send).await?;
                    }
                }
            }
        }
    }
}

async fn handle_list_lobbies(game_id: String, send: &mut quinn::SendStream) -> Result<()> {
    Ok(())
}

async fn handle_create_lobby(conn: Connection, lobby_info: LobbyInfo, send: &mut quinn::SendStream) -> Result<()> {
    Ok(())
}

async fn handle_join_lobby(conn: Connection, lobby_id: LobbyId, password: Option<String>, send: &mut quinn::SendStream) -> Result<()> {
    Ok(())
}

async fn handle_request_match(ep: Endpoint, conn: Connection, match_info: MatchInfo, send: &mut quinn::SendStream) -> Result<()> {
    Ok(())
}

async fn start_match(ep: Endpoint, members: Vec<Connection>, match_info: &MatchInfo) -> Result<()> {
    Ok(())
}