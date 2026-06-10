use std::future::Future;
use std::pin::Pin;

use crate::forges::{Forge, ForgeError, ForgeResult};
use crate::models::*;

pub struct GitLab {
    client: reqwest::Client,
    base_url: String,
    platform_id: String,
    platform_name: String,
}

impl GitLab {
    pub fn new(
        client: reqwest::Client,
        base_url: &str,
        platform_id: &str,
        platform_name: &str,
    ) -> Self {
        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            platform_id: platform_id.to_string(),
            platform_name: platform_name.to_string(),
        }
    }
}

#[derive(serde::Deserialize)]
struct GitLabRelease {
    tag_name: String,
    name: Option<String>,
    released_at: Option<String>,
    #[serde(default)]
    upcoming_release: bool,
    _links: Option<GitLabReleaseLinks>,
    assets: Option<GitLabAssets>,
}

#[derive(serde::Deserialize)]
struct GitLabReleaseLinks {
    #[serde(rename = "self")]
    self_url: Option<String>,
}

#[derive(serde::Deserialize)]
struct GitLabAssets {
    links: Option<Vec<GitLabAssetLink>>,
}

#[derive(serde::Deserialize)]
struct GitLabAssetLink {
    name: String,
    url: String,
}

#[derive(serde::Deserialize)]
struct GitLabProject {
    name: String,
    path_with_namespace: String,
    namespace: GitLabNamespace,
    description: Option<String>,
    star_count: u64,
    #[serde(default)]
    topics: Vec<String>,
    web_url: String,
    last_activity_at: Option<String>,
}

#[derive(serde::Deserialize)]
struct GitLabNamespace {
    path: String,
}

impl Forge for GitLab {
    fn id(&self) -> &str {
        &self.platform_id
    }

    fn display_name(&self) -> &str {
        &self.platform_name
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
            let project_path = format!("{}/{}", owner, repo);
            let encoded = urlencoding::encode(&project_path);
            let url = format!(
                "{}/api/v4/projects/{}/releases?per_page={}",
                self.base_url, encoded, limit
            );
            let resp = self.client.get(&url).send().await?;

            match resp.status().as_u16() {
                200 => {
                    let releases: Vec<GitLabRelease> = crate::forges::bounded_json(resp).await?;
                    Ok(releases
                        .into_iter()
                        .map(|r| {
                            let html_url = r._links.and_then(|l| l.self_url).unwrap_or_default();
                            let assets = r
                                .assets
                                .and_then(|a| a.links)
                                .unwrap_or_default()
                                .into_iter()
                                .map(|a| ReleaseAsset {
                                    name: a.name,
                                    size: 0,
                                    download_url: a.url,
                                    download_count: 0,
                                    content_type: String::new(),
                                })
                                .collect();
                            ReleaseInfo {
                                tag: r.tag_name,
                                name: r.name,
                                published_at: r
                                    .released_at
                                    .as_deref()
                                    .and_then(|ts| ts.parse().ok()),
                                prerelease: r.upcoming_release,
                                draft: false,
                                html_url,
                                assets,
                            }
                        })
                        .collect())
                }
                404 => Ok(Vec::new()),
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
            let order_by = match sort {
                RepoSort::Stars => "star_count",
                RepoSort::Updated => "last_activity_at",
                RepoSort::Relevance => "similarity",
            };
            let url = format!(
                "{}/api/v4/projects?search={}&order_by={}&sort=desc&per_page={}",
                self.base_url,
                urlencoding::encode(query),
                order_by,
                limit.min(100),
            );
            let resp = self.client.get(&url).send().await?;
            match resp.status().as_u16() {
                200 => {}
                429 => return Err(ForgeError::RateLimited),
                status => return Err(ForgeError::Parse(format!("unexpected status {status}"))),
            }
            let projects: Vec<GitLabProject> = crate::forges::bounded_json(resp).await?;
            Ok(projects
                .into_iter()
                .map(|p| RepositorySearchResult {
                    forge: self.platform_id.clone(),
                    full_name: p.path_with_namespace,
                    owner: p.namespace.path,
                    name: p.name,
                    description: p.description,
                    stars: p.star_count,
                    // GitLab's project-list API has no language field.
                    language: None,
                    topics: p.topics,
                    url: p.web_url,
                    updated_at: p.last_activity_at.as_deref().and_then(|ts| ts.parse().ok()),
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
    async fn test_gitlab_list_releases() {
        let gl = GitLab::new(
            crate::http::default_client().unwrap(),
            "https://gitlab.com",
            "gitlab",
            "GitLab",
        );
        let releases = gl.list_releases("gitlab-org", "gitlab", 5).await.unwrap();
        assert!(!releases.is_empty());
        assert!(releases.len() <= 5);
    }

    #[tokio::test]
    #[ignore] // requires network
    async fn test_search_repositories() {
        let gl = GitLab::new(
            crate::http::default_client().unwrap(),
            "https://gitlab.com",
            "gitlab",
            "GitLab",
        );
        let results = gl
            .search_repositories("kubernetes", RepoSort::Stars, None, 5)
            .await
            .unwrap();
        assert!(!results.is_empty());
        assert!(results.len() <= 5);
        assert_eq!(results[0].forge, "gitlab");
    }
}
