pub mod forge;
pub mod lockfiles;
pub mod meta;
pub mod packages;
pub mod runtime;
pub mod search;

use crate::FactoMcp;
use rmcp::handler::server::tool::ToolRouter;
use rmcp::model::{CallToolResult, Content};
use serde::Serialize;

pub fn build_tool_router() -> ToolRouter<FactoMcp> {
    FactoMcp::tool_router_meta_inner()
        + FactoMcp::tool_router_packages_inner()
        + FactoMcp::tool_router_search_inner()
        + FactoMcp::tool_router_forge_inner()
        + FactoMcp::tool_router_runtime_inner()
        + FactoMcp::tool_router_lockfiles_inner()
}

/// Body of a tool-execution failure, returned inside a `CallToolResult` with
/// `is_error: true` so the model can read the failure and self-correct or
/// retry (per the MCP tool error guidance). Protocol-level `McpError` is
/// reserved for malformed requests and genuine internal errors.
#[derive(Debug, Serialize)]
pub(crate) struct ToolError {
    /// Stable kind: invalid_input | rate_limited | timeout | not_found |
    /// not_supported | upstream.
    pub error: &'static str,
    /// Human-readable, actionable message (never a raw stack trace).
    pub message: String,
    /// True when retrying the same call may succeed (rate limit, timeout,
    /// transport). Mirrors the community `isRetryable` convention.
    #[serde(rename = "isRetryable")]
    pub is_retryable: bool,
}

impl ToolError {
    /// A non-retryable bad-input error the model can correct (e.g. an unknown
    /// registry id or an empty parameter).
    pub(crate) fn invalid_input(message: String) -> Self {
        ToolError {
            error: "invalid_input",
            message,
            is_retryable: false,
        }
    }
}

impl From<facto_core::registries::RegistryError> for ToolError {
    fn from(e: facto_core::registries::RegistryError) -> Self {
        use facto_core::registries::RegistryError as E;
        match e {
            E::RateLimited => ToolError {
                error: "rate_limited",
                message: "registry rate-limited the request".into(),
                is_retryable: true,
            },
            E::Timeout => ToolError {
                error: "timeout",
                message: "registry request timed out".into(),
                is_retryable: true,
            },
            E::NotFound => ToolError {
                error: "not_found",
                message: "not found on this registry".into(),
                is_retryable: false,
            },
            E::NotSupported => ToolError {
                error: "not_supported",
                message: "this registry does not support that operation".into(),
                is_retryable: false,
            },
            E::Http(err) => ToolError {
                error: "upstream",
                message: format!("registry request failed: {err}"),
                is_retryable: true,
            },
            E::Parse(msg) => ToolError {
                error: "upstream",
                message: format!("registry response error: {msg}"),
                is_retryable: false,
            },
        }
    }
}

impl From<facto_core::forges::ForgeError> for ToolError {
    fn from(e: facto_core::forges::ForgeError) -> Self {
        use facto_core::forges::ForgeError as E;
        match e {
            E::RateLimited => ToolError {
                error: "rate_limited",
                message: "forge rate-limited the request".into(),
                is_retryable: true,
            },
            E::Timeout => ToolError {
                error: "timeout",
                message: "forge request timed out".into(),
                is_retryable: true,
            },
            E::NotSupported => ToolError {
                error: "not_supported",
                message: "this forge does not support that operation".into(),
                is_retryable: false,
            },
            E::Http(err) => ToolError {
                error: "upstream",
                message: format!("forge request failed: {err}"),
                is_retryable: true,
            },
            E::Parse(msg) => ToolError {
                error: "upstream",
                message: format!("forge response error: {msg}"),
                is_retryable: false,
            },
        }
    }
}

impl From<facto_core::runtimes::RuntimeError> for ToolError {
    fn from(e: facto_core::runtimes::RuntimeError) -> Self {
        use facto_core::runtimes::RuntimeError as E;
        match e {
            E::Unknown(id) => ToolError::invalid_input(format!(
                "unknown runtime '{id}'; call list_runtimes for valid ids"
            )),
            E::Http(err) => ToolError {
                error: "upstream",
                message: format!("runtime metadata request failed: {err}"),
                is_retryable: true,
            },
            E::Parse(msg) => ToolError {
                error: "upstream",
                message: format!("runtime metadata error: {msg}"),
                is_retryable: false,
            },
        }
    }
}

/// Build an `is_error: true` tool result from any upstream/tool error. Use for
/// execution failures and bad input the model can correct; keep `McpError` for
/// malformed requests and internal errors.
pub(crate) fn tool_error_result(err: impl Into<ToolError>) -> CallToolResult {
    let te: ToolError = err.into();
    let content = match Content::json(&te) {
        Ok(c) => c,
        Err(_) => Content::text(te.message),
    };
    CallToolResult::error(vec![content])
}

/// `is_error` result for an unrecognised registry/forge/runtime id, pointing
/// the model at the discovery tool that lists valid ids.
pub(crate) fn unknown_id(kind: &str, id: &str, list_tool: &str) -> CallToolResult {
    tool_error_result(ToolError::invalid_input(format!(
        "unknown {kind} '{id}'; call {list_tool} for valid ids"
    )))
}

/// Validate string parameters. Returns the `is_error` result to hand back on
/// the first invalid one (empty, over-long, or NUL-containing), else `None`.
pub(crate) fn check_params(params: &[(&str, &str)]) -> Option<CallToolResult> {
    for (value, name) in params {
        let msg = if value.is_empty() {
            format!("{name} must not be empty")
        } else if value.len() > 256 {
            format!("{name} exceeds 256 characters")
        } else if value.contains('\0') {
            format!("{name} contains null bytes")
        } else {
            continue;
        };
        return Some(tool_error_result(ToolError::invalid_input(msg)));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use facto_core::registries::RegistryError;

    #[test]
    fn registry_rate_limit_is_retryable() {
        let te: ToolError = RegistryError::RateLimited.into();
        assert_eq!(te.error, "rate_limited");
        assert!(te.is_retryable);
    }

    #[test]
    fn registry_not_supported_is_not_retryable() {
        let te: ToolError = RegistryError::NotSupported.into();
        assert_eq!(te.error, "not_supported");
        assert!(!te.is_retryable);
    }

    #[test]
    fn tool_error_result_sets_is_error_and_json_body() {
        let result = tool_error_result(RegistryError::RateLimited);
        assert_eq!(result.is_error, Some(true));
        assert_eq!(result.content.len(), 1);
    }

    #[test]
    fn check_params_flags_first_invalid_only() {
        assert!(check_params(&[("ok", "a"), ("", "b")]).is_some());
        assert!(check_params(&[("ok", "a"), ("also-ok", "b")]).is_none());
    }
}
