use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::*;
use rmcp::tool_router;
use rmcp::{ErrorData as McpError, tool};
use schemars::JsonSchema;
use serde::Deserialize;

use super::{check_params, tool_error_result, unknown_id};
use crate::FactoMcp;

#[derive(Debug, Deserialize, JsonSchema)]
struct PinActionParams {
    /// GitHub Action in owner/repo format (e.g. "actions/checkout")
    action: String,
    /// Tag or version to pin (e.g. "v4", "v4.2.0"). If omitted, resolves the latest stable release.
    tag: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ListForgeReleasesParams {
    /// Repository owner (user or organisation)
    owner: String,
    /// Repository name
    repo: String,
    /// Forge ID (e.g. "github", "gitlab", "codeberg")
    forge: String,
    /// Maximum number of releases to return (default 20, max 100)
    limit: Option<usize>,
}

#[tool_router(router = tool_router_forge_inner, vis = "pub")]
impl FactoMcp {
    #[tool(
        description = "Resolve a GitHub Action to its commit SHA for secure workflow pinning. Supply chain attacks can mutate tags to point to malicious code. Always pin actions to their full commit SHA. If tag is omitted, resolves the latest stable release automatically. Examples: pin_action(\"actions/checkout\", \"v4\") or pin_action(\"actions/checkout\") for the latest version.",
        annotations(read_only_hint = true)
    )]
    async fn pin_action(
        &self,
        Parameters(p): Parameters<PinActionParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut params = vec![(p.action.as_str(), "action")];
        if let Some(t) = &p.tag {
            params.push((t.as_str(), "tag"));
        }
        if let Some(err) = check_params(&params) {
            return Ok(err);
        }
        let pin = match self
            .forges
            .pin_github_action(&p.action, p.tag.as_deref())
            .await
        {
            Ok(pin) => pin,
            Err(e) => return Ok(tool_error_result(e)),
        };
        Ok(CallToolResult::success(vec![Content::json(pin)?]))
    }

    #[tool(
        description = "List releases of a repository on a code forge with tags, dates, prerelease/draft flags, and downloadable assets.",
        annotations(read_only_hint = true)
    )]
    async fn list_forge_releases(
        &self,
        Parameters(p): Parameters<ListForgeReleasesParams>,
    ) -> Result<CallToolResult, McpError> {
        if let Some(err) =
            check_params(&[(&p.owner, "owner"), (&p.repo, "repo"), (&p.forge, "forge")])
        {
            return Ok(err);
        }

        let forge = match self.forges.get_forge(&p.forge) {
            Some(f) => f,
            None => return Ok(unknown_id("forge", &p.forge, "list_forges")),
        };

        let limit = p.limit.unwrap_or(20).min(100);
        let mut releases = match forge.list_releases(&p.owner, &p.repo, limit).await {
            Ok(r) => r,
            Err(e) => return Ok(tool_error_result(e)),
        };

        releases.truncate(limit);
        Ok(CallToolResult::success(vec![Content::json(releases)?]))
    }
}
