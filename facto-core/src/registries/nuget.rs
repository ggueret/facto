use std::future::Future;
use std::pin::Pin;

use crate::models::*;
use crate::registries::{Registry, RegistryError, RegistryResult};

pub struct NuGet {
    client: reqwest::Client,
}

impl NuGet {
    pub fn new(client: reqwest::Client) -> Self {
        Self { client }
    }
}

#[derive(serde::Deserialize)]
struct NuGetVersionIndex {
    versions: Vec<String>,
}

#[derive(serde::Deserialize, Default)]
struct NuGetSearchResponse {
    #[serde(default)]
    data: Vec<NuGetSearchItem>,
}

#[derive(serde::Deserialize)]
struct NuGetSearchItem {
    id: String,
    version: Option<String>,
    description: Option<String>,
    authors: Option<Vec<String>>,
    #[serde(rename = "projectUrl")]
    project_url: Option<String>,
    #[serde(rename = "licenseUrl")]
    #[expect(
        dead_code,
        reason = "deserialized from the API response, not yet surfaced"
    )]
    license_url: Option<String>,
    #[serde(default)]
    #[expect(
        dead_code,
        reason = "deserialized from the API response, not yet surfaced"
    )]
    verified: bool,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(rename = "totalDownloads", default)]
    total_downloads: u64,
}

impl Registry for NuGet {
    fn id(&self) -> &str {
        "nuget"
    }

    fn display_name(&self) -> &str {
        "NuGet"
    }

    fn get_package<'a>(
        &'a self,
        name: &'a str,
    ) -> Pin<Box<dyn Future<Output = RegistryResult<PackageInfo>> + Send + 'a>> {
        Box::pin(async move {
            let url = format!(
                "https://api.nuget.org/v3/registration5-gz-semver2/{}/index.json",
                urlencoding::encode(&name.to_lowercase())
            );
            let resp = self.client.get(&url).send().await?;

            if resp.status().as_u16() == 404 {
                return Err(RegistryError::NotFound);
            }

            // Use search API for richer metadata
            let search_url = format!(
                "https://azuresearch-usnc.nuget.org/query?q=packageid:{}&take=1",
                urlencoding::encode(name)
            );
            let search_resp = self.client.get(&search_url).send().await?;
            let data: NuGetSearchResponse = crate::registries::bounded_json(search_resp).await?;

            let item = data
                .data
                .into_iter()
                .next()
                .ok_or(RegistryError::NotFound)?;

            Ok(PackageInfo {
                name: item.id,
                registry: "nuget".into(),
                latest_version: item.version,
                description: item.description,
                license: None,
                homepage: item.project_url,
                repository: None,
                authors: item.authors.unwrap_or_default(),
                updated_at: None,
                keywords: item.tags,
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
                "https://api.nuget.org/v3-flatcontainer/{}/index.json",
                urlencoding::encode(&name.to_lowercase())
            );
            let resp = self.client.get(&url).send().await?;

            if resp.status().as_u16() == 404 {
                return Err(RegistryError::NotFound);
            }

            let data: NuGetVersionIndex = crate::registries::bounded_json(resp).await?;

            let mut versions: Vec<VersionInfo> = data
                .versions
                .into_iter()
                .map(|v| {
                    let prerelease = crate::registries::is_prerelease(&v);
                    VersionInfo {
                        version: v,
                        released_at: None,
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
        query: &'a str,
        limit: usize,
    ) -> Pin<Box<dyn Future<Output = RegistryResult<Vec<SearchResult>>> + Send + 'a>> {
        Box::pin(async move {
            let url = format!(
                "https://azuresearch-usnc.nuget.org/query?q={}&take={}",
                urlencoding::encode(query),
                limit
            );
            let resp = self.client.get(&url).send().await?;

            let resp = crate::registries::ensure_status(resp)?;

            let data: NuGetSearchResponse = crate::registries::bounded_json(resp).await?;

            Ok(data
                .data
                .into_iter()
                .map(|item| SearchResult {
                    name: item.id,
                    description: item.description,
                    latest_version: item.version,
                    downloads: Some(item.total_downloads),
                    keywords: item.tags,
                })
                .collect())
        })
    }
}
