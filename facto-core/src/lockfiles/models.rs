//! Data models for the lockfile parsing module.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Identifier for a recognised lockfile format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum LockfileFormat {
    UvLock,
    PoetryLock,
    PdmLock,
    PipfileLock,
    NpmLock,
    PnpmLock,
    CargoLock,
}

impl LockfileFormat {
    /// Return the registry identifier this lockfile format's deps belong to.
    ///
    /// Matches the registry IDs used by `RegistryManager::get_registry`.
    pub fn registry(&self) -> &'static str {
        match self {
            LockfileFormat::UvLock
            | LockfileFormat::PoetryLock
            | LockfileFormat::PdmLock
            | LockfileFormat::PipfileLock => "pypi",
            LockfileFormat::NpmLock | LockfileFormat::PnpmLock => "npm",
            LockfileFormat::CargoLock => "crates",
        }
    }
}

/// Dependency group as extracted from a lockfile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[non_exhaustive]
pub enum DepGroup {
    Main,
    Dev,
    Build,
    Optional { name: String },
    Unknown,
}

/// A resolved dependency extracted from a lockfile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ResolvedDep {
    pub name: String,
    pub version: String,
    pub registry: String,
    pub group: DepGroup,
}

/// A lockfile dependency checked against its registry for the latest version.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CheckedDep {
    pub name: String,
    pub current: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outdated: Option<bool>,
    pub registry: String,
    pub group: DepGroup,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// A non-fatal warning emitted while parsing a lockfile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ParseWarning {
    pub line: Option<usize>,
    pub message: String,
}

/// Outcome of a successful parse. May still contain warnings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ParseResult {
    pub format: LockfileFormat,
    pub deps: Vec<ResolvedDep>,
    pub warnings: Vec<ParseWarning>,
}

/// Fatal error when a lockfile cannot be parsed at all.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ParseError {
    #[error("unknown lockfile format: {0}")]
    UnknownFormat(String),
    #[error("invalid lockfile content: {0}")]
    InvalidContent(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolved_dep_roundtrip_json() {
        let dep = ResolvedDep {
            name: "fastapi".to_string(),
            version: "0.115.8".to_string(),
            registry: "pypi".to_string(),
            group: DepGroup::Main,
        };
        let json = serde_json::to_string(&dep).unwrap();
        assert!(json.contains("\"group\":{\"kind\":\"main\"}"));
        let back: ResolvedDep = serde_json::from_str(&json).unwrap();
        assert_eq!(dep, back);
    }

    #[test]
    fn test_dep_group_optional_serializes_with_name() {
        let group = DepGroup::Optional {
            name: "test".to_string(),
        };
        let json = serde_json::to_string(&group).unwrap();
        assert!(json.contains("\"name\":\"test\""));
        assert!(json.contains("\"kind\":\"optional\""));
        let back: DepGroup = serde_json::from_str(&json).unwrap();
        assert_eq!(group, back);
    }

    #[test]
    fn test_parse_result_empty_is_valid() {
        let result = ParseResult {
            format: LockfileFormat::UvLock,
            deps: vec![],
            warnings: vec![],
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"format\":\"uv_lock\""));
    }

    #[test]
    fn test_registry_for_python_formats() {
        assert_eq!(LockfileFormat::UvLock.registry(), "pypi");
        assert_eq!(LockfileFormat::PoetryLock.registry(), "pypi");
        assert_eq!(LockfileFormat::PdmLock.registry(), "pypi");
        assert_eq!(LockfileFormat::PipfileLock.registry(), "pypi");
    }

    #[test]
    fn test_registry_for_node_formats() {
        assert_eq!(LockfileFormat::NpmLock.registry(), "npm");
        assert_eq!(LockfileFormat::PnpmLock.registry(), "npm");
    }

    #[test]
    fn test_registry_for_rust_format() {
        assert_eq!(LockfileFormat::CargoLock.registry(), "crates");
    }
}
