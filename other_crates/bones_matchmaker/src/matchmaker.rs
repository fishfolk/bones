use bones_matchmaker_proto::{MatchInfo, MatchmakerRequest, MatchmakerResponse};
use iroh_net::{magic_endpoint::get_remote_node_id, MagicEndpoint, NodeAddr};
use once_cell::sync::Lazy;
use quinn::{Connection, ConnectionError};
use scc::HashMap;

pub async fn handle_connection(ep: MagicEndpoint, conn: Connection) {
    let connection_id = conn.stable_id();
    debug!(connection_id, "Accepted matchmaker connection");

    if let Err(e) = impl_matchmaker(ep, conn).await {
        match e.downcast::<ConnectionError>() {
            Ok(conn_err) => match conn_err {
                ConnectionError::ApplicationClosed(e) => {
                    debug!(connection_id, "Application close connection: {e:?}");
                }
                e => {
                    error!(connection_id, "Error in matchmaker connection: {e:?}");
                }
            },
            Err(e) => {
                error!(connection_id, "Error in matchmaker connection: {e:?}");
            }
        }
    }
}

/// The matchmaker state
#[derive(Default)]
struct State {
    /// The mapping of match info to the vector connected clients in the waiting room.
    rooms: HashMap<MatchInfo, Vec<Connection>>,
}

static STATE: Lazy<State> = Lazy::new(State::default);

/// After a matchmaker connection is established, it will open a bi-directional channel with the
/// client.
///
/// At this point the client is free to engage in the matchmaking protocol over that channel.
async fn impl_matchmaker(ep: iroh_net::MagicEndpoint, conn: Connection) -> anyhow::Result<()> {
    let connection_id = conn.stable_id();

    loop {
        // Get the next channel open or connection close event
        tokio::select! {
            close = conn.closed() => {
                debug!("Connection closed {close:?}");
                return Ok(());
            }
            bi = conn.accept_bi() => {
                let (mut send, mut recv) = bi?;

                // Parse matchmaker request
                let request: MatchmakerRequest =
                    postcard::from_bytes(&recv.read_to_end(256).await?)?;

                match request {
                    MatchmakerRequest::RequestMatch(match_info) => {
                        debug!(connection_id, ?match_info, "Got request for match");

                        // Accept request
                        let message = postcard::to_allocvec(&MatchmakerResponse::Accepted)?;
                        send.write_all(&message).await?;
                        send.finish().await?;

                        let player_count = match_info.client_count;

                        let mut members_to_join = Vec::new();
                        let mut members_to_notify = Vec::new();

                        // Make sure room exists
                        STATE
                            .rooms
                            .insert_async(match_info.clone(), Vec::new())
                            .await
                            .ok();

                        STATE
                            .rooms
                            .update_async(&match_info, |match_info, members| {
                                // Add the current client to the room
                                members.push(conn.clone());

                                // Spawn task to wait for connction to close and remove it from the room if it does
                                let conn = conn.clone();
                                let info = match_info.clone();
                                tokio::task::spawn(async move {
                                    conn.closed().await;
                                    let members = STATE
                                        .rooms
                                        .update_async(&info, |_, members| {
                                            let mut was_removed = false;
                                            members.retain(|x| {
                                                if x.stable_id() != conn.stable_id() {
                                                    true
                                                } else {
                                                    was_removed = true;
                                                    false
                                                }
                                            });

                                            if was_removed {
                                                Some(members.clone())
                                            } else {
                                                None
                                            }
                                        })
                                        .await
                                        .flatten();
                                    if let Some(members) = members {
                                        let result = async {
                                            let message = postcard::to_allocvec(
                                                &MatchmakerResponse::ClientCount(
                                                    members.len().try_into()?
                                                ),
                                            )?;
                                            for conn in members {
                                                let mut send = conn.open_uni().await?;
                                                send.write_all(&message).await?;
                                                send.finish().await?;
                                            }
                                            Ok::<(), anyhow::Error>(())
                                        };
                                        result.await.ok();
                                    }
                                });

                                let member_count = members.len();

                                // If we have a complete room
                                debug!(
                                    ?match_info,
                                    "Room now has {}/{} members", member_count, player_count
                                );

                                if member_count >= player_count as _ {
                                    // Clear the room
                                    members_to_join.append(members);
                                } else {
                                    members_to_notify = members.clone();
                                }
                            })
                            .await;

                        if !members_to_notify.is_empty() {
                            let message = postcard::to_allocvec(&MatchmakerResponse::ClientCount(
                                members_to_notify.len().try_into()?
                            ))?;
                            for conn in members_to_notify {
                                let mut send = conn.open_uni().await?;
                                send.write_all(&message).await?;
                                send.finish().await?;
                            }
                        }

                        if !members_to_join.is_empty() {
                            // Send the match ID to all of the clients in the room
                            let mut player_ids = Vec::new();
                            let random_seed = rand::random();

                            for (idx, conn) in members_to_join.iter().enumerate() {
                                let id = get_remote_node_id(&conn)?;
                                let mut addr = NodeAddr::new(id);
                                if let Some(info) = ep.connection_info(id) {
                                    if let Some(relay_url) = info.relay_url {
                                        addr = addr.with_relay_url(relay_url.relay_url);
                                    }
                                    addr = addr.with_direct_addresses(
                                        info.addrs.into_iter().map(|addr| addr.addr),
                                    );
                                }

                                player_ids.push((u32::try_from(idx)?, addr));
                            }

                            for (player_idx, conn) in members_to_join.into_iter().enumerate() {
                                // Respond with success
                                let message =
                                    postcard::to_allocvec(&MatchmakerResponse::Success {
                                        random_seed,
                                        client_count: player_count,
                                        player_idx: player_idx.try_into()?,
                                        player_ids: player_ids.clone(),
                                    })?;
                                let mut send = conn.open_uni().await?;
                                send.write_all(&message).await?;
                                send.finish().await?;

                                // Close connection, we are done here
                                conn.close(0u32.into(), b"done");
                            }

                            // cleanup
                            STATE.rooms.remove_async(&match_info).await;
                        }
                    }
                }
            }
        }
    }
}
