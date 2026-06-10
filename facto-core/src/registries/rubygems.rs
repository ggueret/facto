use std::future::Future;
use std::pin::Pin;

use crate::models::*;
use crate::registries::{Registry, RegistryError, RegistryResult};

pub struct RubyGems {
    client: reqwest::Client,
}

impl RubyGems {
    pub fn new(client: reqwest::Client) -> Self {
        Self { client }
    }
}

#[derive(serde::Deserialize)]
struct GemInfo {
    name: String,
    info: Option<String>,
    version: Option<String>,
    licenses: Option<Vec<String>>,
    homepage_uri: Option<String>,
    source_code_uri: Option<String>,
    authors: Option<String>,
}

#[derive(serde::Deserialize)]
struct GemVersion {
    number: String,
    created_at: Option<String>,
    prerelease: bool,
}

#[derive(serde::Deserialize)]
struct GemSearchItem {
    name: String,
    info: Option<String>,
    version: Option<String>,
    #[serde(default)]
    downloads: u64,
}

impl Registry for RubyGems {
    fn id(&self) -> &str {
        "rubygems"
    }

    fn display_name(&self) -> &str {
        "RubyGems"
    }

    fn get_package<'a>(
        &'a self,
        name: &'a str,
    ) -> Pin<Box<dyn Future<Output = RegistryResult<PackageInfo>> + Send + 'a>> {
        Box::pin(async move {
            let url = format!(
                "https://rubygems.org/api/v1/gems/{}.json",
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

            let data: GemInfo = crate::registries::bounded_json(resp).await?;

            let authors = data
                .authors
                .map(|a| a.split(',').map(|s| s.trim().to_string()).collect())
                .unwrap_or_default();

            Ok(PackageInfo {
                name: data.name,
                registry: "rubygems".into(),
                latest_version: data.version,
                description: data.info,
                license: data.licenses.and_then(|l| l.into_iter().next()),
                homepage: data.homepage_uri,
                repository: data.source_code_uri,
                authors,
                updated_at: None,
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
            let url = format!(
                "https://rubygems.org/api/v1/versions/{}.json",
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

            let data: Vec<GemVersion> = crate::registries::bounded_json(resp).await?;

            let mut versions: Vec<VersionInfo> = data
                .into_iter()
                .map(|v| {
                    let released_at = v.created_at.as_ref().and_then(|ts| ts.parse().ok());
                    VersionInfo {
                        version: v.number,
                        released_at,
                        prerelease: v.prerelease,
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
                "https://rubygems.org/api/v1/search.json?query={}&page=1",
                urlencoding::encode(query)
            );
            let resp = self.client.get(&url).send().await?;

            let resp = crate::registries::ensure_status(resp)?;

            let data: Vec<GemSearchItem> = crate::registries::bounded_json(resp).await?;

            Ok(data
                .into_iter()
                .take(limit)
                .map(|g| SearchResult {
                    name: g.name,
                    description: g.info,
                    latest_version: g.version,
                    downloads: Some(g.downloads),
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
        let rb = RubyGems::new(crate::http::default_client().unwrap());
        let pkg = rb.get_package("rails").await.unwrap();
        assert_eq!(pkg.name, "rails");
        assert_eq!(pkg.registry, "rubygems");
    }
}
