//! Parser for `Pipfile.lock` (Pipenv).

use std::collections::BTreeMap;

use serde::Deserialize;

use super::models::{DepGroup, LockfileFormat, ParseResult, ParseWarning, ResolvedDep};

#[derive(Deserialize)]
struct PipfileLock {
    #[serde(default)]
    default: BTreeMap<String, Entry>,
    #[serde(default)]
    develop: BTreeMap<String, Entry>,
}

#[derive(Deserialize)]
struct Entry {
    version: Option<String>,
}

/// Strip the `==` prefix Pipenv writes on exact versions. Returns `None`
/// if the prefix is missing, so the caller can emit a warning.
///
/// A PEP 440 identity pin (`===1.2.3`) starts with `===` — the third `=`
/// would leak into the version string if we only stripped `==`, so we reject
/// those here and let the caller emit a "not an exact pin" warning.
fn strip_exact(raw: &str) -> Option<&str> {
    raw.strip_prefix("==").filter(|s| !s.starts_with('='))
}

pub(super) fn parse(content: &str) -> ParseResult {
    let parsed: PipfileLock = match serde_json::from_str(content) {
        Ok(p) => p,
        Err(e) => {
            return ParseResult {
                format: LockfileFormat::PipfileLock,
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

    for (group, section) in [
        (DepGroup::Main, parsed.default),
        (DepGroup::Dev, parsed.develop),
    ] {
        for (name, entry) in section {
            let Some(raw_version) = entry.version else {
                warnings.push(ParseWarning {
                    line: None,
                    message: format!("Pipfile.lock: '{}' has no version", name),
                });
                continue;
            };
            let Some(version) = strip_exact(&raw_version) else {
                warnings.push(ParseWarning {
                    line: None,
                    message: format!(
                        "Pipfile.lock: '{}' version '{}' is not an exact pin",
                        name, raw_version
                    ),
                });
                continue;
            };
            deps.push(ResolvedDep {
                name,
                version: version.to_string(),
                registry: "pypi".to_string(),
                group: group.clone(),
            });
        }
    }

    ParseResult {
        format: LockfileFormat::PipfileLock,
        deps,
        warnings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_exact_rejects_triple_equals() {
        // PEP 440 identity pin (`===1.2.3`) is not an exact `==` pin.
        assert_eq!(strip_exact("===1.2.3"), None);
    }

    #[test]
    fn test_strip_exact_accepts_double_equals() {
        assert_eq!(strip_exact("==1.2.3"), Some("1.2.3"));
    }

    #[test]
    fn test_strip_exact_rejects_range() {
        assert_eq!(strip_exact(">=1.0"), None);
    }
}
