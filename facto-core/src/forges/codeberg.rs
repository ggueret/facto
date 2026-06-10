use std::future::Future;
use std::pin::Pin;

use crate::forges::{Forge, ForgeError, ForgeResult};
use crate::models::*;

pub struct Codeberg {
    client: reqwest::Client,
}

impl Codeberg {
    pub fn new(client: reqwest::Client) -> Self {
        Self { client }
    }
}

#[derive(serde::Deserialize)]
struct CodebergRelease {
    tag_name: String,
    name: Option<String>,
    published_at: Option<String>,
    prerelease: bool,
    draft: bool,
    html_url: Option<String>,
    assets: Vec<CodebergAsset>,
}

#[derive(serde::Deserialize)]
struct CodebergAsset {
    name: String,
    size: u64,
    browser_download_url: String,
    download_count: u64,
    #[serde(default)]
    content_type: String,
}

#[derive(serde::Deserialize)]
struct CodebergSearchResponse {
    data: Vec<CodebergRepo>,
}

#[derive(serde::Deserialize)]
struct CodebergRepo {
    full_name: String,
    name: String,
    owner: CodebergRepoOwner,
    description: Option<String>,
    stars_count: u64,
    language: Option<String>,
    #[serde(default)]
    topics: Vec<String>,
    html_url: String,
    updated_at: Option<String>,
}

#[derive(serde::Deserialize)]
struct CodebergRepoOwner {
    login: String,
}

impl Forge for Codeberg {
    fn id(&self) -> &str {
        "codeberg"
    }

    fn display_name(&self) -> &str {
        "Codeberg"
    }

    fn supports_releases(&self) -> bool {
        true
    }

    fn list_releases<'a>(
        &'a self,
        owner: &'a str,
        repo: &'a str,
        limit: usize,
    ) -> Pin<Box<dyn Future<Output = ForgeResult<Vec<ReleaseInfo>>> + Send + 'a>> {
        Box::pin(async move {
            let url = format!(
                "https://codeberg.org/api/v1/repos/{}/{}/releases?limit={}",
                urlencoding::encode(owner),
                urlencoding::encode(repo),
                limit
            );
            let resp = self.client.get(&url).send().await?;

            match resp.status().as_u16() {
                200 => {
                    let releases: Vec<CodebergRelease> = crate::forges::bounded_json(resp).await?;
                    Ok(releases
                        .into_iter()
                        .map(|r| ReleaseInfo {
                            tag: r.tag_name,
                            name: r.name,
                            published_at: r.published_at.as_deref().and_then(|ts| ts.parse().ok()),
                            prerelease: r.prerelease,
                            draft: r.draft,
                            html_url: r.html_url.unwrap_or_default(),
                            assets: r
                                .assets
                                .into_iter()
                                .map(|a| ReleaseAsset {
                                    name: a.name,
                                    size: a.size,
                                    download_url: a.browser_download_url,
                                    download_count: a.download_count,
                                    content_type: a.content_type,
                                })
                                .collect(),
                        })
                        .collect())
                }
                404 => Ok(Vec::new()),
                429 => Err(ForgeError::RateLimited),
                status => Err(ForgeError::Parse(format!("unexpected status {}", status))),
            }
        })
    }

    fn supports_search(&self) -> bool {
        true
    }

    fn search_repositories<'a>(
        &'a self,
        query: &'a str,
        sort: RepoSort,
        _language: Option<&'a str>,
        limit: usize,
    ) -> Pin<Box<dyn Future<Output = ForgeResult<Vec<RepositorySearchResult>>> + Send + 'a>> {
        Box::pin(async move {
            let mut url = format!(
                "https://codeberg.org/api/v1/repos/search?q={}&limit={}",
                urlencoding::encode(query),
                limit.min(50),
            );
            // Forgejo (Codeberg) honours sort=stars/updated; verified live.
            match sort {
                RepoSort::Stars => url.push_str("&sort=stars&order=desc"),
                RepoSort::Updated => url.push_str("&sort=updated&order=desc"),
                RepoSort::Relevance => {}
            }
            let resp = self.client.get(&url).send().await?;
            match resp.status().as_u16() {
                200 => {}
                429 => return Err(ForgeError::RateLimited),
                status => return Err(ForgeError::Parse(format!("unexpected status {status}"))),
            }
            let data: CodebergSearchResponse = crate::forges::bounded_json(resp).await?;
            Ok(data
                .data
                .into_iter()
                .map(|r| RepositorySearchResult {
                    forge: "codeberg".into(),
                    full_name: r.full_name,
                    owner: r.owner.login,
                    name: r.name,
                    description: r.description,
                    stars: r.stars_count,
                    language: r.language,
                    topics: r.topics,
                    url: r.html_url,
                    updated_at: r.updated_at.as_deref().and_then(|ts| ts.parse().ok()),
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
    async fn test_codeberg_list_releases() {
        let cb = Codeberg::new(crate::http::default_client().unwrap());
        let releases = cb.list_releases("forgejo", "forgejo", 5).await.unwrap();
        assert!(!releases.is_empty());
        assert!(releases.len() <= 5);
    }

    #[tokio::test]
    #[ignore] // requires network
    async fn test_search_repositories() {
        let cb = Codeberg::new(crate::http::default_client().unwrap());
        let results = cb
            .search_repositories("forgejo", RepoSort::Stars, None, 5)
            .await
            .unwrap();
        assert!(!results.is_empty());
        assert!(results.len() <= 5);
        assert_eq!(results[0].forge, "codeberg");
    }
}
