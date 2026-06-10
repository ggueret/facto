use std::future::Future;
use std::pin::Pin;

use crate::models::*;
use crate::registries::{Registry, RegistryError, RegistryResult};

pub struct CratesIo {
    client: reqwest::Client,
}

impl CratesIo {
    pub fn new(client: reqwest::Client) -> Self {
        Self { client }
    }
}

#[derive(serde::Deserialize)]
struct CrateResponse {
    #[serde(rename = "crate")]
    krate: CrateInfo,
    #[serde(default)]
    keywords: Vec<CrateKeyword>,
    #[serde(default)]
    categories: Vec<CrateCategory>,
}

#[derive(serde::Deserialize)]
struct CrateKeyword {
    keyword: String,
}

#[derive(serde::Deserialize)]
struct CrateCategory {
    category: String,
}

#[derive(serde::Deserialize)]
struct CrateInfo {
    name: String,
    description: Option<String>,
    max_version: Option<String>,
    max_stable_version: Option<String>,
    homepage: Option<String>,
    repository: Option<String>,
    updated_at: Option<String>,
}

#[derive(serde::Deserialize)]
struct CrateVersion {
    num: String,
    created_at: Option<String>,
    yanked: bool,
}

#[derive(serde::Deserialize)]
struct CrateSearchResponse {
    crates: Vec<CrateSearchItem>,
}

#[derive(serde::Deserialize)]
struct CrateSearchItem {
    name: String,
    description: Option<String>,
    max_version: Option<String>,
    #[serde(default)]
    downloads: u64,
}

#[derive(serde::Deserialize)]
struct CrateVersionsResponse {
    versions: Vec<CrateVersion>,
    meta: CrateVersionsMeta,
}

#[derive(serde::Deserialize)]
struct CrateVersionsMeta {
    #[expect(
        dead_code,
        reason = "deserialized from the API response, not yet surfaced"
    )]
    total: u64,
    next_page: Option<String>,
}

impl Registry for CratesIo {
    fn id(&self) -> &str {
        "crates"
    }

    fn display_name(&self) -> &str {
        "crates.io"
    }

    fn get_package<'a>(
        &'a self,
        name: &'a str,
    ) -> Pin<Box<dyn Future<Output = RegistryResult<PackageInfo>> + Send + 'a>> {
        Box::pin(async move {
            let url = format!(
                "https://crates.io/api/v1/crates/{}",
                urlencoding::encode(name)
            );
            let resp = self.client.get(&url).send().await?;

            if resp.status().as_u16() == 404 {
                return Err(RegistryError::NotFound);
            }

            let data: CrateResponse = crate::registries::bounded_json(resp).await?;

            let latest_version = data.krate.max_stable_version.or(data.krate.max_version);

            let updated_at = data
                .krate
                .updated_at
                .as_ref()
                .and_then(|ts| ts.parse().ok());

            let mut keywords: Vec<String> = data.keywords.into_iter().map(|k| k.keyword).collect();
            let categories: Vec<String> = data.categories.into_iter().map(|c| c.category).collect();
            keywords.extend(categories);

            Ok(PackageInfo {
                name: data.krate.name,
                registry: "crates".into(),
                latest_version,
                description: data.krate.description,
                license: None,
                homepage: data.krate.homepage,
                repository: data.krate.repository,
                authors: Vec::new(),
                updated_at,
                keywords,
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
            let max_versions = 1000;
            let mut url = format!(
                "https://crates.io/api/v1/crates/{}/versions?per_page=100",
                urlencoding::encode(name)
            );

            loop {
                let resp = self.client.get(&url).send().await?;

                if resp.status().as_u16() == 404 {
                    return Err(RegistryError::NotFound);
                }

                let data: CrateVersionsResponse = crate::registries::bounded_json(resp).await?;

                for v in data.versions {
                    if v.yanked {
                        continue;
                    }
                    let released_at = v.created_at.as_ref().and_then(|ts| ts.parse().ok());
                    let prerelease = crate::registries::is_prerelease(&v.num);
                    all_versions.push(VersionInfo {
                        version: v.num,
                        released_at,
                        prerelease,
                    });
                }

                match data.meta.next_page {
                    Some(query) if all_versions.len() < max_versions => {
                        url = format!(
                            "https://crates.io/api/v1/crates/{}/versions{}",
                            urlencoding::encode(name),
                            query
                        );
                    }
                    _ => break,
                }
            }

            crate::registries::sort_versions_semver(&mut all_versions);
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
                "https://crates.io/api/v1/crates?q={}&per_page={}",
                urlencoding::encode(query),
                limit
            );
            let resp = self.client.get(&url).send().await?;

            let resp = crate::registries::ensure_status(resp)?;

            let data: CrateSearchResponse = crate::registries::bounded_json(resp).await?;

            Ok(data
                .crates
                .into_iter()
                .map(|c| SearchResult {
                    name: c.name,
                    description: c.description,
                    latest_version: c.max_version,
                    downloads: Some(c.downloads),
                    keywords: Vec::new(),
                })
                .collect())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // requires network
    async fn test_get_package() {
        let crates = CratesIo::new(crate::http::default_client().unwrap());
        let pkg = crates.get_package("serde").await.unwrap();
        assert_eq!(pkg.name, "serde");
        assert_eq!(pkg.registry, "crates");
        assert!(pkg.latest_version.is_some());
    }

    #[tokio::test]
    #[ignore] // requires network
    async fn test_get_versions() {
        let crates = CratesIo::new(crate::http::default_client().unwrap());
        let versions = crates.get_versions("serde").await.unwrap();
        assert!(!versions.is_empty());
        assert!(!versions[0].version.is_empty());
    }

    #[tokio::test]
    #[ignore] // requires network
    async fn test_search() {
        let crates = CratesIo::new(crate::http::default_client().unwrap());
        let results = crates.search("json", 5).await.unwrap();
        assert!(!results.is_empty());
        assert!(results.len() <= 5);
    }
}
