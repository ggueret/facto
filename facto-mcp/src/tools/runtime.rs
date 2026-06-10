use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::*;
use rmcp::tool_router;
use rmcp::{ErrorData as McpError, tool};
use schemars::JsonSchema;
use serde::Deserialize;

use super::{check_params, tool_error_result};
use crate::FactoMcp;

#[derive(Debug, Deserialize, JsonSchema)]
struct GetRuntimeInfoParams {
    /// Runtime ID (e.g. "python", "node", "rust")
    runtime: String,
}

#[tool_router(router = tool_router_runtime_inner, vis = "pub")]
impl FactoMcp {
    #[tool(
        description = "Get current version and end-of-life status for a language or runtime. Returns all release cycles with EOL dates, LTS status, and support timelines. Use this to verify if a runtime version is still supported before recommending it.",
        annotations(read_only_hint = true)
    )]
    async fn get_runtime_info(
        &self,
        Parameters(p): Parameters<GetRuntimeInfoParams>,
    ) -> Result<CallToolResult, McpError> {
        if let Some(err) = check_params(&[(&p.runtime, "runtime")]) {
            return Ok(err);
        }
        let info = match self.runtimes.get_runtime_info(&p.runtime).await {
            Ok(info) => info,
            Err(e) => return Ok(tool_error_result(e)),
        };
        Ok(CallToolResult::success(vec![Content::json(info)?]))
    }
}
