use anyhow::anyhow;
use clap::{Parser, Subcommand};
use facto_core::config::Config;
use tracing_subscriber::EnvFilter;

/// Real-time package ecosystem data for AI coding agents.
#[derive(Parser)]
#[command(name = "facto", version = facto_core::build_info::LONG_VERSION, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Run the MCP stdio server for AI coding agents.
    Mcp,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();
    match cli.command {
        Command::Mcp => {
            let config = Config::load().map_err(|e| anyhow!("failed to load config: {e}"))?;
            facto_mcp::serve_stdio(&config).await
        }
    }
}
