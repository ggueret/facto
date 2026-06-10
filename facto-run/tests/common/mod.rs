use std::path::PathBuf;

use rmcp::{
    ServiceExt,
    model::{CallToolRequestParams, CallToolResult, Content, RawContent},
    service::RunningService,
    transport::{ConfigureCommandExt, TokioChildProcess},
};
use serde_json::{Map, Value};

/// Type alias for the MCP client returned by `spawn_facto_mcp`.
pub type McpClient = RunningService<rmcp::RoleClient, ()>;

/// Resolve the path to a cargo binary built in the workspace.
///
/// Appends `std::env::consts::EXE_SUFFIX` so the lookup works on
/// Windows (where cargo produces `<name>.exe`) as well as unix.
fn cargo_bin(name: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop(); // from facto-run/ to workspace root
    path.push("target");
    path.push("debug");
    path.push(format!("{name}{}", std::env::consts::EXE_SUFFIX));
    path
}

/// Spawn the `facto` binary with the `mcp` subcommand as a child process and
/// connect as an MCP client. The caller is responsible for shutting the client
/// down (e.g. via `client.cancel().await`).
pub async fn spawn_facto_mcp() -> McpClient {
    let bin = cargo_bin("facto");
    assert!(
        bin.exists(),
        "facto binary not found at {bin:?} -- run `cargo build -p facto-run` first"
    );

    // Tracing routes to stderr (see facto-run/src/main.rs), so warn-level
    // logs do not corrupt the JSON-RPC stdout channel.
    let transport = TokioChildProcess::new(tokio::process::Command::new(&bin).configure(|cmd| {
        cmd.arg("mcp").env("RUST_LOG", "warn");
    }))
    .expect("failed to spawn facto mcp");

    ().serve(transport)
        .await
        .expect("MCP client handshake failed")
}

/// Call an MCP tool with JSON arguments, returning the result as a
/// `serde_json::Value`.
pub async fn call_tool(client: &McpClient, name: &str, args: Value) -> Value {
    let arguments: Option<Map<String, Value>> = match args {
        Value::Object(map) => Some(map),
        Value::Null => None,
        other => panic!("call_tool args must be a JSON object or null, got: {other}"),
    };

    let mut params = CallToolRequestParams::new(name.to_owned());
    if let Some(args) = arguments {
        params = params.with_arguments(args);
    }

    let result: CallToolResult = client
        .call_tool(params)
        .await
        .unwrap_or_else(|e| panic!("call_tool({name}) failed: {e}"));

    assert!(
        result.is_error != Some(true),
        "tool {name} returned an error: {result:?}"
    );

    extract_json(&result.content)
}

/// Call an MCP tool with no arguments, returning the result as a
/// `serde_json::Value`.
///
/// Each integration test binary compiles `common/mod.rs` independently,
/// so this helper appears unused in binaries that never call it (it is
/// only used by `test_meta.rs`). Allow dead_code to keep the shared
/// helper available without failing `-D warnings` in CI.
#[allow(dead_code)]
pub async fn call_tool_no_args(client: &McpClient, name: &str) -> Value {
    call_tool(client, name, Value::Null).await
}

/// Call an MCP tool expecting a tool-level error. Returns the error
/// message as a `String`. Panics if the tool call unexpectedly succeeds.
#[allow(dead_code)]
pub async fn call_tool_expect_err(client: &McpClient, name: &str, args: Value) -> String {
    let arguments: Option<Map<String, Value>> = match args {
        Value::Object(map) => Some(map),
        Value::Null => None,
        other => panic!("call_tool args must be a JSON object or null, got: {other}"),
    };

    let mut params = CallToolRequestParams::new(name.to_owned());
    if let Some(args) = arguments {
        params = params.with_arguments(args);
    }

    // rmcp surfaces invalid-params errors at the RPC layer (Err), not as
    // is_error=true content. Accept either form.
    match client.call_tool(params).await {
        Err(e) => e.to_string(),
        Ok(result) => {
            assert_eq!(
                result.is_error,
                Some(true),
                "expected error from tool {name}, got success: {result:?}"
            );
            // is_error=true results carry a text body with the message.
            result
                .content
                .iter()
                .find_map(|c| {
                    if let RawContent::Text(t) = &c.raw {
                        Some(t.text.clone())
                    } else {
                        None
                    }
                })
                .unwrap_or_default()
        }
    }
}

/// Extract JSON from the first text content block.
fn extract_json(content: &[Content]) -> Value {
    let text = content
        .iter()
        .find_map(|c| {
            if let RawContent::Text(t) = &c.raw {
                Some(&t.text)
            } else {
                None
            }
        })
        .expect("no text content in tool result");

    serde_json::from_str(text).unwrap_or_else(|_| Value::String(text.clone()))
}
