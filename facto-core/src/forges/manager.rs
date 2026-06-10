use std::collections::HashMap;
use std::sync::Arc;

use crate::config::Config;
use crate::forges::*;
use crate::http;

pub struct ForgeManager {
    forges: HashMap<String, Arc<dyn Forge>>,
}

impl ForgeManager {
    pub fn new(config: &Config) -> Result<Self, http::HttpClientError> {
        let mut forges: HashMap<String, Arc<dyn Forge>> = HashMap::new();
        let default = http::default_client()?;

        for id in &config.forges.enabled {
            let forge: Option<Arc<dyn Forge>> = match id.as_str() {
                "github" => {
                    let c = match &config.forges.github_token {
                        Some(t) => http::bearer_client(t)?,
                        None => default.clone(),
                    };
                    Some(Arc::new(crate::forges::github::GitHub::new(c)))
                }
                "gitlab" => {
                    let c = match &config.forges.gitlab_token {
                        Some(t) => http::private_token_client(t)?,
                        None => default.clone(),
                    };
                    Some(Arc::new(crate::forges::gitlab::GitLab::new(
                        c,
                        &config.forges.gitlab_base_url,
                        "gitlab",
                        "GitLab",
                    )))
                }
                "codeberg" => Some(Arc::new(crate::forges::codeberg::Codeberg::new(
                    default.clone(),
                ))),
                _ => {
                    tracing::warn!("unknown forge: {}", id);
                    None
                }
            };

            if let Some(f) = forge {
                forges.insert(id.clone(), f);
            }
        }

        Ok(Self { forges })
    }

    pub fn list_forges(&self) -> Vec<(String, String)> {
        self.forges
            .iter()
            .map(|(id, f)| (id.clone(), f.display_name().to_string()))
            .collect()
    }

    pub fn get_forge(&self, id: &str) -> Option<&Arc<dyn Forge>> {
        self.forges.get(id)
    }

    /// Resolve a GitHub Action (owner/repo) to a pinned commit SHA. When
    /// `tag` is None, the latest stable (non-prerelease, non-draft) release
    /// is used.
    pub async fn pin_github_action(
        &self,
        action: &str,
        tag: Option<&str>,
    ) -> Result<crate::models::ActionPin, crate::forges::ForgeError> {
        let parts: Vec<&str> = action.splitn(2, '/').collect();
        if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
            return Err(crate::forges::ForgeError::Parse(
                "action must be in owner/repo format (e.g. \"actions/checkout\")".into(),
            ));
        }
        let (owner, repo) = (parts[0], parts[1]);

        let forge = self
            .get_forge("github")
            .ok_or(crate::forges::ForgeError::NotSupported)?;

        let tag = match tag {
            Some(t) => t.to_string(),
            None => {
                let releases = forge.list_releases(owner, repo, 20).await?;
                releases
                    .into_iter()
                    .find(|r| !r.prerelease && !r.draft)
                    .map(|r| r.tag)
                    .ok_or_else(|| {
                        crate::forges::ForgeError::Parse(format!(
                            "no stable release found for {action}"
                        ))
                    })?
            }
        };

        let pin = forge.resolve_tag(owner, repo, &tag).await?;
        let pinned = format!("{}@{} # {}", action, pin.commit_sha, tag);
        Ok(crate::models::ActionPin {
            pinned,
            action: action.to_string(),
            tag,
            commit_sha: pin.commit_sha,
            url: pin.url,
        })
    }
}
