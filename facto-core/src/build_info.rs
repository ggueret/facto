//! Build-time provenance, populated by `build.rs`.
//!
//! Reports the git commit the binary was compiled from, so `facto --version`
//! and the MCP `initialize` handshake can state exactly which code is running.
//! Falls back to `unknown` when built outside a git checkout.

/// Crate version from Cargo (`CARGO_PKG_VERSION`).
pub const PKG_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Short git commit SHA, or `unknown` outside a git checkout.
pub const GIT_SHA: &str = env!("FACTO_GIT_SHA");

/// Commit date (`YYYY-MM-DD`), or empty when unavailable.
pub const GIT_DATE: &str = env!("FACTO_GIT_DATE");

/// Human-readable version for `--version`, e.g. `0.1.0-alpha.1 (665b3ca 2026-06-04)`.
pub const LONG_VERSION: &str = env!("FACTO_LONG_VERSION");

/// Semver build-metadata form for the MCP handshake, e.g. `0.1.0-alpha.1+665b3ca`.
pub const MCP_VERSION: &str = env!("FACTO_MCP_VERSION");

#[cfg(test)]
mod tests {
    #[test]
    fn long_version_embeds_pkg_version_and_sha() {
        assert!(super::LONG_VERSION.starts_with(super::PKG_VERSION));
        assert!(!super::GIT_SHA.is_empty());
        assert!(super::MCP_VERSION.contains(super::PKG_VERSION));
    }
}
