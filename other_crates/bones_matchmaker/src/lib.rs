#![doc = include_str!("../README.md")]
// This cfg_attr is needed because `rustdoc::all` includes lints not supported on stable
#![cfg_attr(doc, allow(unknown_lints))]
#![deny(rustdoc::all)]
#[macro_use]
extern crate tracing;

use std::net::SocketAddr;

pub mod cli;

mod matchmaker;
mod proxy;

#[derive(clap::Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Config {
    /// The server address to listen on
    #[clap(short, long = "listen", default_value = "0.0.0.0:8943")]
    listen_addr: SocketAddr,
}

pub const ALPN: &[u8] = b"/bones/match/0";

async fn server(args: Config) -> anyhow::Result<()> {
    let port = args.listen_addr.port();

    let secret_key = iroh_net::key::SecretKey::generate();
    let endpoint = iroh_net::MagicEndpoint::builder()
        .alpns(vec![ALPN.to_vec()])
        .discovery(Box::new(
            iroh_net::discovery::ConcurrentDiscovery::from_services(vec![
                Box::new(iroh_net::discovery::dns::DnsDiscovery::n0_dns()),
                Box::new(iroh_net::discovery::pkarr_publish::PkarrPublisher::n0_dns(
                    secret_key.clone(),
                )),
            ]),
        ))
        .secret_key(secret_key)
        .bind(port)
        .await?;

    let my_addr = endpoint.my_addr().await?;

    info!(address=?my_addr, "Started server");

    println!("Node ID: {}", my_addr.node_id);

    // Listen for incomming connections
    while let Some(connecting) = endpoint.accept().await {
        let connection = connecting.await;

        match connection {
            Ok(conn) => {
                info!(
                    connection_id = conn.stable_id(),
                    "Accepted connection from client"
                );

                // Spawn a task to handle the new connection
                tokio::task::spawn(matchmaker::handle_connection(conn));
            }
            Err(e) => error!("Error opening client connection: {e:?}"),
        }
    }

    info!("Server shutdown");

    Ok(())
}
