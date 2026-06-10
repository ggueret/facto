use std::future::Future;
use std::pin::Pin;

use thiserror::Error;

use crate::models::{ReleaseInfo, RepoSort, RepositorySearchResult, TagPin};

#[derive(Debug, Error)]
pub enum ForgeError {
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("timeout")]
    Timeout,
    #[error("rate limited")]
    RateLimited,
    #[error("parse error: {0}")]
    Parse(String),
    #[error("not supported")]
    NotSupported,
}

pub type ForgeResult<T> = Result<T, ForgeError>;

/// A code forge backend (e.g., GitHub, GitLab, Codeberg).
///
/// Implementations provide release listings and tag resolution.
pub trait Forge: Send + Sync {
    /// Unique forge identifier (e.g., "github", "gitlab").
    fn id(&self) -> &str;
    /// Human-readable forge name.
    fn display_name(&self) -> &str;

    /// Whether this forge supports release listings.
    fn supports_releases(&self) -> bool {
        false
    }

    /// List releases for a repository, newest first.
    fn list_releases<'a>(
        &'a self,
        _owner: &'a str,
        _repo: &'a str,
        _limit: usize,
    ) -> Pin<Box<dyn Future<Output = ForgeResult<Vec<ReleaseInfo>>> + Send + 'a>> {
        Box::pin(async { Err(ForgeError::NotSupported) })
    }

    /// Resolve a tag or ref to its full commit SHA.
    fn resolve_tag<'a>(
        &'a self,
        _owner: &'a str,
        _repo: &'a str,
        _tag: &'a str,
    ) -> Pin<Box<dyn Future<Output = ForgeResult<TagPin>> + Send + 'a>> {
        Box::pin(async { Err(ForgeError::NotSupported) })
    }

    /// Whether this forge supports repository search.
    fn supports_search(&self) -> bool {
        false
    }

    /// Search repositories/projects, best-effort sorted. `language` is a
    /// best-effort filter (only some forges honour it).
    fn search_repositories<'a>(
        &'a self,
        _query: &'a str,
        _sort: RepoSort,
        _language: Option<&'a str>,
        _limit: usize,
    ) -> Pin<Box<dyn Future<Output = ForgeResult<Vec<RepositorySearchResult>>> + Send + 'a>> {
        Box::pin(async { Err(ForgeError::NotSupported) })
    }
}
