use std::future::Future;
use std::pin::Pin;

use crate::models::*;
use crate::normalize::pypi_normalize;
use crate::registries::{Registry, RegistryError, RegistryResult};

pub struct PyPI {
    client: reqwest::Client,
}

impl PyPI {
    pub fn new(client: reqwest::Client) -> Self {
        Self { client }
    }
}

#[derive(serde::Deserialize)]
struct PyPIJsonResponse {
    info: PyPIInfo,
    releases: std::collections::HashMap<String, Vec<PyPIRelease>>,
}

#[derive(serde::Deserialize)]
struct PyPIInfo {
    name: String,
    version: Option<String>,
    summary: Option<String>,
    license: Option<String>,
    home_page: Option<String>,
    author: Option<String>,
    #[expect(
        dead_code,
        reason = "deserialized from the API response, not yet surfaced"
    )]
    author_email: Option<String>,
    project_urls: Option<std::collections::HashMap<String, String>>,
    #[serde(default)]
    classifiers: Vec<String>,
    #[serde(default)]
    keywords: Option<String>,
    requires_python: Option<String>,
}

#[derive(serde::Deserialize)]
struct PyPIRelease {
    upload_time_iso_8601: Option<String>,
}

/// PEP 440 prerelease detection.
/// Matches: .devN, .alphaN, .betaN, .rcN, aN, bN (where N is digits after version segment)
fn is_prerelease(version: &str) -> bool {
    let v = version.to_lowercase();
    // PEP 440 long forms
    if v.contains(".dev") || v.contains("alpha") || v.contains("beta") {
        return true;
    }
    // Release candidate: ensure 'rc' is preceded by a digit (avoids matching words like "source")
    if let Some(pos) = v.find("rc")
        && pos > 0
        && v.as_bytes()[pos - 1].is_ascii_digit()
    {
        return true;
    }
    // PEP 440 shorthand: 1.0a1, 1.0b2 (digit + a/b + digit)
    let bytes = v.as_bytes();
    for i in 1..bytes.len().saturating_sub(1) {
        if (bytes[i] == b'a' || bytes[i] == b'b')
            && bytes[i - 1].is_ascii_digit()
            && bytes[i + 1].is_ascii_digit()
        {
            return true;
        }
    }
    false
}

impl Registry for PyPI {
    fn id(&self) -> &str {
        "pypi"
    }

    fn display_name(&self) -> &str {
        "PyPI"
    }

    fn get_package<'a>(
        &'a self,
        name: &'a str,
    ) -> Pin<Box<dyn Future<Output = RegistryResult<PackageInfo>> + Send + 'a>> {
        Box::pin(async move {
            let name = pypi_normalize(name);
            let url = format!("https://pypi.org/pypi/{}/json", urlencoding::encode(&name));
            let resp = self.client.get(&url).send().await?;

            if resp.status().as_u16() == 404 {
                return Err(RegistryError::NotFound);
            }

            let data: PyPIJsonResponse = crate::registries::bounded_json(resp).await?;

            let repository = data
                .info
                .project_urls
                .as_ref()
                .and_then(|urls| {
                    urls.get("Repository")
                        .or_else(|| urls.get("Source"))
                        .or_else(|| urls.get("Source Code"))
                        .or_else(|| urls.get("GitHub"))
                })
                .cloned();

            let mut authors = Vec::new();
            if let Some(author) = &data.info.author
                && !author.is_empty()
            {
                authors.push(author.clone());
            }

            let updated_at = data
                .releases
                .get(data.info.version.as_deref().unwrap_or(""))
                .and_then(|files| files.first())
                .and_then(|f| f.upload_time_iso_8601.as_ref())
                .and_then(|ts| ts.parse().ok());

            let keywords: Vec<String> = data
                .info
                .keywords
                .as_deref()
                .unwrap_or("")
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();

            Ok(PackageInfo {
                name: data.info.name,
                registry: "pypi".into(),
                latest_version: data.info.version,
                description: data.info.summary,
                license: data.info.license,
                homepage: data.info.home_page,
                repository,
                authors,
                updated_at,
                keywords,
                classifiers: data.info.classifiers,
                requires_python: data.info.requires_python,
            })
        })
    }

    fn get_versions<'a>(
        &'a self,
        name: &'a str,
    ) -> Pin<Box<dyn Future<Output = RegistryResult<Vec<VersionInfo>>> + Send + 'a>> {
        Box::pin(async move {
            let name = pypi_normalize(name);
            let url = format!("https://pypi.org/pypi/{}/json", urlencoding::encode(&name));
            let resp = self.client.get(&url).send().await?;

            if resp.status().as_u16() == 404 {
                return Err(RegistryError::NotFound);
            }

            let data: PyPIJsonResponse = crate::registries::bounded_json(resp).await?;

            let mut versions: Vec<VersionInfo> = data
                .releases
                .iter()
                .map(|(version, files)| {
                    let released_at = files
                        .first()
                        .and_then(|f| f.upload_time_iso_8601.as_ref())
                        .and_then(|ts| ts.parse().ok());

                    let prerelease = is_prerelease(version);

                    VersionInfo {
                        version: version.clone(),
                        released_at,
                        prerelease,
                    }
                })
                .collect();

            crate::registries::sort_versions_semver(&mut versions);
            Ok(versions)
        })
    }

    fn search<'a>(
        &'a self,
        _query: &'a str,
        _limit: usize,
    ) -> Pin<Box<dyn Future<Output = RegistryResult<Vec<SearchResult>>> + Send + 'a>> {
        // PyPI exposes no search API. The legacy XML-RPC `search` was
        // permanently disabled, and the HTML search page is now served behind a
        // bot challenge (HTTP 200 with no result markup), so scraping yields
        // nothing. Returning an empty Vec would masquerade as "no matches";
        // report search as unsupported instead of failing silently.
        Box::pin(async { Err(RegistryError::NotSupported) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_prerelease_detects_dev() {
        assert!(is_prerelease("1.0.0.dev1"));
        assert!(is_prerelease("1.0.dev0"));
    }

    #[test]
    fn test_is_prerelease_detects_rc() {
        assert!(is_prerelease("1.0.0rc1"));
        assert!(is_prerelease("2.0rc2"));
    }

    #[test]
    fn test_is_prerelease_detects_alpha_beta() {
        assert!(is_prerelease("1.0.0alpha1"));
        assert!(is_prerelease("1.0.0beta2"));
        assert!(is_prerelease("1.0a1"));
        assert!(is_prerelease("1.0b2"));
    }

    #[test]
    fn test_is_prerelease_rejects_stable() {
        assert!(!is_prerelease("1.0.0"));
        assert!(!is_prerelease("2.31.0"));
        assert!(!is_prerelease("0.1.0"));
        assert!(!is_prerelease("10.0"));
    }

    #[test]
    fn test_is_prerelease_rejects_false_positives() {
        assert!(!is_prerelease("1.0.0"));
        assert!(!is_prerelease("3.2.1"));
        assert!(!is_prerelease("1.0.0.post1"));
    }

    #[tokio::test]
    #[ignore] // requires network
    async fn test_get_package() {
        let pypi = PyPI::new(crate::http::default_client().unwrap());
        let pkg = pypi.get_package("requests").await.unwrap();
        assert_eq!(pkg.name, "requests");
        assert_eq!(pkg.registry, "pypi");
        assert!(pkg.latest_version.is_some());
    }

    #[tokio::test]
    #[ignore] // requires network
    async fn test_get_versions() {
        let pypi = PyPI::new(crate::http::default_client().unwrap());
        let versions = pypi.get_versions("requests").await.unwrap();
        assert!(!versions.is_empty());
        assert!(!versions[0].version.is_empty());
    }

    #[tokio::test]
    async fn test_search_unsupported() {
        // PyPI has no search API; search must report NotSupported rather than
        // silently returning an empty result set.
        let pypi = PyPI::new(crate::http::default_client().unwrap());
        let err = pypi.search("http client", 5).await.unwrap_err();
        assert!(matches!(err, RegistryError::NotSupported));
    }
}
