//! TTL-based cache decorator for [`Registry`] implementations.
//!
//! `CachedRegistry<R>` wraps any concrete `Registry` and caches successful
//! responses for each method, keyed by input arguments. TTLs are taken from
//! [`CacheTtlConfig`]. Errors are never cached -- the next call retries.
//!
//! Used in [`super::manager::RegistryManager::new`] to wire configured TTLs
//! to actual registry calls (`get_package`, `get_versions`, `search`).
use std::future::Future;
use std::hash::Hash;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use dashmap::DashMap;
use tokio::time::Instant;

use crate::config::CacheTtlConfig;
use crate::models::*;
use crate::registries::{Registry, RegistryResult};

type CacheMap<K, V> = Arc<DashMap<K, (V, Instant)>>;

pub struct CachedRegistry<R: Registry> {
    inner: R,
    get_package_cache: CacheMap<String, PackageInfo>,
    get_versions_cache: CacheMap<String, Vec<VersionInfo>>,
    search_cache: CacheMap<(String, usize), Vec<SearchResult>>,
    ttl: CacheTtlConfig,
}

impl<R: Registry> CachedRegistry<R> {
    pub fn new(inner: R, ttl: CacheTtlConfig) -> Self {
        Self {
            inner,
            get_package_cache: Arc::new(DashMap::new()),
            get_versions_cache: Arc::new(DashMap::new()),
            search_cache: Arc::new(DashMap::new()),
            ttl,
        }
    }
}

fn get_fresh<K, V>(cache: &DashMap<K, (V, Instant)>, key: &K, ttl: Duration) -> Option<V>
where
    K: Hash + Eq,
    V: Clone,
{
    cache
        .get(key)
        .filter(|e| e.1.elapsed() < ttl)
        .map(|e| e.0.clone())
}

impl<R: Registry + 'static> Registry for CachedRegistry<R> {
    fn id(&self) -> &str {
        self.inner.id()
    }

    fn display_name(&self) -> &str {
        self.inner.display_name()
    }

    fn supports_latest_version(&self) -> bool {
        self.inner.supports_latest_version()
    }

    fn get_package<'a>(
        &'a self,
        name: &'a str,
    ) -> Pin<Box<dyn Future<Output = RegistryResult<PackageInfo>> + Send + 'a>> {
        Box::pin(async move {
            let ttl = Duration::from_secs(self.ttl.package_secs);
            let key = name.to_string();
            if let Some(v) = get_fresh(&self.get_package_cache, &key, ttl) {
                return Ok(v);
            }
            let v = self.inner.get_package(name).await?;
            self.get_package_cache
                .insert(key, (v.clone(), Instant::now()));
            Ok(v)
        })
    }

    fn get_versions<'a>(
        &'a self,
        name: &'a str,
    ) -> Pin<Box<dyn Future<Output = RegistryResult<Vec<VersionInfo>>> + Send + 'a>> {
        Box::pin(async move {
            let ttl = Duration::from_secs(self.ttl.package_secs);
            let key = name.to_string();
            if let Some(v) = get_fresh(&self.get_versions_cache, &key, ttl) {
                return Ok(v);
            }
            let v = self.inner.get_versions(name).await?;
            self.get_versions_cache
                .insert(key, (v.clone(), Instant::now()));
            Ok(v)
        })
    }

    fn search<'a>(
        &'a self,
        query: &'a str,
        limit: usize,
    ) -> Pin<Box<dyn Future<Output = RegistryResult<Vec<SearchResult>>> + Send + 'a>> {
        Box::pin(async move {
            let ttl = Duration::from_secs(self.ttl.search_secs);
            let key = (query.to_string(), limit);
            if let Some(v) = get_fresh(&self.search_cache, &key, ttl) {
                return Ok(v);
            }
            let v = self.inner.search(query, limit).await?;
            self.search_cache.insert(key, (v.clone(), Instant::now()));
            Ok(v)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registries::{Registry, RegistryError, RegistryResult};
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct CountingRegistry {
        get_package_calls: AtomicUsize,
    }

    impl Registry for CountingRegistry {
        fn id(&self) -> &str {
            "counting"
        }
        fn display_name(&self) -> &str {
            "Counting"
        }

        fn supports_latest_version(&self) -> bool {
            false
        }

        fn get_package<'a>(
            &'a self,
            _name: &'a str,
        ) -> Pin<Box<dyn Future<Output = RegistryResult<PackageInfo>> + Send + 'a>> {
            Box::pin(async move {
                self.get_package_calls.fetch_add(1, Ordering::SeqCst);
                Ok(PackageInfo {
                    name: "foo".into(),
                    registry: "counting".into(),
                    latest_version: None,
                    description: None,
                    license: None,
                    homepage: None,
                    repository: None,
                    authors: Vec::new(),
                    updated_at: None,
                    keywords: Vec::new(),
                    classifiers: Vec::new(),
                    requires_python: None,
                })
            })
        }

        fn get_versions<'a>(
            &'a self,
            _name: &'a str,
        ) -> Pin<Box<dyn Future<Output = RegistryResult<Vec<VersionInfo>>> + Send + 'a>> {
            Box::pin(async move { Err(RegistryError::Parse("unused".into())) })
        }

        fn search<'a>(
            &'a self,
            _query: &'a str,
            _limit: usize,
        ) -> Pin<Box<dyn Future<Output = RegistryResult<Vec<SearchResult>>> + Send + 'a>> {
            Box::pin(async move { Ok(Vec::new()) })
        }
    }

    #[tokio::test]
    async fn cached_registry_hits_cache_on_second_call() {
        let inner = CountingRegistry {
            get_package_calls: AtomicUsize::new(0),
        };
        let cached = CachedRegistry::new(inner, CacheTtlConfig::default());

        cached.get_package("foo").await.unwrap();
        cached.get_package("foo").await.unwrap();

        assert_eq!(
            cached.inner.get_package_calls.load(Ordering::SeqCst),
            1,
            "second call should have come from cache"
        );
    }

    #[tokio::test]
    async fn cached_registry_different_keys_hit_inner() {
        let inner = CountingRegistry {
            get_package_calls: AtomicUsize::new(0),
        };
        let cached = CachedRegistry::new(inner, CacheTtlConfig::default());

        cached.get_package("foo").await.unwrap();
        cached.get_package("bar").await.unwrap();

        assert_eq!(cached.inner.get_package_calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn cached_registry_expired_entries_refetch() {
        let inner = CountingRegistry {
            get_package_calls: AtomicUsize::new(0),
        };
        let ttl = CacheTtlConfig {
            package_secs: 0, // instant expiry
            ..Default::default()
        };
        let cached = CachedRegistry::new(inner, ttl);

        cached.get_package("foo").await.unwrap();
        tokio::time::sleep(Duration::from_millis(10)).await;
        cached.get_package("foo").await.unwrap();

        assert_eq!(
            cached.inner.get_package_calls.load(Ordering::SeqCst),
            2,
            "expired entry should refetch"
        );
    }

    #[test]
    fn forwards_supports_latest_version() {
        let inner = CountingRegistry {
            get_package_calls: AtomicUsize::new(0),
        };
        let cached = CachedRegistry::new(inner, CacheTtlConfig::default());
        assert!(!cached.supports_latest_version());
    }
}
