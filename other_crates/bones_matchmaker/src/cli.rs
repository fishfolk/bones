use clap::Parser;
use tracing::metadata::LevelFilter;
use tracing::warn;

pub async fn start() {
    configure_logging();

    let args = crate::Config::parse();
    let secret_key = match std::env::var("BONES_MATCHMAKER_SECRET_KEY") {
        Ok(key) => match key.parse::<iroh_net::key::SecretKey>() {
            Ok(key) => Some(key),
            Err(_) => {
                warn!("invalid matchmaker key provided");
                None
            }
        },
        Err(_) => None,
    };

    if let Err(e) = super::server(args, secret_key).await {
        eprintln!("Error: {e}");
    }
}

fn configure_logging() {
    tracing::subscriber::set_global_default(
        tracing_subscriber::FmtSubscriber::builder()
            .with_env_filter(
                tracing_subscriber::EnvFilter::builder()
                    .with_default_directive(LevelFilter::INFO.into())
                    .from_env_lossy(),
            )
            .finish(),
    )
    .unwrap();
}
