mod common;

use rmcp::model::CallToolRequestParams;
use serde_json::json;

// ---------- get_package ----------

#[tokio::test]
#[ignore] // requires network
async fn test_get_package_crates() {
    let client = common::spawn_facto_mcp().await;
    let result = common::call_tool(
        &client,
        "get_package",
        json!({"name": "serde", "registry": "crates"}),
    )
    .await;

    assert_eq!(result["name"].as_str(), Some("serde"));
    assert!(
        result["package"]["latest_version"].as_str().is_some(),
        "expected package.latest_version field, got: {result}"
    );
    // license is Option and may be absent (skip_serializing_if = None),
    // but the field should exist in the model -- just check it's not an error
    assert!(
        result["package"].get("license").is_none() || result["package"]["license"].is_string(),
        "expected package.license to be absent or a string, got: {result}"
    );
    client.cancel().await.expect("failed to cancel client");
}

#[tokio::test]
#[ignore] // requires network
async fn test_get_package_pypi() {
    let client = common::spawn_facto_mcp().await;
    let result = common::call_tool(
        &client,
        "get_package",
        json!({"name": "requests", "registry": "pypi"}),
    )
    .await;

    assert_eq!(result["name"].as_str(), Some("requests"));
    client.cancel().await.expect("failed to cancel client");
}

#[tokio::test]
#[ignore] // requires network
async fn test_get_package_npm() {
    let client = common::spawn_facto_mcp().await;
    let result = common::call_tool(
        &client,
        "get_package",
        json!({"name": "express", "registry": "npm"}),
    )
    .await;

    assert_eq!(result["name"].as_str(), Some("express"));
    client.cancel().await.expect("failed to cancel client");
}

#[tokio::test]
#[ignore] // requires network
async fn test_get_package_unknown_registry() {
    let client = common::spawn_facto_mcp().await;

    let params = CallToolRequestParams::new("get_package".to_owned()).with_arguments(
        json!({"name": "serde", "registry": "fakerepo"})
            .as_object()
            .unwrap()
            .clone(),
    );

    let result = client.call_tool(params).await.expect("call should succeed");
    assert_eq!(
        result.is_error,
        Some(true),
        "expected an is_error tool result for unknown registry, got: {result:?}"
    );

    client.cancel().await.expect("failed to cancel client");
}

// ---------- get_latest_version ----------

#[tokio::test]
#[ignore] // requires network
async fn test_get_latest_version() {
    let client = common::spawn_facto_mcp().await;
    let result = common::call_tool(
        &client,
        "get_latest_version",
        json!({"name": "serde", "registry": "crates"}),
    )
    .await;

    assert_eq!(result["found"].as_bool(), Some(true));
    assert!(
        result["version"]["version"].as_str().is_some(),
        "expected version.version field, got: {result}"
    );
    assert_eq!(
        result["version"]["prerelease"].as_bool(),
        Some(false),
        "expected version.prerelease == false, got: {result}"
    );
    client.cancel().await.expect("failed to cancel client");
}

// ---------- list_versions ----------

#[tokio::test]
#[ignore] // requires network
async fn test_list_versions() {
    let client = common::spawn_facto_mcp().await;
    let result = common::call_tool(
        &client,
        "list_versions",
        json!({"name": "serde", "registry": "crates"}),
    )
    .await;

    assert_eq!(result["found"].as_bool(), Some(true));
    let versions = result["versions"]
        .as_array()
        .expect("expected versions array");
    assert!(!versions.is_empty(), "expected non-empty versions array");
    assert!(
        versions[0]["version"].as_str().is_some(),
        "expected version field on first item, got: {:?}",
        versions[0]
    );
    client.cancel().await.expect("failed to cancel client");
}

#[tokio::test]
#[ignore] // requires network
async fn test_list_versions_stable_only() {
    let client = common::spawn_facto_mcp().await;
    let result = common::call_tool(
        &client,
        "list_versions",
        json!({"name": "serde", "registry": "crates", "stable_only": true}),
    )
    .await;

    let versions = result["versions"]
        .as_array()
        .expect("expected versions array");
    assert!(!versions.is_empty(), "expected non-empty versions array");
    for v in versions {
        assert_eq!(
            v["prerelease"].as_bool(),
            Some(false),
            "expected all versions to have prerelease == false, got: {v}"
        );
    }
    client.cancel().await.expect("failed to cancel client");
}

// ---------- search_packages ----------

#[tokio::test]
#[ignore] // requires network
async fn test_search_packages() {
    let client = common::spawn_facto_mcp().await;
    let result = common::call_tool(
        &client,
        "search_packages",
        json!({"query": "json", "registry": "crates"}),
    )
    .await;

    let results = result.as_array().expect("expected array");
    assert!(!results.is_empty(), "expected non-empty search results");
    client.cancel().await.expect("failed to cancel client");
}

// ---------- get_runtime_info ----------

#[tokio::test]
#[ignore] // requires network
async fn test_get_runtime_info() {
    let client = common::spawn_facto_mcp().await;
    let result = common::call_tool(&client, "get_runtime_info", json!({"runtime": "python"})).await;

    // get_runtime_info returns a RuntimeInfo object with a "versions" array
    let versions = result["versions"]
        .as_array()
        .expect("expected versions array");
    assert!(!versions.is_empty(), "expected non-empty versions array");
    client.cancel().await.expect("failed to cancel client");
}

// ---------- pin_action ----------

#[tokio::test]
#[ignore] // requires network
async fn test_pin_action_with_tag() {
    let client = common::spawn_facto_mcp().await;
    let result = common::call_tool(
        &client,
        "pin_action",
        json!({"action": "actions/checkout", "tag": "v4"}),
    )
    .await;

    assert_eq!(result["action"].as_str(), Some("actions/checkout"));
    assert_eq!(result["tag"].as_str(), Some("v4"));

    let sha = result["commit_sha"]
        .as_str()
        .expect("expected commit_sha field");
    assert_eq!(sha.len(), 40, "expected 40-char hex SHA, got: {sha}");
    assert!(
        sha.chars().all(|c| c.is_ascii_hexdigit()),
        "expected hex chars in SHA, got: {sha}"
    );

    let pinned = result["pinned"].as_str().expect("expected pinned field");
    assert!(
        pinned.contains('@'),
        "expected pinned to contain '@', got: {pinned}"
    );

    client.cancel().await.expect("failed to cancel client");
}

#[tokio::test]
#[ignore] // requires network
async fn test_pin_action_latest() {
    let client = common::spawn_facto_mcp().await;
    let result =
        common::call_tool(&client, "pin_action", json!({"action": "actions/checkout"})).await;

    assert!(
        result["tag"].as_str().is_some(),
        "expected tag to be resolved, got: {result}"
    );
    let sha = result["commit_sha"]
        .as_str()
        .expect("expected commit_sha field");
    assert_eq!(sha.len(), 40, "expected 40-char hex SHA, got: {sha}");

    client.cancel().await.expect("failed to cancel client");
}

#[tokio::test]
#[ignore] // requires network
async fn test_pin_action_invalid_format() {
    let client = common::spawn_facto_mcp().await;

    let params = CallToolRequestParams::new("pin_action".to_owned()).with_arguments(
        json!({"action": "invalid-no-slash"})
            .as_object()
            .unwrap()
            .clone(),
    );

    let result = client.call_tool(params).await.expect("call should succeed");
    assert_eq!(
        result.is_error,
        Some(true),
        "expected an is_error tool result for invalid action format, got: {result:?}"
    );

    client.cancel().await.expect("failed to cancel client");
}

// ---------- list_forge_releases ----------

#[tokio::test]
#[ignore] // requires network
async fn test_list_forge_releases() {
    let client = common::spawn_facto_mcp().await;
    let result = common::call_tool(
        &client,
        "list_forge_releases",
        json!({"owner": "serde-rs", "repo": "serde", "forge": "github"}),
    )
    .await;

    let releases = result.as_array().expect("expected array");
    assert!(!releases.is_empty(), "expected non-empty releases array");
    assert!(
        releases[0]["tag"].as_str().is_some(),
        "expected tag field on first release, got: {:?}",
        releases[0]
    );
    client.cancel().await.expect("failed to cancel client");
}
