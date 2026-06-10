mod common;

#[tokio::test]
#[ignore] // requires built binary
async fn test_list_registries() {
    let client = common::spawn_facto_mcp().await;
    let result = common::call_tool_no_args(&client, "list_registries").await;
    let registries = result.as_array().expect("expected array");
    assert!(
        registries.len() >= 9,
        "expected at least 9 registries, got {}",
        registries.len()
    );
    client.cancel().await.expect("failed to cancel client");
}

#[tokio::test]
#[ignore] // requires built binary
async fn test_list_forges() {
    let client = common::spawn_facto_mcp().await;
    let result = common::call_tool_no_args(&client, "list_forges").await;
    let forges = result.as_array().expect("expected array");
    assert!(
        forges.len() >= 3,
        "expected at least 3 forges, got {}",
        forges.len()
    );
    let names: Vec<&str> = forges.iter().filter_map(|f| f["name"].as_str()).collect();
    for expected in &["GitHub", "GitLab", "Codeberg"] {
        assert!(
            names.contains(expected),
            "expected forge {expected} in {names:?}"
        );
    }
    client.cancel().await.expect("failed to cancel client");
}

#[tokio::test]
#[ignore] // requires built binary
async fn test_list_runtimes() {
    let client = common::spawn_facto_mcp().await;
    let result = common::call_tool_no_args(&client, "list_runtimes").await;
    let runtimes = result.as_array().expect("expected array");
    assert!(
        runtimes.len() >= 14,
        "expected at least 14 runtimes, got {}",
        runtimes.len()
    );
    let names: Vec<&str> = runtimes.iter().filter_map(|r| r["name"].as_str()).collect();
    for expected in &["Python", "Rust", "Node.js"] {
        assert!(
            names.contains(expected),
            "expected runtime {expected} in {names:?}"
        );
    }
    client.cancel().await.expect("failed to cancel client");
}
