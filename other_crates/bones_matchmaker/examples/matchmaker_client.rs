use std::time::Duration;

use bones_matchmaker_proto::{
    MatchInfo, MatchmakerRequest, MatchmakerResponse, MATCH_ALPN, PLAY_ALPN,
};
use serde::{Deserialize, Serialize};
use tokio::task::JoinSet;

const CLIENT_PORT: u16 = 0;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Hello {
    i_am: String,
}

#[tokio::main]
async fn main() {
    if let Err(e) = client().await {
        eprintln!("Error: {e}");
    }
}

async fn client() -> anyhow::Result<()> {
    let secret_key = iroh_net::key::SecretKey::generate();
    let endpoint = iroh_net::MagicEndpoint::builder()
        .alpns(vec![MATCH_ALPN.to_vec(), PLAY_ALPN.to_vec()])
        .discovery(Box::new(
            iroh_net::discovery::ConcurrentDiscovery::from_services(vec![
                Box::new(iroh_net::discovery::dns::DnsDiscovery::n0_dns()),
                Box::new(iroh_net::discovery::pkarr_publish::PkarrPublisher::n0_dns(
                    secret_key.clone(),
                )),
            ]),
        ))
        .secret_key(secret_key)
        .bind(CLIENT_PORT)
        .await?;

    let i_am = std::env::args().nth(2).unwrap();
    let hello = Hello { i_am };
    println!("o  Opened client ID: {}. {hello:?}", endpoint.node_id());

    let server_id: iroh_net::NodeId = std::env::args().nth(3).expect("missing node id").parse()?;
    let server_addr = iroh_net::NodeAddr::new(server_id);

    // Connect to the server
    let conn = endpoint.connect(server_addr, MATCH_ALPN).await?;

    // Send a match request to the server
    let (mut send, mut recv) = conn.open_bi().await?;

    let message = MatchmakerRequest::RequestMatch(MatchInfo {
        client_count: std::env::args()
            .nth(1)
            .map(|x| x.parse().unwrap())
            .unwrap_or(0),
        match_data: b"example-client".to_vec(),
    });
    println!("=> Sending match request: {message:?}");
    let message = postcard::to_allocvec(&message)?;

    send.write_all(&message).await?;
    send.finish().await?;

    println!("o  Waiting for response");

    let message = recv.read_to_end(256).await?;
    let message: MatchmakerResponse = postcard::from_bytes(&message)?;

    if let MatchmakerResponse::Accepted = message {
        println!("<= Request accepted, waiting for match");
    } else {
        panic!("<= Unexpected message from server!");
    }

    let (player_idx, player_ids, _client_count) = loop {
        let mut recv = conn.accept_uni().await?;
        let message = recv.read_to_end(256).await?;
        let message: MatchmakerResponse = postcard::from_bytes(&message)?;

        match message {
            MatchmakerResponse::ClientCount(count) => {
                println!("<= {count} players in lobby");
            }
            MatchmakerResponse::Success {
                random_seed,
                player_idx,
                client_count,
                player_ids,
            } => {
                println!("<= Match is ready! Random seed: {random_seed}. Player IDX: {player_idx}. Client count: {client_count}");
                break (player_idx, player_ids, client_count as usize);
            }
            _ => panic!("<= Unexpected message from server"),
        }
    };

    println!("Closing matchmaking connection");
    conn.close(0u8.into(), b"done");

    let mut tasks = JoinSet::default();
    for (idx, player_id) in player_ids {
        if idx != player_idx {
            let endpoint_ = endpoint.clone();
            let hello = hello.clone();

            tasks.spawn(async move {
                let result = async move {
                    let conn = endpoint_.connect(player_id.clone(), PLAY_ALPN).await?;
                    println!("Connected to {}", player_id.node_id);

                    for _ in 0..3 {
                        println!("=> {hello:?}");
                        let mut sender = conn.open_uni().await?;
                        sender
                            .write_all(&postcard::to_allocvec(&hello.clone())?)
                            .await?;
                        sender.finish().await?;

                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }

                    conn.close(0u8.into(), b"done");

                    Ok::<_, anyhow::Error>(())
                };

                if let Err(e) = result.await {
                    eprintln!("<= Error: {e:?}");
                }
            });

            let endpoint = endpoint.clone();
            tasks.spawn(async move {
                if let Some(mut conn) = endpoint.accept().await {
                    let result = async {
                        let alpn = conn.alpn().await?;
                        if alpn.as_bytes() != PLAY_ALPN {
                            anyhow::bail!("unexpected ALPN: {}", alpn);
                        }
                        let conn = conn.await?;

                        for _ in 0..3 {
                            let mut recv = conn.accept_uni().await?;
                            println!("<= accepted connection");

                            let incomming = recv.read_to_end(256).await?;
                            let message: Hello = postcard::from_bytes(&incomming).unwrap();

                            println!("<= {message:?}");
                        }
                        Ok::<_, anyhow::Error>(())
                    };
                    if let Err(e) = result.await {
                        eprintln!("Error: {e:?}");
                    }
                }
            });
        }
    }

    // Wait for all tasks to finish
    while let Some(task) = tasks.join_next().await {
        task?;
    }

    // Shutdown the endpoint
    endpoint.close(0u8.into(), b"done").await?;

    Ok(())
}
