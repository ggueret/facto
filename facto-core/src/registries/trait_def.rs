use std::future::Future;
use std::pin::Pin;

use thiserror::Error;

use crate::models::*;

#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("timeout")]
    Timeout,
    #[error("not found")]
    NotFound,
    #[error("rate limited")]
    RateLimited,
    #[error("parse error: {0}")]
    Parse(String),
    #[error("not supported")]
    NotSupported,
}

pub type RegistryResult<T> = Result<T, RegistryError>;

/// A package registry backend (e.g., PyPI, npm, crates.io).
///
/// Implementations provide package metadata, version listings, and search
/// across a single registry.
pub trait Registry: Send + Sync {
    /// Unique registry identifier (e.g., "pypi", "npm").
    fn id(&self) -> &str;
    /// Human-readable registry name.
    fn display_name(&self) -> &str;

    /// Fetch package metadata (description, latest version, URLs).
    fn get_package<'a>(
        &'a self,
        name: &'a str,
    ) -> Pin<Box<dyn Future<Output = RegistryResult<PackageInfo>> + Send + 'a>>;
    /// List all published versions, newest first.
    fn get_versions<'a>(
        &'a self,
        name: &'a str,
    ) -> Pin<Box<dyn Future<Output = RegistryResult<Vec<VersionInfo>>> + Send + 'a>>;
    /// Search packages by query string.
    fn search<'a>(
        &'a self,
        query: &'a str,
        limit: usize,
    ) -> Pin<Box<dyn Future<Output = RegistryResult<Vec<SearchResult>>> + Send + 'a>>;

    /// Whether `get_latest_version` yields a reliable result. Registries whose
    /// versions can be ordered by semver return `true`. Docker Hub orders
    /// free-form, non-semver tags by push date, so "latest" is not meaningful;
    /// it returns `false` and callers should use `list_versions` instead.
    fn supports_latest_version(&self) -> bool {
        true
    }
}
