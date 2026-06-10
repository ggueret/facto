use rmcp::model::*;
use rmcp::tool_router;
use rmcp::{ErrorData as McpError, tool};

use crate::FactoMcp;

#[tool_router(router = tool_router_meta_inner, vis = "pub")]
impl FactoMcp {
    #[tool(
        description = "List all supported package registries with their IDs and display names. Call this first if you don't know which registry ID to use.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn list_registries(&self) -> Result<CallToolResult, McpError> {
        let registries: Vec<_> = self
            .registries
            .list_registries()
            .into_iter()
            .map(|(id, name)| serde_json::json!({ "id": id, "name": name }))
            .collect();
        Ok(CallToolResult::success(vec![Content::json(registries)?]))
    }

    #[tool(
        description = "List all supported code forges with their IDs and display names.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn list_forges(&self) -> Result<CallToolResult, McpError> {
        let forges: Vec<_> = self
            .forges
            .list_forges()
            .into_iter()
            .map(|(id, name)| serde_json::json!({ "id": id, "name": name }))
            .collect();
        Ok(CallToolResult::success(vec![Content::json(forges)?]))
    }

    #[tool(
        description = "List all supported language runtimes with their IDs and display names.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn list_runtimes(&self) -> Result<CallToolResult, McpError> {
        let runtimes: Vec<_> = self
            .runtimes
            .list_runtimes()
            .into_iter()
            .map(|(id, name)| serde_json::json!({ "id": id, "name": name }))
            .collect();
        Ok(CallToolResult::success(vec![Content::json(runtimes)?]))
    }
}
