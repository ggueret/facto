use std::future::Future;
use std::pin::Pin;

use crate::models::*;
use crate::registries::{Registry, RegistryError, RegistryResult};

pub struct Packagist {
    client: reqwest::Client,
}

impl Packagist {
    pub fn new(client: reqwest::Client) -> Self {
        Self { client }
    }
}

#[derive(serde::Deserialize)]
struct PackagistSearchResponse {
    results: Vec<PackagistSearchItem>,
}

#[derive(serde::Deserialize)]
struct PackagistSearchItem {
    name: String,
    description: Option<String>,
    #[serde(default)]
    downloads: u64,
}

impl Registry for Packagist {
    fn id(&self) -> &str {
        "packagist"
    }

    fn display_name(&self) -> &str {
        "Packagist"
    }

    fn get_package<'a>(
        &'a self,
        name: &'a str,
    ) -> Pin<Box<dyn Future<Output = RegistryResult<PackageInfo>> + Send + 'a>> {
        Box::pin(async move {
            let url = format!(
                "https://repo.packagist.org/p2/{}.json",
                urlencoding::encode(name)
            );
            let resp = self.client.get(&url).send().await?;

            if resp.status().as_u16() == 404 {
                return Err(RegistryError::NotFound);
            }

            let data: serde_json::Value = crate::registries::bounded_json(resp).await?;

            let versions = data["packages"][name]
                .as_array()
                .ok_or_else(|| RegistryError::Parse("invalid response".into()))?;

            let latest = versions.first();

            let keywords: Vec<String> = latest
                .and_then(|v| v["keywords"].as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();

            Ok(PackageInfo {
                name: name.to_string(),
                registry: "packagist".into(),
                latest_version: latest
                    .and_then(|v| v["version"].as_str())
                    .map(|s| s.to_string()),
                description: latest
                    .and_then(|v| v["description"].as_str())
                    .map(|s| s.to_string()),
                license: latest
                    .and_then(|v| v["license"].as_array())
                    .and_then(|l| l.first())
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                homepage: latest
                    .and_then(|v| v["homepage"].as_str())
                    .map(|s| s.to_string()),
                repository: latest
                    .and_then(|v| v["source"]["url"].as_str())
                    .map(|s| s.to_string()),
                authors: Vec::new(),
                updated_at: None,
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
            let url = format!(
                "https://repo.packagist.org/p2/{}.json",
                urlencoding::encode(name)
            );
            let resp = self.client.get(&url).send().await?;

            if resp.status().as_u16() == 404 {
                return Err(RegistryError::NotFound);
            }

            let data: serde_json::Value = crate::registries::bounded_json(resp).await?;

            let versions = data["packages"][name]
                .as_array()
                .ok_or_else(|| RegistryError::Parse("invalid response".into()))?;

            let mut result: Vec<VersionInfo> = versions
                .iter()
                .filter_map(|v| {
                    let version = v["version"].as_str()?.to_string();
                    let prerelease = version.contains("-dev")
                        || version.contains("-alpha")
                        || version.contains("-beta")
                        || version.contains("-RC");
                    Some(VersionInfo {
                        version,
                        released_at: None,
                        prerelease,
                    })
                })
                .collect();

            crate::registries::sort_versions_semver(&mut result);
            Ok(result)
        })
    }

    fn search<'a>(
        &'a self,
        query: &'a str,
        limit: usize,
    ) -> Pin<Box<dyn Future<Output = RegistryResult<Vec<SearchResult>>> + Send + 'a>> {
        Box::pin(async move {
            let url = format!(
                "https://packagist.org/search.json?q={}&per_page={}",
                urlencoding::encode(query),
                limit
            );
            let resp = self.client.get(&url).send().await?;

            let resp = crate::registries::ensure_status(resp)?;

            let data: PackagistSearchResponse = crate::registries::bounded_json(resp).await?;

            Ok(data
                .results
                .into_iter()
                .map(|item| SearchResult {
                    name: item.name,
                    description: item.description,
                    latest_version: None,
                    downloads: Some(item.downloads),
                    keywords: Vec::new(),
                })
                .collect())
        })
    }
}
