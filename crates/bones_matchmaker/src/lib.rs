#![doc = include_str!("../README.md")]
// This cfg_attr is needed because `rustdoc::all` includes lints not supported on stable
#![cfg_attr(doc, allow(unknown_lints))]
#![deny(rustdoc::all)]
#[macro_use]
extern crate tracing;

use std::{net::SocketAddr, sync::Arc, time::Duration};

use bevy_tasks::IoTaskPool;
use quinn::{Endpoint, EndpointConfig, ServerConfig, TransportConfig};
use quinn_runtime_bevy::BevyIoTaskPoolExecutor;

pub mod cli;

mod certs;
mod matchmaker;
mod proxy;

#[derive(clap::Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Config {
    /// The server address to listen on
    #[clap(short, long = "listen", default_value = "0.0.0.0:8943")]
    listen_addr: SocketAddr,
}

async fn server(args: Config) -> anyhow::Result<()> {
    let task_pool = IoTaskPool::get();

    // Generate certificate
    let (cert, key) = certs::generate_self_signed_cert()?;

    let mut transport_config = TransportConfig::default();
    transport_config.keep_alive_interval(Some(Duration::from_secs(5)));

    let mut server_config = ServerConfig::with_single_cert([cert].to_vec(), key)?;
    server_config.transport = Arc::new(transport_config);

    // Open Socket and create endpoint
    let socket = std::net::UdpSocket::bind(args.listen_addr)?;
    let endpoint = Endpoint::new(
        EndpointConfig::default(),
        Some(server_config),
        socket,
        Arc::new(BevyIoTaskPoolExecutor),
    )?;
    info!(address=%endpoint.local_addr()?, "Started server");

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
                task_pool
                    .spawn(matchmaker::handle_connection(conn))
                    .detach();
            }
            Err(e) => error!("Error opening client connection: {e:?}"),
        }
    }

    info!("Server shutdown");

    Ok(())
}
