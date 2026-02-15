use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser;
use tracing::{error, info, Level};
use tracing_subscriber::FmtSubscriber;

use matrix_bridge_qq::bridge::QQBridge;
use matrix_bridge_qq::config::Config;

#[derive(Parser, Debug)]
#[command(name = "matrix-bridge-qq")]
#[command(version)]
#[command(about = "A Matrix-QQ bridge implemented in Rust with Salvo")]
struct Args {
    #[arg(short, long, default_value = "config.yaml")]
    config: PathBuf,

    #[arg(long)]
    generate_config: bool,
}

const EXAMPLE_CONFIG: &str = include_str!("../example-config.yaml");

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    if args.generate_config {
        println!("{EXAMPLE_CONFIG}");
        return Ok(());
    }

    FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .pretty()
        .init();

    let config_path = args.config.to_string_lossy();
    info!("loading config from {config_path}");

    let config = match Config::load(&config_path) {
        Ok(cfg) => cfg,
        Err(err) => {
            error!("failed to load config: {err}");
            return Err(err);
        }
    };

    let bridge = Arc::new(QQBridge::new(config).await?);

    let run_bridge = bridge.clone();
    tokio::select! {
        result = run_bridge.start() => {
            if let Err(err) = result {
                error!("bridge stopped with error: {err}");
                return Err(err);
            }
        }
        _ = tokio::signal::ctrl_c() => {
            info!("received shutdown signal");
        }
    }

    bridge.stop().await;
    info!("bridge stopped");

    Ok(())
}
