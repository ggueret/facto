//! Shared TOML `[[package]]` parser used by uv, poetry, pdm, and cargo.
//!
//! The four formats publish TOML lockfiles with very similar shapes, so a
//! single permissive deserialisation struct covers them all. Each format
//! then applies its own group-detection logic on top of the shared raw data.

use serde::Deserialize;

use super::models::{DepGroup, LockfileFormat, ParseResult, ParseWarning, ResolvedDep};

#[derive(Deserialize, Default)]
struct TomlLockfile {
    #[serde(default)]
    package: Vec<RawPackage>,
}

#[derive(Deserialize)]
struct RawPackage {
    name: String,
    version: Option<String>,
    /// Present on uv.lock and Cargo.lock; shape differs. We only inspect it
    /// to reject non-registry sources (path, git, workspace).
    source: Option<toml::Value>,
    /// Present on poetry.lock pre-v2.
    category: Option<String>,
    /// Present on pdm.lock.
    groups: Option<Vec<String>>,
}

/// Parse the shared `[[package]]` array. Returns the raw entries and a
/// warning for a structurally invalid file (which yields an empty vec).
fn parse_raw(content: &str) -> (Vec<RawPackage>, Vec<ParseWarning>) {
    match toml::from_str::<TomlLockfile>(content) {
        Ok(parsed) => (parsed.package, Vec::new()),
        Err(e) => (
            Vec::new(),
            vec![ParseWarning {
                line: None,
                message: format!("invalid TOML: {}", e),
            }],
        ),
    }
}

/// Parse a `uv.lock` file.
///
/// Registry: always `"pypi"`. Group detection in uv.lock is non-trivial
/// (dependency groups live in the top-level `[[package]]` metadata of the
/// current project, not on each resolved dep), so every dep is marked as
/// `Main` unless it has no version at all.
pub(super) fn parse_uv(content: &str) -> ParseResult {
    let (raw, mut warnings) = parse_raw(content);
    let mut deps = Vec::with_capacity(raw.len());
    for pkg in raw {
        let Some(version) = pkg.version else {
            warnings.push(ParseWarning {
                line: None,
                message: format!("uv.lock: package '{}' has no version", pkg.name),
            });
            continue;
        };
        deps.push(ResolvedDep {
            name: pkg.name,
            version,
            registry: "pypi".to_string(),
            group: DepGroup::Main,
        });
    }
    ParseResult {
        format: LockfileFormat::UvLock,
        deps,
        warnings,
    }
}

/// Parse a `poetry.lock` file.
///
/// Registry: always `"pypi"`. Poetry pre-v2 marks dev deps with
/// `category = "dev"`. Poetry v2+ removed the field, so entries without
/// it are assumed `Main`.
pub(super) fn parse_poetry(content: &str) -> ParseResult {
    let (raw, mut warnings) = parse_raw(content);
    let mut deps = Vec::with_capacity(raw.len());
    for pkg in raw {
        let Some(version) = pkg.version else {
            warnings.push(ParseWarning {
                line: None,
                message: format!("poetry.lock: package '{}' has no version", pkg.name),
            });
            continue;
        };
        let group = match pkg.category.as_deref() {
            Some("dev") => DepGroup::Dev,
            Some("main") | None => DepGroup::Main,
            Some(other) => DepGroup::Optional {
                name: other.to_string(),
            },
        };
        deps.push(ResolvedDep {
            name: pkg.name,
            version,
            registry: "pypi".to_string(),
            group,
        });
    }
    ParseResult {
        format: LockfileFormat::PoetryLock,
        deps,
        warnings,
    }
}

/// Parse a `pdm.lock` file.
///
/// Registry: always `"pypi"`. pdm stores group membership as a list in
/// `groups`. `"default"` means main. A dep that appears in both
/// `"default"` and `"dev"` is classified as `Main` (default wins).
pub(super) fn parse_pdm(content: &str) -> ParseResult {
    let (raw, mut warnings) = parse_raw(content);
    let mut deps = Vec::with_capacity(raw.len());
    for pkg in raw {
        let Some(version) = pkg.version else {
            warnings.push(ParseWarning {
                line: None,
                message: format!("pdm.lock: package '{}' has no version", pkg.name),
            });
            continue;
        };
        let groups = pkg.groups.unwrap_or_default();
        let group = if groups.iter().any(|g| g == "default") {
            DepGroup::Main
        } else if groups.iter().any(|g| g == "dev") {
            DepGroup::Dev
        } else if let Some(first) = groups.first() {
            DepGroup::Optional {
                name: first.clone(),
            }
        } else {
            DepGroup::Unknown
        };
        deps.push(ResolvedDep {
            name: pkg.name,
            version,
            registry: "pypi".to_string(),
            group,
        });
    }
    ParseResult {
        format: LockfileFormat::PdmLock,
        deps,
        warnings,
    }
}

/// Parse a `Cargo.lock` file.
///
/// Registry: `"crates"` for entries with a `source` starting with
/// `"registry+"`. Entries without a source (path/workspace deps) are
/// skipped with a warning. Cargo.lock does not distinguish dev from
/// main dependencies, so every entry is `Unknown`.
pub(super) fn parse_cargo(content: &str) -> ParseResult {
    let (raw, mut warnings) = parse_raw(content);
    let mut deps = Vec::with_capacity(raw.len());
    for pkg in raw {
        let Some(version) = pkg.version else {
            warnings.push(ParseWarning {
                line: None,
                message: format!("Cargo.lock: package '{}' has no version", pkg.name),
            });
            continue;
        };
        // Path/workspace deps have no `source` and cannot be looked up
        // on a registry.
        let source_str = match pkg.source {
            Some(ref v) => v.as_str().map(|s| s.to_string()),
            None => None,
        };
        let Some(source) = source_str else {
            warnings.push(ParseWarning {
                line: None,
                message: format!(
                    "Cargo.lock: package '{}' has no registry source (path or workspace dep), skipped",
                    pkg.name
                ),
            });
            continue;
        };
        if !source.starts_with("registry+") {
            warnings.push(ParseWarning {
                line: None,
                message: format!(
                    "Cargo.lock: package '{}' has non-registry source '{}', skipped",
                    pkg.name, source
                ),
            });
            continue;
        }
        deps.push(ResolvedDep {
            name: pkg.name,
            version,
            registry: "crates".to_string(),
            group: DepGroup::Unknown,
        });
    }
    ParseResult {
        format: LockfileFormat::CargoLock,
        deps,
        warnings,
    }
}
