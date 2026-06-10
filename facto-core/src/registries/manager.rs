use std::collections::HashMap;
use std::sync::Arc;

use futures::stream::{self, StreamExt};

use crate::config::Config;
use crate::http;
use crate::registries::cache::CachedRegistry;
use crate::registries::*;

pub struct RegistryManager {
    registries: HashMap<String, Arc<dyn Registry>>,
}

impl RegistryManager {
    pub fn new(config: &Config) -> Result<Self, http::HttpClientError> {
        let mut registries: HashMap<String, Arc<dyn Registry>> = HashMap::new();
        let client = http::default_client()?;
        let ttl = config.cache.clone();

        for id in &config.registries.enabled {
            let registry: Option<Arc<dyn Registry>> = match id.as_str() {
                "pypi" => Some(Arc::new(CachedRegistry::new(
                    crate::registries::pypi::PyPI::new(client.clone()),
                    ttl.clone(),
                ))),
                "npm" => Some(Arc::new(CachedRegistry::new(
                    crate::registries::npm::Npm::new(client.clone()),
                    ttl.clone(),
                ))),
                "crates" => Some(Arc::new(CachedRegistry::new(
                    crate::registries::crates_io::CratesIo::new(client.clone()),
                    ttl.clone(),
                ))),
                "go" => Some(Arc::new(CachedRegistry::new(
                    crate::registries::go::GoModules::new(client.clone()),
                    ttl.clone(),
                ))),
                "rubygems" => Some(Arc::new(CachedRegistry::new(
                    crate::registries::rubygems::RubyGems::new(client.clone()),
                    ttl.clone(),
                ))),
                "maven" => Some(Arc::new(CachedRegistry::new(
                    crate::registries::maven::Maven::new(client.clone()),
                    ttl.clone(),
                ))),
                "nuget" => Some(Arc::new(CachedRegistry::new(
                    crate::registries::nuget::NuGet::new(client.clone()),
                    ttl.clone(),
                ))),
                "packagist" => Some(Arc::new(CachedRegistry::new(
                    crate::registries::packagist::Packagist::new(client.clone()),
                    ttl.clone(),
                ))),
                "dockerhub" => Some(Arc::new(CachedRegistry::new(
                    crate::registries::dockerhub::DockerHub::new(client.clone()),
                    ttl.clone(),
                ))),
                _ => {
                    tracing::warn!("unknown registry: {}", id);
                    None
                }
            };

            if let Some(r) = registry {
                registries.insert(id.clone(), r);
            }
        }

        Ok(Self { registries })
    }

    pub fn list_registries(&self) -> Vec<(String, String)> {
        self.registries
            .iter()
            .map(|(id, r)| (id.clone(), r.display_name().to_string()))
            .collect()
    }

    pub fn get_registry(&self, id: &str) -> Option<&Arc<dyn Registry>> {
        self.registries.get(id)
    }

    /// For each dependency, look up the latest stable version on its
    /// registry and flag whether it is outdated. Concurrency-bounded.
    pub async fn check_deps(
        &self,
        deps: Vec<crate::lockfiles::ResolvedDep>,
    ) -> Vec<crate::lockfiles::CheckedDep> {
        stream::iter(deps)
            .map(|dep| {
                let registry = self.registries.get(&dep.registry).cloned();
                async move {
                    let Some(registry) = registry else {
                        return crate::lockfiles::CheckedDep {
                            name: dep.name,
                            current: dep.version,
                            latest: None,
                            outdated: None,
                            registry: dep.registry,
                            group: dep.group,
                            error: Some("unknown registry".to_string()),
                        };
                    };
                    match registry.get_versions(&dep.name).await {
                        Ok(versions) => {
                            let latest = versions
                                .into_iter()
                                .find(|v| !v.prerelease)
                                .map(|v| v.version);
                            let outdated = latest.as_ref().map(|l| l != &dep.version);
                            crate::lockfiles::CheckedDep {
                                name: dep.name,
                                current: dep.version,
                                latest,
                                outdated,
                                registry: dep.registry,
                                group: dep.group,
                                error: None,
                            }
                        }
                        Err(e) => crate::lockfiles::CheckedDep {
                            name: dep.name,
                            current: dep.version,
                            latest: None,
                            outdated: None,
                            registry: dep.registry,
                            group: dep.group,
                            error: Some(e.to_string()),
                        },
                    }
                }
            })
            .buffer_unordered(16)
            .collect()
            .await
    }
}
