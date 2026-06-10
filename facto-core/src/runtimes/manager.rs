use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use dashmap::DashMap;
use tokio::time::Instant;

use crate::config::Config;
use crate::http;
use crate::models::{RuntimeInfo, RuntimeVersion};
use crate::runtimes::catalog::all_runtimes;
use crate::runtimes::endoflife::EndOfLifeClient;
use crate::runtimes::{Runtime, RuntimeResult};

struct CachedVersions {
    versions: Vec<RuntimeVersion>,
    fetched_at: Instant,
}

pub struct RuntimeManager {
    runtimes: HashMap<String, Box<dyn Runtime>>,
    cache: Arc<DashMap<String, CachedVersions>>,
    client: EndOfLifeClient,
    ttl: Duration,
}

impl RuntimeManager {
    pub fn new(config: &Config) -> Result<Self, http::HttpClientError> {
        let mut runtimes = HashMap::new();

        for rt in all_runtimes() {
            if config.runtimes.enabled.contains(&rt.id().to_string()) {
                runtimes.insert(rt.id().to_string(), rt);
            }
        }

        Ok(Self {
            runtimes,
            cache: Arc::new(DashMap::new()),
            client: EndOfLifeClient::new(http::default_client()?),
            ttl: Duration::from_secs(config.cache.runtimes_secs),
        })
    }

    pub fn list_runtimes(&self) -> Vec<(String, String)> {
        self.runtimes
            .iter()
            .map(|(id, rt)| (id.clone(), rt.display_name().to_string()))
            .collect()
    }

    pub async fn get_runtime_info(&self, id: &str) -> RuntimeResult<RuntimeInfo> {
        let rt = self
            .runtimes
            .get(id)
            .ok_or_else(|| crate::runtimes::RuntimeError::Unknown(id.to_string()))?;

        let versions = self.get_versions_cached(rt.endoflife_id()).await?;

        let (latest_stable, latest_cycle) = versions
            .first()
            .map(|v| (v.latest.clone(), v.cycle.clone()))
            .unwrap_or_default();

        let changelog_url = rt.changelog_url(&latest_cycle);

        Ok(RuntimeInfo {
            id: rt.id().to_string(),
            name: rt.display_name().to_string(),
            latest_stable,
            latest_cycle,
            changelog_url,
            versions,
        })
    }

    pub async fn get_versions(
        &self,
        id: &str,
        active_only: bool,
    ) -> RuntimeResult<Vec<RuntimeVersion>> {
        let rt = self
            .runtimes
            .get(id)
            .ok_or_else(|| crate::runtimes::RuntimeError::Unknown(id.to_string()))?;

        let versions = self.get_versions_cached(rt.endoflife_id()).await?;

        if active_only {
            let today = chrono::Utc::now().date_naive();
            Ok(versions
                .into_iter()
                .filter(|v| match &v.eol {
                    crate::models::EolStatus::Date(d) => *d > today,
                    crate::models::EolStatus::Bool(b) => !b,
                })
                .collect())
        } else {
            Ok(versions)
        }
    }

    async fn get_versions_cached(&self, endoflife_id: &str) -> RuntimeResult<Vec<RuntimeVersion>> {
        // Check cache
        if let Some(cached) = self.cache.get(endoflife_id)
            && cached.fetched_at.elapsed() < self.ttl
        {
            return Ok(cached.versions.clone());
        }

        // Fetch and cache
        let versions = self.client.fetch_cycles(endoflife_id).await?;
        self.cache.insert(
            endoflife_id.to_string(),
            CachedVersions {
                versions: versions.clone(),
                fetched_at: Instant::now(),
            },
        );

        Ok(versions)
    }
}
