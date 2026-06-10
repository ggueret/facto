//! Parser for `pnpm-lock.yaml` (pnpm v9+ format).
//!
//! Packages are encoded as keys like `express@4.21.2` or
//! `@scope/name@1.0.0`. For scoped names the key may be quoted in YAML,
//! but serde_yaml_ng unquotes it for us.
//!
//! In pnpm v9+, individual package entries no longer carry a `dev: true`
//! flag. Dev/main classification lives under the top-level `importers`
//! section, which maps each importer to `dependencies` and
//! `devDependencies` objects referencing packages by key. For v1 we skip
//! that correlation and mark every dep as `DepGroup::Unknown`, matching
//! the honest "best-effort" contract for lockfiles that do not record
//! group information directly on the resolved entry.

use std::collections::BTreeMap;

use serde::Deserialize;
use serde::de::IgnoredAny;

use super::models::{DepGroup, LockfileFormat, ParseResult, ParseWarning, ResolvedDep};

#[derive(Deserialize)]
struct PnpmLock {
    #[serde(default)]
    packages: BTreeMap<String, IgnoredAny>,
}

/// Split an `[@scope/]name@version` key into `(name, version)`.
/// Handles scoped packages by finding the rightmost `@` that is not
/// the leading scope separator.
fn split_name_version(key: &str) -> Option<(&str, &str)> {
    // Scoped: "@scope/name@1.0.0" -> split at the last '@' which is index > 0.
    let (start, rest) = if let Some(stripped) = key.strip_prefix('@') {
        (1, stripped)
    } else {
        (0, key)
    };
    let at_in_rest = rest.rfind('@')?;
    let at_abs = start + at_in_rest;
    let name = &key[..at_abs];
    let version = &key[at_abs + 1..];
    if name.is_empty() || version.is_empty() {
        None
    } else {
        Some((name, version))
    }
}

pub(super) fn parse(content: &str) -> ParseResult {
    let parsed: PnpmLock = match serde_yaml_ng::from_str(content) {
        Ok(p) => p,
        Err(e) => {
            return ParseResult {
                format: LockfileFormat::PnpmLock,
                deps: Vec::new(),
                warnings: vec![ParseWarning {
                    line: None,
                    message: format!("invalid YAML: {}", e),
                }],
            };
        }
    };

    let mut deps = Vec::new();
    let mut warnings = Vec::new();

    for (key, _entry) in parsed.packages {
        let Some((name, version)) = split_name_version(&key) else {
            warnings.push(ParseWarning {
                line: None,
                message: format!("pnpm-lock.yaml: cannot parse key '{}'", key),
            });
            continue;
        };
        deps.push(ResolvedDep {
            name: name.to_string(),
            version: version.to_string(),
            registry: "npm".to_string(),
            group: DepGroup::Unknown,
        });
    }

    ParseResult {
        format: LockfileFormat::PnpmLock,
        deps,
        warnings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_name_version_simple() {
        assert_eq!(
            split_name_version("express@4.21.2"),
            Some(("express", "4.21.2"))
        );
    }

    #[test]
    fn test_split_name_version_scoped() {
        assert_eq!(
            split_name_version("@types/node@20.11.0"),
            Some(("@types/node", "20.11.0"))
        );
    }

    #[test]
    fn test_split_name_version_missing_version() {
        assert_eq!(split_name_version("express"), None);
    }

    #[test]
    fn test_split_name_version_empty() {
        assert_eq!(split_name_version(""), None);
    }
}
