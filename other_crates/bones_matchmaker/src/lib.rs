#![doc = include_str!("../README.md")]
// This cfg_attr is needed because `rustdoc::all` includes lints not supported on stable
#![cfg_attr(doc, allow(unknown_lints))]
#![deny(rustdoc::all)]
#[macro_use]
extern crate tracing;

use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::Arc;

use bones_matchmaker_proto::MATCH_ALPN;
use iroh::key::SecretKey;
use matchmaker::Matchmaker;

pub mod cli;
mod helpers;
mod lobbies;
mod matchmaker;
mod matchmaking;

#[derive(clap::Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Config {
    /// The server address to listen on
    #[clap(short, long = "listen", default_value = "0.0.0.0:8943")]
    listen_addr: SocketAddr,
    /// If enabled, prints the current secret key. Use with caution.
    #[clap(long)]
    print_secret_key: bool,
    /// Use this secret key for the node
    #[clap(short, long, env = "BONES_MATCHMAKER_SECRET_KEY")]
    secret_key: Option<iroh::key::SecretKey>,
}

async fn server(args: Config) -> anyhow::Result<()> {
    let port = args.listen_addr.port();

    match args.secret_key {
        Some(ref key) => {
            info!("Using existing key: {}", key.public());
        }
        None => {
            info!("Generating new key");
        }
    }

    let secret_key = args.secret_key.unwrap_or_else(SecretKey::generate);

    if args.print_secret_key {
        println!("Secret Key: {}", secret_key);
    }

    let endpoint = iroh::Endpoint::builder()
        .alpns(vec![MATCH_ALPN.to_vec()])
        .discovery(Box::new(
            iroh::discovery::ConcurrentDiscovery::from_services(vec![
                Box::new(
                    iroh::discovery::local_swarm_discovery::LocalSwarmDiscovery::new(
                        secret_key.public(),
                    )?,
                ),
                Box::new(iroh::discovery::dns::DnsDiscovery::n0_dns()),
                Box::new(iroh::discovery::pkarr::PkarrPublisher::n0_dns(
                    secret_key.clone(),
                )),
            ]),
        ))
        .secret_key(secret_key)
        .bind_addr_v4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port))
        .bind()
        .await?;

    let my_addr = endpoint.node_addr().await?;

    info!(address=?my_addr, "Started server");

    println!("Node ID: {}", my_addr.node_id);

    let matchmaker = Matchmaker::new(endpoint.clone());
    let router = iroh::protocol::Router::builder(endpoint)
        .accept(MATCH_ALPN, Arc::new(matchmaker))
        .spawn()
        .await?;

    // wait for shutdown
    tokio::signal::ctrl_c().await?;

    router.shutdown().await?;

    info!("Server shutdown");

    Ok(())
}
