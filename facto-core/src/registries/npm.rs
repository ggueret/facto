use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

use crate::models::*;
use crate::registries::{Registry, RegistryError, RegistryResult};

pub struct Npm {
    client: reqwest::Client,
}

impl Npm {
    pub fn new(client: reqwest::Client) -> Self {
        Self { client }
    }
}

#[derive(serde::Deserialize)]
struct NpmPackageResponse {
    name: String,
    description: Option<String>,
    #[serde(rename = "dist-tags")]
    dist_tags: Option<HashMap<String, String>>,
    license: Option<serde_json::Value>,
    homepage: Option<String>,
    repository: Option<NpmRepository>,
    author: Option<NpmAuthor>,
    time: Option<HashMap<String, String>>,
    versions: Option<HashMap<String, serde_json::Value>>,
    #[serde(default)]
    keywords: Vec<String>,
}

#[derive(serde::Deserialize)]
#[serde(untagged)]
enum NpmRepository {
    Object { url: Option<String> },
    Str(String),
}

#[derive(serde::Deserialize)]
#[serde(untagged)]
enum NpmAuthor {
    Object { name: Option<String> },
    Str(String),
}

#[derive(serde::Deserialize)]
struct NpmSearchResponse {
    objects: Vec<NpmSearchObject>,
}

#[derive(serde::Deserialize)]
struct NpmSearchObject {
    package: NpmSearchPackage,
    downloads: Option<NpmDownloads>,
}

/// npm search exposes per-result download counts (monthly/weekly); use monthly.
#[derive(serde::Deserialize)]
struct NpmDownloads {
    monthly: Option<u64>,
}

#[derive(serde::Deserialize)]
struct NpmSearchPackage {
    name: String,
    description: Option<String>,
    version: Option<String>,
    #[serde(default)]
    keywords: Vec<String>,
}

impl NpmRepository {
    fn url(&self) -> Option<String> {
        match self {
            NpmRepository::Object { url } => url.clone(),
            NpmRepository::Str(s) => Some(s.clone()),
        }
    }
}

impl NpmAuthor {
    fn name(&self) -> Option<String> {
        match self {
            NpmAuthor::Object { name } => name.clone(),
            NpmAuthor::Str(s) => Some(s.clone()),
        }
    }
}

impl Registry for Npm {
    fn id(&self) -> &str {
        "npm"
    }

    fn display_name(&self) -> &str {
        "npm"
    }

    fn get_package<'a>(
        &'a self,
        name: &'a str,
    ) -> Pin<Box<dyn Future<Output = RegistryResult<PackageInfo>> + Send + 'a>> {
        Box::pin(async move {
            let url = format!("https://registry.npmjs.org/{}", urlencoding::encode(name));
            let resp = self.client.get(&url).send().await?;

            if resp.status().as_u16() == 404 {
                return Err(RegistryError::NotFound);
            }

            let data: NpmPackageResponse = crate::registries::bounded_json(resp).await?;

            let latest_version = data
                .dist_tags
                .as_ref()
                .and_then(|dt| dt.get("latest"))
                .cloned();

            let license = data.license.as_ref().and_then(|l| match l {
                serde_json::Value::String(s) => Some(s.clone()),
                serde_json::Value::Object(o) => o
                    .get("type")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                _ => None,
            });

            let repository = data.repository.as_ref().and_then(|r| r.url());
            let mut authors = Vec::new();
            if let Some(author) = &data.author
                && let Some(name) = author.name()
                && !name.is_empty()
            {
                authors.push(name);
            }

            let updated_at = data
                .time
                .as_ref()
                .and_then(|t| t.get("modified"))
                .and_then(|ts| ts.parse().ok());

            Ok(PackageInfo {
                name: data.name,
                registry: "npm".into(),
                latest_version,
                description: data.description,
                license,
                homepage: data.homepage,
                repository,
                authors,
                updated_at,
                keywords: data.keywords,
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
            let url = format!("https://registry.npmjs.org/{}", urlencoding::encode(name));
            let resp = self.client.get(&url).send().await?;

            if resp.status().as_u16() == 404 {
                return Err(RegistryError::NotFound);
            }

            let data: NpmPackageResponse = crate::registries::bounded_json(resp).await?;

            let mut versions: Vec<VersionInfo> = data
                .versions
                .unwrap_or_default()
                .keys()
                .map(|version| {
                    let released_at = data
                        .time
                        .as_ref()
                        .and_then(|t| t.get(version))
                        .and_then(|ts| ts.parse().ok());

                    let prerelease = crate::registries::is_prerelease(version);

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
        query: &'a str,
        limit: usize,
    ) -> Pin<Box<dyn Future<Output = RegistryResult<Vec<SearchResult>>> + Send + 'a>> {
        Box::pin(async move {
            let url = format!(
                "https://registry.npmjs.org/-/v1/search?text={}&size={}",
                urlencoding::encode(query),
                limit
            );
            let resp = self.client.get(&url).send().await?;

            let resp = crate::registries::ensure_status(resp)?;

            let data: NpmSearchResponse = crate::registries::bounded_json(resp).await?;

            Ok(data
                .objects
                .into_iter()
                .map(|obj| SearchResult {
                    name: obj.package.name,
                    description: obj.package.description,
                    latest_version: obj.package.version,
                    downloads: obj.downloads.and_then(|d| d.monthly),
                    keywords: obj.package.keywords,
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
        let npm = Npm::new(crate::http::default_client().unwrap());
        let pkg = npm.get_package("express").await.unwrap();
        assert_eq!(pkg.name, "express");
        assert_eq!(pkg.registry, "npm");
        assert!(pkg.latest_version.is_some());
    }

    #[tokio::test]
    #[ignore] // requires network
    async fn test_get_versions() {
        let npm = Npm::new(crate::http::default_client().unwrap());
        let versions = npm.get_versions("express").await.unwrap();
        assert!(!versions.is_empty());
        assert!(!versions[0].version.is_empty());
    }

    #[tokio::test]
    #[ignore] // requires network
    async fn test_search() {
        let npm = Npm::new(crate::http::default_client().unwrap());
        let results = npm.search("http server", 5).await.unwrap();
        assert!(!results.is_empty());
        assert!(results.len() <= 5);
    }
}
