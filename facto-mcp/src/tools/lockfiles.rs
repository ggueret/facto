use std::path::Path;

use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::*;
use rmcp::tool_router;
use rmcp::{ErrorData as McpError, tool};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tokio::fs;

use facto_core::lockfiles;
use facto_core::lockfiles::LockfileFormat;

use crate::FactoMcp;

/// Maximum size of a lockfile body accepted via the MCP tool layer.
/// Protects against adversarial input (billion-laughs YAML, allocator DoS
/// via huge synthetic lockfiles) when an untrusted source supplies content.
const MAX_LOCKFILE_CONTENT: usize = 4 * 1024 * 1024; // 4 MiB

/// Validate `path` and read the referenced lockfile.
///
/// Rules enforced here (surfaces as `McpError::invalid_params` on failure):
/// - non-empty
/// - no null bytes (defence in depth: should not happen from a JSON client)
/// - absolute path -- relative paths are ambiguous because facto-mcp has
///   no concept of a "current project"; the agent should always pass a
///   fully-qualified path
/// - basename matches a known lockfile filename
/// - the final path component is a regular file -- symlinks, directories,
///   FIFOs, sockets and devices are rejected. This prevents an attacker
///   with write access to a queried directory from planting
///   `Cargo.lock -> ~/.ssh/id_rsa` and exfiltrating bytes via the parser
///   error path
/// - file size <= `MAX_LOCKFILE_CONTENT` (checked via `symlink_metadata()`
///   before reading, so an attacker cannot DoS the process by pointing at
///   a 100 GiB file -- we never allocate a 100 GiB buffer)
/// - content is valid UTF-8 (enforced by `read_to_string`)
///
/// Note: the size cap is best-effort. Between `symlink_metadata()` and
/// `read_to_string()`, a concurrent writer on the same path could
/// grow the file past `MAX_LOCKFILE_CONTENT` — the read will not
/// re-check. This is acceptable because the threat model is
/// agent-supplied paths on a local dev machine, not adversarial
/// concurrent writers on a shared filesystem.
async fn read_and_validate_lockfile(path: &str) -> Result<(LockfileFormat, String), McpError> {
    if path.is_empty() {
        return Err(McpError::invalid_params("path is required", None));
    }
    if path.contains('\0') {
        return Err(McpError::invalid_params(
            "path must not contain null bytes",
            None,
        ));
    }
    let p = Path::new(path);
    if !p.is_absolute() {
        return Err(McpError::invalid_params(
            format!("path must be absolute, got: {path}"),
            None,
        ));
    }

    let format = lockfiles::detect_format(path).ok_or_else(|| {
        McpError::invalid_params(format!("unknown lockfile format: {path}"), None)
    })?;

    // symlink_metadata (not metadata) so we see the final component as-is,
    // rather than following any symlink to its target.
    let metadata = fs::symlink_metadata(p)
        .await
        .map_err(|e| McpError::invalid_params(format!("cannot stat {path}: {e}"), None))?;
    if !metadata.file_type().is_file() {
        return Err(McpError::invalid_params(
            format!("path is not a regular file (symlink, directory, or special): {path}"),
            None,
        ));
    }
    if metadata.len() > MAX_LOCKFILE_CONTENT as u64 {
        return Err(McpError::invalid_params(
            format!(
                "file exceeds {} bytes (got {})",
                MAX_LOCKFILE_CONTENT,
                metadata.len()
            ),
            None,
        ));
    }

    let content = fs::read_to_string(p)
        .await
        .map_err(|e| McpError::invalid_params(format!("cannot read {path}: {e}"), None))?;

    Ok((format, content))
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ParseLockfileParams {
    /// Absolute path to the lockfile on the local filesystem. The
    /// basename must match a known lockfile filename: uv.lock,
    /// poetry.lock, pdm.lock, Pipfile.lock, package-lock.json,
    /// pnpm-lock.yaml, or Cargo.lock. facto reads the file itself
    /// (up to 4 MiB) — callers do not pass the content.
    path: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct CheckLockfileParams {
    /// Absolute path to the lockfile on the local filesystem. See
    /// `parse_lockfile` for supported filenames and size limits.
    path: String,
}

#[derive(Debug, Serialize, JsonSchema)]
struct CheckLockfileResponse {
    format: facto_core::lockfiles::LockfileFormat,
    deps: Vec<facto_core::lockfiles::CheckedDep>,
    warnings: Vec<facto_core::lockfiles::ParseWarning>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct DiscoverLockfilesParams {
    /// List of file paths to classify. Paths are matched against known
    /// lockfile filenames by their basename; the rest of the path is
    /// preserved in the output. Paths that do not match a known lockfile
    /// are silently filtered out. Supported filenames: uv.lock,
    /// poetry.lock, pdm.lock, Pipfile.lock, package-lock.json,
    /// pnpm-lock.yaml, Cargo.lock.
    paths: Vec<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
struct DiscoveredLockfile {
    /// The path exactly as provided in the input.
    path: String,
    /// The detected lockfile format.
    format: facto_core::lockfiles::LockfileFormat,
    /// The package registry this lockfile belongs to ("pypi", "npm", "crates").
    registry: String,
}

#[derive(Debug, Serialize, JsonSchema)]
struct DiscoverLockfilesResponse {
    lockfiles: Vec<DiscoveredLockfile>,
}

#[tool_router(router = tool_router_lockfiles_inner, vis = "pub")]
impl FactoMcp {
    #[tool(
        description = "Parse a lockfile at a given filesystem path and \
                       return the list of resolved dependencies (name, \
                       version, registry, group). facto reads the file \
                       itself; the agent only supplies an absolute path. \
                       No network calls. Supported lockfiles: uv.lock, \
                       poetry.lock, pdm.lock, Pipfile.lock, \
                       package-lock.json, pnpm-lock.yaml, Cargo.lock.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn parse_lockfile(
        &self,
        Parameters(p): Parameters<ParseLockfileParams>,
    ) -> Result<CallToolResult, McpError> {
        let (format, content) = read_and_validate_lockfile(&p.path).await?;
        let result = lockfiles::parse(&content, format);
        Ok(CallToolResult::success(vec![Content::json(result)?]))
    }

    #[tool(
        description = "Parse a lockfile at a given filesystem path and \
                       check each dependency against its registry for the \
                       latest version. Returns one entry per dep with \
                       current/latest/outdated status. facto reads the \
                       file itself; the agent only supplies an absolute \
                       path. Hits the upstream registries (pypi, npm, \
                       crates) directly.",
        annotations(read_only_hint = true)
    )]
    async fn check_lockfile(
        &self,
        Parameters(p): Parameters<CheckLockfileParams>,
    ) -> Result<CallToolResult, McpError> {
        let (format, content) = read_and_validate_lockfile(&p.path).await?;
        let parsed = lockfiles::parse(&content, format);
        let checked = self.registries.check_deps(parsed.deps).await;
        let response = CheckLockfileResponse {
            format,
            deps: checked,
            warnings: parsed.warnings,
        };
        Ok(CallToolResult::success(vec![Content::json(response)?]))
    }

    #[tool(
        description = "Filter a list of file paths down to the ones \
                       that match a known lockfile filename. Takes an \
                       array of paths (typically from a filesystem glob \
                       on the client side) and returns the subset that \
                       can be parsed by parse_lockfile / check_lockfile, \
                       each tagged with format and registry. Works \
                       entirely locally with no I/O. Use this to classify \
                       a monorepo's filesystem listing before iterating \
                       parse_lockfile per matched path.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn discover_lockfiles(
        &self,
        Parameters(p): Parameters<DiscoverLockfilesParams>,
    ) -> Result<CallToolResult, McpError> {
        const MAX_PATHS: usize = 1024;
        if p.paths.len() > MAX_PATHS {
            return Err(McpError::invalid_params(
                format!("too many paths: got {}, max {}", p.paths.len(), MAX_PATHS),
                None,
            ));
        }

        // Individual paths are not run through `read_and_validate_lockfile`
        // (unlike `parse_lockfile` / `check_lockfile`): this tool does no I/O
        // and `detect_format` is safe on arbitrary `&str` (empty strings and
        // null bytes just fail to match any known filename and are filtered
        // out below).
        let lockfiles: Vec<DiscoveredLockfile> = p
            .paths
            .into_iter()
            .filter_map(|path| {
                lockfiles::detect_format(&path).map(|format| DiscoveredLockfile {
                    registry: format.registry().to_string(),
                    path,
                    format,
                })
            })
            .collect();

        let response = DiscoverLockfilesResponse { lockfiles };
        Ok(CallToolResult::success(vec![Content::json(response)?]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_empty_path_rejected() {
        let err = read_and_validate_lockfile("").await.unwrap_err();
        assert!(format!("{err:?}").contains("required"));
    }

    #[tokio::test]
    async fn test_null_byte_rejected() {
        let err = read_and_validate_lockfile("/abs/Cargo.lock\0")
            .await
            .unwrap_err();
        assert!(format!("{err:?}").contains("null"));
    }

    #[tokio::test]
    async fn test_relative_path_rejected() {
        let err = read_and_validate_lockfile("Cargo.lock").await.unwrap_err();
        assert!(format!("{err:?}").contains("absolute"));
    }

    #[tokio::test]
    async fn test_unknown_format_rejected() {
        // Use temp_dir().join(...) so the path is absolute on every OS.
        // Windows' Path::is_absolute requires a drive/UNC prefix, which
        // "/tmp/random.txt" does not have.
        let p = std::env::temp_dir().join("random.txt");
        let err = read_and_validate_lockfile(p.to_str().unwrap())
            .await
            .unwrap_err();
        assert!(format!("{err:?}").contains("unknown lockfile format"));
    }

    #[tokio::test]
    async fn test_nonexistent_file_rejected() {
        // Absolute path under temp_dir with a Cargo.lock basename in a
        // subdirectory that does not exist. Guarantees `is_absolute`
        // passes on every OS and `fs::metadata` fails with NotFound.
        let p = std::env::temp_dir()
            .join("facto_lockfiles_does_not_exist_xyz")
            .join("Cargo.lock");
        let err = read_and_validate_lockfile(p.to_str().unwrap())
            .await
            .unwrap_err();
        let msg = format!("{err:?}");
        // Either "cannot stat" or an NotFound-like message surfaces.
        assert!(
            msg.contains("cannot stat") || msg.contains("not found") || msg.contains("No such")
        );
    }

    #[tokio::test]
    async fn test_reads_valid_lockfile() {
        // Per-test subdir: `detect_format` matches the basename only, so
        // we must end with "/Cargo.lock". Embedding a unique dir name
        // avoids collisions if `cargo test` runs two tokio::tests that
        // both touch Cargo.lock in parallel.
        let dir = std::env::temp_dir().join("facto_lockfiles_test_reads_valid");
        std::fs::create_dir_all(&dir).unwrap();
        let tmp = dir.join("Cargo.lock");
        std::fs::write(&tmp, "version = 3\n").unwrap();
        let (format, content) = read_and_validate_lockfile(tmp.to_str().unwrap())
            .await
            .unwrap();
        assert_eq!(format, facto_core::lockfiles::LockfileFormat::CargoLock);
        assert!(content.contains("version = 3"));
        std::fs::remove_file(&tmp).ok();
        std::fs::remove_dir(&dir).ok();
    }

    #[tokio::test]
    async fn test_oversized_file_rejected() {
        let dir = std::env::temp_dir().join("facto_lockfiles_test_oversized");
        std::fs::create_dir_all(&dir).unwrap();
        let tmp = dir.join("Cargo.lock");
        // Write MAX + 1 bytes of valid-ish TOML.
        let payload = "x".repeat(MAX_LOCKFILE_CONTENT + 1);
        std::fs::write(&tmp, &payload).unwrap();
        let err = read_and_validate_lockfile(tmp.to_str().unwrap())
            .await
            .unwrap_err();
        let msg = format!("{err:?}");
        assert!(msg.contains("exceeds"));
        std::fs::remove_file(&tmp).ok();
        std::fs::remove_dir(&dir).ok();
    }

    #[tokio::test]
    async fn test_invalid_utf8_rejected() {
        let dir = std::env::temp_dir().join("facto_lockfiles_test_invalid_utf8");
        std::fs::create_dir_all(&dir).unwrap();
        let tmp = dir.join("Cargo.lock");
        std::fs::write(&tmp, [0xff, 0xfe, 0xfd]).unwrap();
        let err = read_and_validate_lockfile(tmp.to_str().unwrap())
            .await
            .unwrap_err();
        let msg = format!("{err:?}");
        assert!(msg.contains("cannot read"));
        std::fs::remove_file(&tmp).ok();
        std::fs::remove_dir(&dir).ok();
    }

    #[tokio::test]
    async fn test_path_is_directory_rejected() {
        let dir = std::env::temp_dir().join("facto_lockfiles_test_path_is_dir");
        std::fs::create_dir_all(&dir).unwrap();
        // The "file" we point at is actually a directory: detect_format
        // succeeds (basename is "Cargo.lock"), symlink_metadata succeeds,
        // but the is_file check rejects it as "not a regular file".
        let sub = dir.join("Cargo.lock");
        std::fs::create_dir_all(&sub).unwrap();
        let err = read_and_validate_lockfile(sub.to_str().unwrap())
            .await
            .unwrap_err();
        let msg = format!("{err:?}");
        assert!(
            msg.contains("not a regular file"),
            "expected regular-file rejection, got: {msg}"
        );
        std::fs::remove_dir(&sub).ok();
        std::fs::remove_dir(&dir).ok();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn test_symlink_rejected() {
        // An agent-supplied path whose final component is a symlink must
        // be rejected, regardless of what the symlink points to. Otherwise
        // an attacker who controls a writable directory could plant
        // `Cargo.lock -> ~/.ssh/id_rsa` and facto would happily read the
        // target on the agent's behalf, leaking bytes into the parser
        // error path (and the MCP response).
        let dir = std::env::temp_dir().join("facto_lockfiles_test_symlink");
        std::fs::create_dir_all(&dir).unwrap();
        let target = dir.join("target.txt");
        std::fs::write(&target, "irrelevant content\n").unwrap();
        let link = dir.join("Cargo.lock");
        // Defensive cleanup for re-runs of a failing test.
        std::fs::remove_file(&link).ok();
        std::os::unix::fs::symlink(&target, &link).unwrap();

        let err = read_and_validate_lockfile(link.to_str().unwrap())
            .await
            .unwrap_err();
        let msg = format!("{err:?}");
        assert!(
            msg.contains("symlink") || msg.contains("not a regular file"),
            "expected symlink rejection, got: {msg}"
        );

        std::fs::remove_file(&link).ok();
        std::fs::remove_file(&target).ok();
        std::fs::remove_dir(&dir).ok();
    }
}
