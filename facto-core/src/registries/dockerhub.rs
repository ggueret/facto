use std::future::Future;
use std::pin::Pin;

use crate::models::*;
use crate::registries::{Registry, RegistryError, RegistryResult};

pub struct DockerHub {
    client: reqwest::Client,
}

impl DockerHub {
    pub fn new(client: reqwest::Client) -> Self {
        Self { client }
    }
}

#[derive(serde::Deserialize)]
struct DockerHubRepo {
    name: String,
    description: Option<String>,
    #[expect(
        dead_code,
        reason = "deserialized from the API response, not yet surfaced"
    )]
    star_count: Option<u64>,
    last_updated: Option<String>,
}

#[derive(serde::Deserialize)]
struct DockerHubSearchResponse {
    results: Vec<DockerHubSearchItem>,
}

#[derive(serde::Deserialize)]
struct DockerHubSearchItem {
    repo_name: Option<String>,
    short_description: Option<String>,
    name: Option<String>,
    #[serde(default)]
    pull_count: u64,
}

#[derive(serde::Deserialize)]
struct DockerHubTagsResponse {
    results: Vec<DockerHubTag>,
    next: Option<String>,
}

#[derive(serde::Deserialize)]
struct DockerHubTag {
    name: String,
    last_updated: Option<String>,
}

/// Heuristic prerelease detection for Docker tags.
///
/// Docker tags are free-form strings, not semver, so the shared semver-based
/// `is_prerelease` does not apply (its `-` fallback would flag every distro
/// variant like `17.5-trixie`). A tag is a prerelease only when one of its
/// alphabetic segments is a recognised marker, so `alpine` is not `alpha` and
/// `arch` is not `rc`.
fn tag_is_prerelease(tag: &str) -> bool {
    const MARKERS: &[&str] = &[
        "alpha", "beta", "rc", "dev", "preview", "snapshot", "nightly", "canary",
    ];
    tag.to_ascii_lowercase()
        .split(|c: char| !c.is_ascii_alphabetic())
        .any(|segment| MARKERS.contains(&segment))
}

impl Registry for DockerHub {
    fn id(&self) -> &str {
        "dockerhub"
    }

    fn display_name(&self) -> &str {
        "Docker Hub"
    }

    /// Docker tags are not semver and are ordered by push date, so a single
    /// "latest" is not meaningful. `list_versions` still works.
    fn supports_latest_version(&self) -> bool {
        false
    }

    fn get_package<'a>(
        &'a self,
        name: &'a str,
    ) -> Pin<Box<dyn Future<Output = RegistryResult<PackageInfo>> + Send + 'a>> {
        Box::pin(async move {
            let url = format!(
                "https://hub.docker.com/v2/repositories/library/{}/",
                urlencoding::encode(name)
            );
            let resp = self.client.get(&url).send().await?;

            match resp.status().as_u16() {
                404 | 410 => return Err(RegistryError::NotFound),
                429 => return Err(RegistryError::RateLimited),
                code if !(200..300).contains(&code) => {
                    return Err(RegistryError::Parse(format!("unexpected status {code}")));
                }
                _ => {}
            }

            let data: DockerHubRepo = crate::registries::bounded_json(resp).await?;
            let updated_at = data.last_updated.as_ref().and_then(|ts| ts.parse().ok());

            Ok(PackageInfo {
                name: data.name,
                registry: "dockerhub".into(),
                latest_version: Some("latest".into()),
                description: data.description,
                license: None,
                homepage: Some(format!(
                    "https://hub.docker.com/_/{}",
                    urlencoding::encode(name)
                )),
                repository: None,
                authors: Vec::new(),
                updated_at,
                keywords: Vec::new(),
                classifiers: Vec::new(),
                requires_python: None,
            })
        })
    }

    fn get_versions<'a>(
        &'a self,
        name: &'a str,
    ) -> Pin<Box<dyn Future<Output = RegistryResult<Vec<VersionInfo>>> + Send + 'a>> {
        Box::pin(async move {
            let mut all_versions = Vec::new();
            let max_tags = 1000;
            let mut url = format!(
                "https://hub.docker.com/v2/repositories/library/{}/tags/?page_size=100&ordering=last_updated",
                urlencoding::encode(name)
            );

            loop {
                let resp = self.client.get(&url).send().await?;

                match resp.status().as_u16() {
                    404 | 410 => return Err(RegistryError::NotFound),
                    429 => return Err(RegistryError::RateLimited),
                    code if !(200..300).contains(&code) => {
                        return Err(RegistryError::Parse(format!("unexpected status {code}")));
                    }
                    _ => {}
                }

                let data: DockerHubTagsResponse = crate::registries::bounded_json(resp).await?;

                for tag in data.results {
                    let released_at = tag.last_updated.as_ref().and_then(|ts| ts.parse().ok());
                    let prerelease = tag_is_prerelease(&tag.name);
                    all_versions.push(VersionInfo {
                        version: tag.name,
                        released_at,
                        prerelease,
                    });
                }

                match data.next {
                    Some(next_url) if all_versions.len() < max_tags => url = next_url,
                    _ => break,
                }
            }

            // Docker tags are not semver — keep date ordering
            Ok(all_versions)
        })
    }

    fn search<'a>(
        &'a self,
        query: &'a str,
        limit: usize,
    ) -> Pin<Box<dyn Future<Output = RegistryResult<Vec<SearchResult>>> + Send + 'a>> {
        Box::pin(async move {
            let url = format!(
                "https://hub.docker.com/v2/search/repositories/?query={}&page_size={}",
                urlencoding::encode(query),
                limit
            );
            let resp = self.client.get(&url).send().await?;

            let resp = crate::registries::ensure_status(resp)?;

            let data: DockerHubSearchResponse = crate::registries::bounded_json(resp).await?;

            Ok(data
                .results
                .into_iter()
                .map(|item| SearchResult {
                    name: item.repo_name.or(item.name).unwrap_or_default(),
                    description: item.short_description,
                    latest_version: None,
                    downloads: Some(item.pull_count),
                    keywords: Vec::new(),
                })
                .collect())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::tag_is_prerelease;

    #[test]
    fn flags_prerelease_tags() {
        assert!(tag_is_prerelease("19beta1-trixie"));
        assert!(tag_is_prerelease("16rc1"));
        assert!(tag_is_prerelease("1.0-alpha"));
        assert!(tag_is_prerelease("17-dev"));
    }

    #[test]
    fn keeps_stable_and_variant_tags() {
        assert!(!tag_is_prerelease("17.5"));
        assert!(!tag_is_prerelease("17.5-trixie"));
        assert!(!tag_is_prerelease("17-alpine"));
        assert!(!tag_is_prerelease("latest"));
        assert!(!tag_is_prerelease("arch"));
        assert!(!tag_is_prerelease("alphabet"));
    }

    #[test]
    fn does_not_support_latest_version() {
        use crate::registries::Registry;
        let dh = super::DockerHub::new(reqwest::Client::new());
        assert!(!dh.supports_latest_version());
    }
}
