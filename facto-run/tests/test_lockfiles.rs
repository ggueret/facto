mod common;

use serde_json::{Value, json};

/// Resolve `facto-run/tests/fixtures/<name>` to an absolute path.
fn fixture(name: &str) -> String {
    let mut p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("tests");
    p.push("fixtures");
    p.push(name);
    p.to_str().expect("fixture path utf8").to_owned()
}

#[tokio::test]
async fn test_parse_lockfile_cargo_fixture() {
    let client = common::spawn_facto_mcp().await;
    let args = json!({"path": fixture("cargo/Cargo.lock")});
    let result: Value = common::call_tool(&client, "parse_lockfile", args).await;

    assert_eq!(result["format"], "cargo_lock");
    let deps = result["deps"].as_array().expect("deps is array");
    // serde + tokio are crates.io-sourced; my-local-crate is path-like and skipped.
    assert_eq!(deps.len(), 2);
    assert!(
        deps.iter()
            .any(|d| d["name"] == "serde" && d["version"] == "1.0.215")
    );
    assert!(
        deps.iter()
            .any(|d| d["name"] == "tokio" && d["version"] == "1.42.0")
    );
    // One warning for the skipped workspace-local crate.
    assert_eq!(result["warnings"].as_array().unwrap().len(), 1);

    client.cancel().await.unwrap();
}

#[tokio::test]
#[ignore] // requires network (hits crates.io)
async fn test_check_lockfile_cargo_fixture() {
    let client = common::spawn_facto_mcp().await;
    let args = json!({"path": fixture("cargo/Cargo.lock")});
    let result: Value = common::call_tool(&client, "check_lockfile", args).await;

    assert_eq!(result["format"], "cargo_lock");
    let deps = result["deps"].as_array().expect("deps is array");
    assert_eq!(deps.len(), 2);
    let serde = deps.iter().find(|d| d["name"] == "serde").unwrap();
    // `latest` may be null if the registry call fails; `outdated` is then null too.
    // The point of this test is that the flow runs end-to-end with a path param.
    assert_eq!(serde["current"], "1.0.215");
    assert_eq!(serde["registry"], "crates");

    client.cancel().await.unwrap();
}

#[tokio::test]
async fn test_parse_lockfile_rejects_relative_path() {
    let client = common::spawn_facto_mcp().await;
    let args = json!({"path": "Cargo.lock"});
    let err = common::call_tool_expect_err(&client, "parse_lockfile", args).await;
    assert!(err.contains("absolute"), "expected 'absolute' in {err}");
    client.cancel().await.unwrap();
}

#[tokio::test]
async fn test_parse_lockfile_rejects_nonexistent_path() {
    let client = common::spawn_facto_mcp().await;
    // Absolute path under temp_dir with a Cargo.lock basename in a
    // subdirectory that does not exist. Works on every OS (Windows
    // requires a drive/UNC prefix for Path::is_absolute).
    let p = std::env::temp_dir()
        .join("facto_lockfiles_does_not_exist_xyz")
        .join("Cargo.lock");
    let args = json!({"path": p.to_str().unwrap()});
    let err = common::call_tool_expect_err(&client, "parse_lockfile", args).await;
    assert!(
        err.contains("cannot stat") || err.contains("No such") || err.contains("not found"),
        "expected filesystem error in {err}"
    );
    client.cancel().await.unwrap();
}

#[tokio::test]
async fn test_parse_lockfile_rejects_unknown_extension() {
    let client = common::spawn_facto_mcp().await;
    // Use temp_dir().join(...) so the path is absolute on every OS.
    let p = std::env::temp_dir().join("random.txt");
    let args = json!({"path": p.to_str().unwrap()});
    let err = common::call_tool_expect_err(&client, "parse_lockfile", args).await;
    assert!(
        err.contains("unknown lockfile format"),
        "expected 'unknown lockfile format' in {err}"
    );
    client.cancel().await.unwrap();
}
