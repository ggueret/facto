//! Parser for `package-lock.json` (npm v2/v3 format).
//!
//! The top-level `packages` map uses the install path as the key. The
//! empty key `""` is the root project itself and is skipped. Other keys
//! look like `node_modules/foo` or `node_modules/foo/node_modules/bar`
//! for nested installs; the package name is the segment after the last
//! `node_modules/`.

use std::collections::BTreeMap;

use serde::Deserialize;

use super::models::{DepGroup, LockfileFormat, ParseResult, ParseWarning, ResolvedDep};

#[derive(Deserialize)]
struct PackageLock {
    #[serde(default)]
    packages: BTreeMap<String, Entry>,
}

#[derive(Deserialize)]
struct Entry {
    version: Option<String>,
    #[serde(default)]
    dev: bool,
}

/// Extract the package name from a `node_modules/...` key. Returns `None`
/// for the root `""` entry.
fn extract_name(key: &str) -> Option<&str> {
    if key.is_empty() {
        return None;
    }
    let after_last = key.rsplit("node_modules/").next()?;
    if after_last.is_empty() {
        None
    } else {
        Some(after_last)
    }
}

pub(super) fn parse(content: &str) -> ParseResult {
    let parsed: PackageLock = match serde_json::from_str(content) {
        Ok(p) => p,
        Err(e) => {
            return ParseResult {
                format: LockfileFormat::NpmLock,
                deps: Vec::new(),
                warnings: vec![ParseWarning {
                    line: None,
                    message: format!("invalid JSON: {}", e),
                }],
            };
        }
    };

    let mut deps = Vec::new();
    let mut warnings = Vec::new();

    for (key, entry) in parsed.packages {
        let Some(name) = extract_name(&key) else {
            continue;
        };
        let Some(version) = entry.version else {
            warnings.push(ParseWarning {
                line: None,
                message: format!("package-lock.json: '{}' has no version", name),
            });
            continue;
        };
        deps.push(ResolvedDep {
            name: name.to_string(),
            version,
            registry: "npm".to_string(),
            group: if entry.dev {
                DepGroup::Dev
            } else {
                DepGroup::Main
            },
        });
    }

    ParseResult {
        format: LockfileFormat::NpmLock,
        deps,
        warnings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_name_simple() {
        assert_eq!(extract_name("node_modules/express"), Some("express"));
    }

    #[test]
    fn test_extract_name_nested() {
        assert_eq!(
            extract_name("node_modules/foo/node_modules/bar"),
            Some("bar")
        );
    }

    #[test]
    fn test_extract_name_scoped() {
        assert_eq!(
            extract_name("node_modules/@types/node"),
            Some("@types/node")
        );
    }

    #[test]
    fn test_extract_name_root_returns_none() {
        assert_eq!(extract_name(""), None);
    }
}
