use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::*;
use rmcp::tool_router;
use rmcp::{ErrorData as McpError, tool};
use schemars::JsonSchema;
use serde::Deserialize;

use super::{check_params, tool_error_result, unknown_id};
use crate::FactoMcp;

#[derive(Debug, Deserialize, JsonSchema)]
struct SearchPackagesParams {
    /// Search query string
    query: String,
    /// Registry ID (e.g. "pypi", "npm", "crates")
    registry: String,
    /// Maximum number of results to return (default 20, max 100)
    limit: Option<usize>,
    /// Sort order: "relevance" (default) or "popularity" (by downloads)
    sort: Option<facto_core::models::SearchSort>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct SearchRepositoriesParams {
    /// Search query string
    query: String,
    /// Forge ID (e.g. "github", "gitlab", "codeberg")
    forge: String,
    /// Sort order: "stars" (default), "updated", or "relevance"
    sort: Option<facto_core::models::RepoSort>,
    /// Best-effort language filter (honoured by GitHub; ignored by others)
    language: Option<String>,
    /// Maximum number of results to return (default 20, max 100)
    limit: Option<usize>,
}

#[tool_router(router = tool_router_search_inner, vis = "pub")]
impl FactoMcp {
    #[tool(
        description = "Search for packages on a specific registry by keyword. Returns matching packages with name, description, latest version, download count (registry-relative: lifetime total for most registries, monthly for npm), and keywords. Supports sort=\"popularity\" to order results by downloads descending (entries without a download count appear last); default sort is registry-native relevance. Use this when the user is looking for a package to solve a specific problem.",
        annotations(read_only_hint = true)
    )]
    async fn search_packages(
        &self,
        Parameters(p): Parameters<SearchPackagesParams>,
    ) -> Result<CallToolResult, McpError> {
        if let Some(err) = check_params(&[(&p.query, "query"), (&p.registry, "registry")]) {
            return Ok(err);
        }

        let registry = match self.registries.get_registry(&p.registry) {
            Some(r) => r,
            None => return Ok(unknown_id("registry", &p.registry, "list_registries")),
        };

        let limit = p.limit.unwrap_or(20).min(100);
        let sort = p.sort.unwrap_or_default();

        // For popularity, over-fetch a candidate pool so the download sort can
        // surface globally-popular packages, not just rerank the top-`limit`
        // relevance hits. For relevance, fetch exactly what we return.
        let fetch = match sort {
            facto_core::models::SearchSort::Popularity => 100,
            facto_core::models::SearchSort::Relevance => limit,
        };

        let mut results = match registry.search(&p.query, fetch).await {
            Ok(r) => r,
            Err(e) => return Ok(tool_error_result(e)),
        };

        if matches!(sort, facto_core::models::SearchSort::Popularity) {
            results.sort_by_key(|b| std::cmp::Reverse(b.downloads.unwrap_or(0)));
        }

        results.truncate(limit);

        Ok(CallToolResult::success(vec![Content::json(results)?]))
    }

    #[tool(
        description = "Search for repositories/projects on a code forge (github, gitlab, codeberg) by keyword, sorted by popularity (stars) by default. Returns owner, name, description, star count, language, topics, and URL. Use this to discover existing projects.",
        annotations(read_only_hint = true)
    )]
    async fn search_repositories(
        &self,
        Parameters(p): Parameters<SearchRepositoriesParams>,
    ) -> Result<CallToolResult, McpError> {
        if let Some(err) = check_params(&[(&p.query, "query"), (&p.forge, "forge")]) {
            return Ok(err);
        }

        let forge = match self.forges.get_forge(&p.forge) {
            Some(f) => f,
            None => return Ok(unknown_id("forge", &p.forge, "list_forges")),
        };

        let limit = p.limit.unwrap_or(20).min(100);
        let sort = p.sort.unwrap_or_default();
        let results = match forge
            .search_repositories(&p.query, sort, p.language.as_deref(), limit)
            .await
        {
            Ok(r) => r,
            Err(e) => return Ok(tool_error_result(e)),
        };

        Ok(CallToolResult::success(vec![Content::json(results)?]))
    }
}

#[cfg(test)]
mod tests {
    use facto_core::models::SearchResult;

    fn r(name: &str, downloads: Option<u64>) -> SearchResult {
        SearchResult {
            name: name.into(),
            description: None,
            latest_version: None,
            downloads,
            keywords: Vec::new(),
        }
    }

    #[test]
    fn popularity_sort_orders_by_downloads_desc_nones_last() {
        let mut results = [r("a", Some(10)), r("b", None), r("c", Some(100))];
        results.sort_by_key(|b| std::cmp::Reverse(b.downloads.unwrap_or(0)));
        let names: Vec<&str> = results.iter().map(|x| x.name.as_str()).collect();
        assert_eq!(names, vec!["c", "a", "b"]);
    }
}
