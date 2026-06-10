use std::future::Future;
use std::pin::Pin;

use crate::forges::{Forge, ForgeError, ForgeResult};
use crate::models::{ReleaseAsset, ReleaseInfo, RepoSort, RepositorySearchResult, TagPin};

pub struct GitHub {
    client: reqwest::Client,
}

impl GitHub {
    pub fn new(client: reqwest::Client) -> Self {
        Self { client }
    }
}

#[derive(serde::Deserialize)]
struct GitHubRelease {
    tag_name: String,
    name: Option<String>,
    published_at: Option<String>,
    prerelease: bool,
    draft: bool,
    html_url: String,
    assets: Vec<GitHubAsset>,
}

#[derive(serde::Deserialize)]
struct GitHubAsset {
    name: String,
    size: u64,
    browser_download_url: String,
    download_count: u64,
    content_type: String,
}

#[derive(serde::Deserialize)]
struct GitHubSearchResponse {
    items: Vec<GitHubRepo>,
}

#[derive(serde::Deserialize)]
struct GitHubRepo {
    full_name: String,
    name: String,
    owner: GitHubRepoOwner,
    description: Option<String>,
    stargazers_count: u64,
    language: Option<String>,
    #[serde(default)]
    topics: Vec<String>,
    html_url: String,
    updated_at: Option<String>,
}

#[derive(serde::Deserialize)]
struct GitHubRepoOwner {
    login: String,
}

impl Forge for GitHub {
    fn id(&self) -> &str {
        "github"
    }

    fn display_name(&self) -> &str {
        "GitHub"
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
                "https://api.github.com/repos/{}/{}/releases?per_page={}",
                urlencoding::encode(owner),
                urlencoding::encode(repo),
                limit
            );
            let resp = self.client.get(&url).send().await?;

            match resp.status().as_u16() {
                200 => {
                    let releases: Vec<GitHubRelease> = crate::forges::bounded_json(resp).await?;
                    Ok(releases
                        .into_iter()
                        .map(|r| ReleaseInfo {
                            tag: r.tag_name,
                            name: r.name,
                            published_at: r.published_at.as_deref().and_then(|ts| ts.parse().ok()),
                            prerelease: r.prerelease,
                            draft: r.draft,
                            html_url: r.html_url,
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
        language: Option<&'a str>,
        limit: usize,
    ) -> Pin<Box<dyn Future<Output = ForgeResult<Vec<RepositorySearchResult>>> + Send + 'a>> {
        Box::pin(async move {
            let mut q = query.to_string();
            if let Some(lang) = language {
                q.push_str(&format!(" language:{lang}"));
            }
            let mut url = format!(
                "https://api.github.com/search/repositories?q={}&per_page={}",
                urlencoding::encode(&q),
                limit.min(100),
            );
            match sort {
                RepoSort::Stars => url.push_str("&sort=stars&order=desc"),
                RepoSort::Updated => url.push_str("&sort=updated&order=desc"),
                RepoSort::Relevance => {}
            }
            let resp = self.client.get(&url).send().await?;
            match resp.status().as_u16() {
                200 => {}
                403 | 429 => return Err(ForgeError::RateLimited),
                status => return Err(ForgeError::Parse(format!("unexpected status {status}"))),
            }
            let data: GitHubSearchResponse = crate::forges::bounded_json(resp).await?;
            Ok(data
                .items
                .into_iter()
                .map(|r| RepositorySearchResult {
                    forge: "github".into(),
                    full_name: r.full_name,
                    owner: r.owner.login,
                    name: r.name,
                    description: r.description,
                    stars: r.stargazers_count,
                    language: r.language,
                    topics: r.topics,
                    url: r.html_url,
                    updated_at: r.updated_at.as_deref().and_then(|ts| ts.parse().ok()),
                })
                .collect())
        })
    }

    fn resolve_tag<'a>(
        &'a self,
        owner: &'a str,
        repo: &'a str,
        tag: &'a str,
    ) -> Pin<Box<dyn Future<Output = ForgeResult<TagPin>> + Send + 'a>> {
        Box::pin(async move {
            let url = format!(
                "https://api.github.com/repos/{}/{}/commits/{}",
                urlencoding::encode(owner),
                urlencoding::encode(repo),
                urlencoding::encode(tag),
            );
            let resp = self.client.get(&url).send().await?;

            match resp.status().as_u16() {
                200 => {
                    let data: serde_json::Value = crate::forges::bounded_json(resp).await?;
                    let sha = data["sha"]
                        .as_str()
                        .ok_or_else(|| ForgeError::Parse("missing sha in response".into()))?
                        .to_string();
                    Ok(TagPin {
                        owner: owner.to_string(),
                        repository: repo.to_string(),
                        forge_id: "github".to_string(),
                        tag: tag.to_string(),
                        commit_sha: sha.clone(),
                        url: Some(format!(
                            "https://github.com/{}/{}/commit/{}",
                            owner,
                            repo,
                            sha.get(..7).unwrap_or(&sha)
                        )),
                    })
                }
                404 => Err(ForgeError::Parse(format!("tag '{}' not found", tag))),
                422 => Err(ForgeError::Parse(format!("invalid ref '{}'", tag))),
                429 => Err(ForgeError::RateLimited),
                status => Err(ForgeError::Parse(format!("unexpected status {}", status))),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // requires network
    async fn test_list_releases() {
        let gh = GitHub::new(crate::http::default_client().unwrap());
        let releases = gh.list_releases("jdx", "mise", 5).await.unwrap();
        assert!(!releases.is_empty());
        assert!(releases.len() <= 5);
        assert!(!releases[0].tag.is_empty());
    }

    #[tokio::test]
    #[ignore] // requires network
    async fn test_resolve_tag() {
        let gh = GitHub::new(crate::http::default_client().unwrap());
        let pin = gh.resolve_tag("actions", "checkout", "v4").await.unwrap();
        assert_eq!(pin.owner, "actions");
        assert_eq!(pin.repository, "checkout");
        assert_eq!(pin.tag, "v4");
        assert_eq!(pin.commit_sha.len(), 40);
        assert!(pin.url.is_some());
    }

    #[tokio::test]
    #[ignore] // requires network
    async fn test_resolve_tag_not_found() {
        let gh = GitHub::new(crate::http::default_client().unwrap());
        let result = gh
            .resolve_tag("actions", "checkout", "v99999-nonexistent")
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    #[ignore] // requires network
    async fn test_search_repositories() {
        let gh = GitHub::new(crate::http::default_client().unwrap());
        let results = gh
            .search_repositories("tokio", RepoSort::Stars, None, 5)
            .await
            .unwrap();
        assert!(!results.is_empty());
        assert!(results.len() <= 5);
        assert!(results[0].stars > 0);
        assert_eq!(results[0].forge, "github");
    }
}
