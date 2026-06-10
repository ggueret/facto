use std::future::Future;
use std::pin::Pin;

use crate::models::*;
use crate::registries::{Registry, RegistryError, RegistryResult};

pub struct Maven {
    client: reqwest::Client,
}

impl Maven {
    pub fn new(client: reqwest::Client) -> Self {
        Self { client }
    }
}

#[derive(serde::Deserialize)]
struct MavenSearchResponse {
    response: MavenResponse,
}

#[derive(serde::Deserialize)]
struct MavenResponse {
    #[serde(rename = "numFound")]
    num_found: u64,
    docs: Vec<MavenDoc>,
}

#[derive(serde::Deserialize)]
struct MavenDoc {
    id: Option<String>,
    g: Option<String>,
    a: Option<String>,
    #[serde(rename = "latestVersion")]
    latest_version: Option<String>,
    #[serde(rename = "versionCount")]
    #[expect(
        dead_code,
        reason = "deserialized from the API response, not yet surfaced"
    )]
    version_count: Option<u64>,
    timestamp: Option<i64>,
}

impl Registry for Maven {
    fn id(&self) -> &str {
        "maven"
    }

    fn display_name(&self) -> &str {
        "Maven Central"
    }

    fn get_package<'a>(
        &'a self,
        name: &'a str,
    ) -> Pin<Box<dyn Future<Output = RegistryResult<PackageInfo>> + Send + 'a>> {
        Box::pin(async move {
            let url = format!(
                "https://search.maven.org/solrsearch/select?q=a:{}&rows=1&wt=json",
                urlencoding::encode(name)
            );
            let resp = self.client.get(&url).send().await?;

            if !resp.status().is_success() {
                return Err(RegistryError::Parse("search failed".into()));
            }

            let data: MavenSearchResponse = crate::registries::bounded_json(resp).await?;

            let doc = data
                .response
                .docs
                .into_iter()
                .next()
                .ok_or(RegistryError::NotFound)?;

            let updated_at = doc.timestamp.and_then(|ts| {
                chrono::DateTime::from_timestamp_millis(ts).map(|dt| dt.with_timezone(&chrono::Utc))
            });

            Ok(PackageInfo {
                name: doc.a.unwrap_or_else(|| name.to_string()),
                registry: "maven".into(),
                latest_version: doc.latest_version,
                description: None,
                license: None,
                homepage: doc.id.map(|id| {
                    format!("https://search.maven.org/artifact/{}", id.replace(':', "/"))
                }),
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
            let rows: u64 = 50;
            let max_versions = 1000;
            let mut start: u64 = 0;

            loop {
                let url = format!(
                    "https://search.maven.org/solrsearch/select?q=a:{}&core=gav&rows={}&start={}&wt=json",
                    urlencoding::encode(name),
                    rows,
                    start
                );
                let resp = self.client.get(&url).send().await?;

                if !resp.status().is_success() {
                    return Err(RegistryError::Parse("search failed".into()));
                }

                let data: MavenSearchResponse = crate::registries::bounded_json(resp).await?;
                let num_found = data.response.num_found;

                for doc in data.response.docs {
                    if let Some(version) = doc.latest_version {
                        let released_at = doc.timestamp.and_then(|ts| {
                            chrono::DateTime::from_timestamp_millis(ts)
                                .map(|dt| dt.with_timezone(&chrono::Utc))
                        });
                        let prerelease = version.contains("-SNAPSHOT")
                            || version.contains("-RC")
                            || version.contains("-alpha")
                            || version.contains("-beta");
                        all_versions.push(VersionInfo {
                            version,
                            released_at,
                            prerelease,
                        });
                    }
                }

                start += rows;
                if start >= num_found || all_versions.len() >= max_versions {
                    break;
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
                "https://search.maven.org/solrsearch/select?q={}&rows={}&wt=json",
                urlencoding::encode(query),
                limit
            );
            let resp = self.client.get(&url).send().await?;

            let resp = crate::registries::ensure_status(resp)?;

            let data: MavenSearchResponse = crate::registries::bounded_json(resp).await?;

            Ok(data
                .response
                .docs
                .into_iter()
                .filter_map(|doc| {
                    let name = doc.a?;
                    Some(SearchResult {
                        name,
                        description: doc.g,
                        latest_version: doc.latest_version,
                        downloads: None,
                        keywords: Vec::new(),
                    })
                })
                .collect())
        })
    }
}
