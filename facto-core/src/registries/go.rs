use std::future::Future;
use std::pin::Pin;

use crate::models::*;
use crate::registries::{Registry, RegistryError, RegistryResult};

pub struct GoModules {
    client: reqwest::Client,
}

impl GoModules {
    pub fn new(client: reqwest::Client) -> Self {
        Self { client }
    }
}

#[derive(serde::Deserialize)]
struct GoLatestResponse {
    #[serde(rename = "Version")]
    version: Option<String>,
    #[serde(rename = "Time")]
    time: Option<String>,
}

impl Registry for GoModules {
    fn id(&self) -> &str {
        "go"
    }

    fn display_name(&self) -> &str {
        "Go Modules"
    }

    fn get_package<'a>(
        &'a self,
        name: &'a str,
    ) -> Pin<Box<dyn Future<Output = RegistryResult<PackageInfo>> + Send + 'a>> {
        Box::pin(async move {
            let url = format!(
                "https://proxy.golang.org/{}/@latest",
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

            let data: GoLatestResponse = crate::registries::bounded_json(resp).await?;
            let updated_at = data.time.as_ref().and_then(|ts| ts.parse().ok());

            Ok(PackageInfo {
                name: name.to_string(),
                registry: "go".into(),
                latest_version: data.version,
                description: None,
                license: None,
                homepage: None,
                repository: Some(format!("https://{}", name)),
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
            let url = format!(
                "https://proxy.golang.org/{}/@v/list",
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

            let text = crate::registries::bounded_text(resp).await?;
            let mut versions: Vec<VersionInfo> = text
                .lines()
                .filter(|l| !l.is_empty())
                .map(|version| VersionInfo {
                    version: version.to_string(),
                    released_at: None,
                    prerelease: crate::registries::is_prerelease(version),
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
        Box::pin(async {
            // Go module proxy has no search API.
            Err(RegistryError::NotSupported)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // requires network
    async fn test_get_package() {
        let go = GoModules::new(crate::http::default_client().unwrap());
        let pkg = go.get_package("golang.org/x/net").await.unwrap();
        assert_eq!(pkg.registry, "go");
        assert!(pkg.latest_version.is_some());
    }
}
