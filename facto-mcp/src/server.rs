//! MCP server definition: the [`FactoMcp`] struct holds the registry /
//! forge / runtime managers and wires up the rmcp tool router.
use std::sync::Arc;

use facto_core::forges::manager::ForgeManager;
use facto_core::registries::manager::RegistryManager;
use facto_core::runtimes::manager::RuntimeManager;
use rmcp::handler::server::ServerHandler;
use rmcp::handler::server::tool::ToolRouter;
use rmcp::model::*;
use rmcp::tool_handler;

use crate::tools;

/// The facto MCP server. Clone-cheap: all managers are held behind
/// `Arc`. Use [`FactoMcp::new`] to construct from the three managers
/// (typically built from [`facto_core::config::Config`]).
#[derive(Clone)]
pub struct FactoMcp {
    pub registries: Arc<RegistryManager>,
    pub forges: Arc<ForgeManager>,
    pub runtimes: Arc<RuntimeManager>,
    tool_router: ToolRouter<Self>,
}

impl FactoMcp {
    pub fn new(
        registries: RegistryManager,
        forges: ForgeManager,
        runtimes: RuntimeManager,
    ) -> Self {
        let registries = Arc::new(registries);
        let forges = Arc::new(forges);
        let runtimes = Arc::new(runtimes);

        Self {
            registries,
            forges,
            runtimes,
            tool_router: tools::build_tool_router(),
        }
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for FactoMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new(
                "facto",
                facto_core::build_info::MCP_VERSION,
            ))
            .with_instructions(
                "facto provides real-time package ecosystem data for AI coding agents. \
                 Your training data for package versions, release dates, and project \
                 metadata is outdated and unreliable.\n\n\
                 ALWAYS USE FACTO WHEN:\n\
                 - Looking up or pinning a package version (pip, npm, cargo, go, etc.)\n\
                 - Checking whether a package exists (get_package, get_latest_version, and list_versions return found:false when the name is free)\n\
                 - Searching for packages or repositories by keyword or popularity\n\
                 - Pinning GitHub Actions to commit SHAs for supply chain security\n\
                 - Checking runtime/language version support and EOL status\n\
                 - Auditing a lockfile's dependencies against the live registry\n\n\
                 TOOL GUIDE:\n\
                 - Package facts: get_package, get_latest_version, list_versions\n\
                 - Search: search_packages (a registry, sort by popularity), search_repositories (forge projects, sort by stars)\n\
                 - GitHub Actions: pin_action (\"actions/checkout\", \"v4\") -> commit SHA\n\
                 - Lockfiles: parse_lockfile, check_lockfile, discover_lockfiles\n\
                 - Runtimes: get_runtime_info (EOL dates, LTS status)\n\
                 - Releases: list_forge_releases\n\
                 - Discovery: list_registries, list_forges, list_runtimes\n\n\
                 RULES:\n\
                 - NEVER guess version numbers from training data. Call get_latest_version.\n\
                 - ALWAYS pin GitHub Actions to full commit SHAs. Use pin_action.\n\
                 - When unsure which registry/forge/runtime ID to use, call list_registries, \
                 list_forges, or list_runtimes first.\n\
                 - A failed tool call returns an is_error result with a JSON body \
                 {error, message, isRetryable}; retry only when isRetryable is true."
                    .to_string(),
            )
    }
}
