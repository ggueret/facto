pub mod cache;
pub mod crates_io;
pub mod dockerhub;
pub mod go;
pub mod manager;
pub mod maven;
pub mod npm;
pub mod nuget;
pub mod packagist;
pub mod pypi;
pub mod rubygems;
mod trait_def;

pub use trait_def::*;

use crate::http::{MAX_RESPONSE_BYTES, bytes_with_limit};
use crate::models::VersionInfo;

/// Read a response body with the default size cap and deserialize as JSON.
/// Use this instead of `resp.json()` to prevent allocator DoS from a
/// compromised or malicious upstream.
pub(crate) async fn bounded_json<T: serde::de::DeserializeOwned>(
    resp: reqwest::Response,
) -> RegistryResult<T> {
    let bytes = bytes_with_limit(resp, MAX_RESPONSE_BYTES)
        .await
        .map_err(|e| RegistryError::Parse(e.to_string()))?;
    serde_json::from_slice::<T>(&bytes)
        .map_err(|e| RegistryError::Parse(format!("json decode: {e}")))
}

/// Read a response body with the default size cap as UTF-8 text.
pub(crate) async fn bounded_text(resp: reqwest::Response) -> RegistryResult<String> {
    let bytes = bytes_with_limit(resp, MAX_RESPONSE_BYTES)
        .await
        .map_err(|e| RegistryError::Parse(e.to_string()))?;
    String::from_utf8(bytes).map_err(|e| RegistryError::Parse(format!("utf-8 decode: {e}")))
}

/// Map an HTTP status code to a registry error, or `None` for 2xx. Centralises
/// the "a non-success response is an error, not empty results" policy so search
/// and list endpoints never mask a 429 or 5xx as "no matches".
fn status_error(code: u16) -> Option<RegistryError> {
    match code {
        200..=299 => None,
        429 => Some(RegistryError::RateLimited),
        _ => Some(RegistryError::Parse(format!("unexpected status {code}"))),
    }
}

/// Return the response on 2xx, otherwise the mapped [`RegistryError`]. Use in
/// `search`/list paths so an upstream failure surfaces as an error instead of a
/// silent empty result set.
pub(crate) fn ensure_status(resp: reqwest::Response) -> RegistryResult<reqwest::Response> {
    match status_error(resp.status().as_u16()) {
        Some(err) => Err(err),
        None => Ok(resp),
    }
}

/// Sort versions by semver descending (highest first).
/// Falls back to `released_at` descending for non-semver strings.
pub fn sort_versions_semver(versions: &mut [VersionInfo]) {
    versions.sort_by(|a, b| {
        let va = parse_version(&a.version);
        let vb = parse_version(&b.version);
        match (va, vb) {
            (Some(va), Some(vb)) => vb.cmp(&va),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => b.released_at.cmp(&a.released_at),
        }
    });
}

fn parse_version(v: &str) -> Option<semver::Version> {
    let v = v.strip_prefix('v').unwrap_or(v);
    semver::Version::parse(v).ok()
}

/// Determine if a version string represents a prerelease.
/// Uses semver parsing when possible, falls back to checking for `-`
/// only in the version core (before any `+` build metadata).
pub fn is_prerelease(version: &str) -> bool {
    let v = version.strip_prefix('v').unwrap_or(version);
    match semver::Version::parse(v) {
        Ok(parsed) => !parsed.pre.is_empty(),
        Err(_) => {
            let without_build = v.split('+').next().unwrap_or(v);
            without_build.contains('-')
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_sort_semver_descending() {
        let mut versions = vec![
            VersionInfo {
                version: "0.9.8".into(),
                released_at: None,
                prerelease: false,
            },
            VersionInfo {
                version: "1.0.6".into(),
                released_at: None,
                prerelease: false,
            },
            VersionInfo {
                version: "0.8.0".into(),
                released_at: None,
                prerelease: false,
            },
            VersionInfo {
                version: "1.1.0".into(),
                released_at: None,
                prerelease: false,
            },
        ];
        sort_versions_semver(&mut versions);
        let vs: Vec<&str> = versions.iter().map(|v| v.version.as_str()).collect();
        assert_eq!(vs, vec!["1.1.0", "1.0.6", "0.9.8", "0.8.0"]);
    }

    #[test]
    fn test_sort_semver_with_prerelease() {
        let mut versions = vec![
            VersionInfo {
                version: "1.0.0".into(),
                released_at: None,
                prerelease: false,
            },
            VersionInfo {
                version: "1.0.1-beta.1".into(),
                released_at: None,
                prerelease: true,
            },
            VersionInfo {
                version: "1.0.1".into(),
                released_at: None,
                prerelease: false,
            },
        ];
        sort_versions_semver(&mut versions);
        let vs: Vec<&str> = versions.iter().map(|v| v.version.as_str()).collect();
        assert_eq!(vs, vec!["1.0.1", "1.0.1-beta.1", "1.0.0"]);
    }

    #[test]
    fn test_sort_non_semver_falls_back_to_date() {
        let now = Utc::now();
        let earlier = now - chrono::Duration::hours(1);
        let mut versions = vec![
            VersionInfo {
                version: "latest".into(),
                released_at: Some(earlier),
                prerelease: false,
            },
            VersionInfo {
                version: "nightly".into(),
                released_at: Some(now),
                prerelease: false,
            },
        ];
        sort_versions_semver(&mut versions);
        assert_eq!(versions[0].version, "nightly");
        assert_eq!(versions[1].version, "latest");
    }

    #[test]
    fn test_sort_semver_before_non_semver() {
        let mut versions = vec![
            VersionInfo {
                version: "not-a-version".into(),
                released_at: None,
                prerelease: false,
            },
            VersionInfo {
                version: "1.0.0".into(),
                released_at: None,
                prerelease: false,
            },
        ];
        sort_versions_semver(&mut versions);
        assert_eq!(versions[0].version, "1.0.0");
        assert_eq!(versions[1].version, "not-a-version");
    }

    #[test]
    fn test_is_prerelease_with_build_metadata() {
        // Build metadata after '+' should NOT mark as prerelease
        assert!(!is_prerelease("1.0.6+spec-1.1.0"));
        assert!(!is_prerelease("0.8.1+commit.abc123"));
    }

    #[test]
    fn test_is_prerelease_true_cases() {
        assert!(is_prerelease("1.0.0-beta.1"));
        assert!(is_prerelease("2.0.0-rc.1"));
        assert!(is_prerelease("v1.0.0-alpha"));
    }

    #[test]
    fn test_is_prerelease_false_cases() {
        assert!(!is_prerelease("1.0.0"));
        assert!(!is_prerelease("v2.3.4"));
        assert!(!is_prerelease("0.9.8"));
    }

    #[test]
    fn test_sort_v_prefixed_versions() {
        let mut versions = vec![
            VersionInfo {
                version: "v0.1.0".into(),
                released_at: None,
                prerelease: false,
            },
            VersionInfo {
                version: "v1.0.0".into(),
                released_at: None,
                prerelease: false,
            },
            VersionInfo {
                version: "v0.9.0".into(),
                released_at: None,
                prerelease: false,
            },
        ];
        sort_versions_semver(&mut versions);
        let vs: Vec<&str> = versions.iter().map(|v| v.version.as_str()).collect();
        assert_eq!(vs, vec!["v1.0.0", "v0.9.0", "v0.1.0"]);
    }

    #[test]
    fn status_error_classifies_codes() {
        assert!(status_error(200).is_none());
        assert!(status_error(299).is_none());
        assert!(matches!(
            status_error(429),
            Some(RegistryError::RateLimited)
        ));
        assert!(matches!(status_error(503), Some(RegistryError::Parse(_))));
        assert!(matches!(status_error(404), Some(RegistryError::Parse(_))));
    }
}
