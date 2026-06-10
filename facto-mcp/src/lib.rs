//! Local MCP server for facto -- real-time package ecosystem data for AI
//! coding agents.
//!
//! This crate exposes the [`FactoMcp`] server and its tools over rmcp.
//! The `facto-run` binary is the thin CLI wrapper that calls
//! [`serve_stdio`] from this crate; third parties can also embed
//! `FactoMcp` directly via [`FactoMcp::new`].
//!
//! ## Quick start
//!
//! ```no_run
//! # async fn run() -> anyhow::Result<()> {
//! let config = facto_core::config::Config::load()
//!     .map_err(|e| anyhow::anyhow!("config: {e}"))?;
//! facto_mcp::serve_stdio(&config).await
//! # }
//! ```
pub mod server;
pub mod tools;

pub use server::FactoMcp;

use anyhow::Context;
use facto_core::config::Config;
use facto_core::forges::manager::ForgeManager;
use facto_core::registries::manager::RegistryManager;
use facto_core::runtimes::manager::RuntimeManager;
use rmcp::ServiceExt;
use tokio_util::sync::CancellationToken;

/// Run the MCP server on stdio until the transport is closed or SIGINT is
/// received. Blocks the calling task.
///
/// Builds the registry / forge / runtime managers from `config`, wires
/// them into a [`FactoMcp`], and serves rmcp over stdin/stdout.
pub async fn serve_stdio(config: &Config) -> anyhow::Result<()> {
    let registries = RegistryManager::new(config).context("failed to create registry manager")?;
    let forges = ForgeManager::new(config).context("failed to create forge manager")?;
    let runtimes = RuntimeManager::new(config).context("failed to create runtime manager")?;

    let server = FactoMcp::new(registries, forges, runtimes);
    let transport = rmcp::transport::io::stdio();
    let ct = CancellationToken::new();

    // Cancel the token on SIGINT so rmcp closes the transport internally.
    let ct_signal = ct.clone();
    tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        tracing::info!("received SIGINT, shutting down");
        ct_signal.cancel();
    });

    tracing::info!("facto mcp starting (stdio)");
    let handle = server
        .serve_with_ct(transport, ct)
        .await
        .context("failed to start MCP server")?;

    let _ = handle.waiting().await;
    Ok(())
}
