use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

mod datetime_format_opt {
    use chrono::{DateTime, Utc};
    use serde::{self, Deserialize, Deserializer, Serializer};

    const FORMAT: &str = "%Y-%m-%dT%H:%M:%SZ";

    pub fn serialize<S>(date: &Option<DateTime<Utc>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match date {
            Some(d) => serializer.serialize_str(&d.format(FORMAT).to_string()),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<DateTime<Utc>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let opt: Option<String> = Option::deserialize(deserializer)?;
        match opt {
            Some(s) => s
                .parse::<DateTime<Utc>>()
                .map(Some)
                .map_err(serde::de::Error::custom),
            None => Ok(None),
        }
    }
}

// --- Registry types ---

/// Result of a package lookup: the metadata when the package exists, or a
/// not-found marker meaning the name is free on this registry.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PackageLookup {
    pub found: bool,
    pub name: String,
    pub registry: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package: Option<PackageInfo>,
}

/// Package metadata from a registry.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PackageInfo {
    pub name: String,
    pub registry: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub authors: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none", with = "datetime_format_opt")]
    #[schemars(with = "Option<String>")]
    pub updated_at: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub keywords: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub classifiers: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requires_python: Option<String>,
}

/// A single published version with release date.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct VersionInfo {
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none", with = "datetime_format_opt")]
    #[schemars(with = "Option<String>")]
    pub released_at: Option<DateTime<Utc>>,
    pub prerelease: bool,
}

/// Result of a version listing: the versions when the package exists, or a
/// not-found marker meaning the name is free on this registry.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct VersionList {
    pub found: bool,
    pub name: String,
    pub registry: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub versions: Vec<VersionInfo>,
}

/// Result of a latest-stable-version lookup: the version when one exists, or a
/// not-found marker. `found: true` with `version: None` means the package
/// exists but publishes no stable (non-prerelease) release.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LatestVersion {
    pub found: bool,
    pub name: String,
    pub registry: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<VersionInfo>,
}

/// A search result entry from a registry.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SearchResult {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub downloads: Option<u64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub keywords: Vec<String>,
}

/// Sort order for package search. Best-effort: popularity sorts the
/// returned page by `downloads` (descending), leaving entries without a
/// download count last.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SearchSort {
    /// Registry-native relevance to the query (default).
    #[default]
    Relevance,
    /// Most downloads first (best-effort over the returned page).
    Popularity,
}

// --- Runtime types ---

/// End-of-life status: either a date or a boolean flag.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
#[non_exhaustive]
pub enum EolStatus {
    Date(chrono::NaiveDate),
    Bool(bool),
}

/// A runtime version cycle with EOL status.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RuntimeVersion {
    pub cycle: String,
    pub latest: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub release_date: Option<chrono::NaiveDate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_release_date: Option<chrono::NaiveDate>,
    pub eol: EolStatus,
    pub lts: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub support: Option<EolStatus>,
}

/// Runtime lifecycle info from endoflife.date.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RuntimeInfo {
    pub id: String,
    pub name: String,
    pub latest_stable: String,
    pub latest_cycle: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub changelog_url: Option<String>,
    pub versions: Vec<RuntimeVersion>,
}

// --- Release types ---

/// A release from a forge repository.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ReleaseInfo {
    pub tag: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", with = "datetime_format_opt")]
    #[schemars(with = "Option<String>")]
    pub published_at: Option<DateTime<Utc>>,
    pub prerelease: bool,
    pub draft: bool,
    pub html_url: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub assets: Vec<ReleaseAsset>,
}

/// A downloadable asset attached to a release.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ReleaseAsset {
    pub name: String,
    pub size: u64,
    pub download_url: String,
    pub download_count: u64,
    pub content_type: String,
}

// --- Forge search types ---

/// Sort order for forge repository search. Best-effort per forge.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RepoSort {
    /// Most stars first (default).
    #[default]
    Stars,
    /// Most recently updated first.
    Updated,
    /// Forge-native relevance to the query.
    Relevance,
}

/// A repository/project search hit from a code forge.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RepositorySearchResult {
    pub forge: String,
    pub full_name: String,
    pub owner: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub stars: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub topics: Vec<String>,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none", with = "datetime_format_opt")]
    #[schemars(with = "Option<String>")]
    pub updated_at: Option<DateTime<Utc>>,
}

// --- Tag pin types ---

/// A pinned GitHub Action: the ready-to-paste reference plus its parts.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ActionPin {
    pub pinned: String,
    pub action: String,
    pub tag: String,
    pub commit_sha: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// Resolved tag reference with full commit SHA for secure pinning.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TagPin {
    pub owner: String,
    pub repository: String,
    pub forge_id: String,
    pub tag: String,
    pub commit_sha: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repository_search_result_omits_empty_optionals() {
        let r = RepositorySearchResult {
            forge: "github".into(),
            full_name: "owner/repo".into(),
            owner: "owner".into(),
            name: "repo".into(),
            description: None,
            stars: 42,
            language: None,
            topics: Vec::new(),
            url: "https://github.com/owner/repo".into(),
            updated_at: None,
        };
        let v = serde_json::to_value(&r).unwrap();
        assert_eq!(v["stars"], 42);
        assert_eq!(v["full_name"], "owner/repo");
        assert!(v.get("description").is_none());
        assert!(v.get("language").is_none());
        assert!(v.get("topics").is_none());
        assert!(v.get("updated_at").is_none());
    }

    #[test]
    fn repo_sort_defaults_to_stars() {
        assert!(matches!(RepoSort::default(), RepoSort::Stars));
    }

    #[test]
    fn search_result_omits_empty_downloads_and_keywords() {
        let r = SearchResult {
            name: "requests".into(),
            description: Some("HTTP for Humans".into()),
            latest_version: Some("2.32.5".into()),
            downloads: None,
            keywords: Vec::new(),
        };
        let v = serde_json::to_value(&r).unwrap();
        assert_eq!(v["name"], "requests");
        assert!(v.get("downloads").is_none());
        assert!(v.get("keywords").is_none());
    }

    #[test]
    fn search_sort_defaults_to_relevance() {
        assert!(matches!(SearchSort::default(), SearchSort::Relevance));
    }

    #[test]
    fn version_list_not_found_omits_empty_fields() {
        let list = VersionList {
            found: false,
            name: "nope".into(),
            registry: "crates".into(),
            latest: None,
            versions: Vec::new(),
        };
        let v = serde_json::to_value(&list).unwrap();
        assert_eq!(v["found"], false);
        assert!(v.get("latest").is_none());
        assert!(v.get("versions").is_none());
    }

    #[test]
    fn latest_version_not_found_omits_version() {
        let lv = LatestVersion {
            found: false,
            name: "nope".into(),
            registry: "crates".into(),
            version: None,
        };
        let v = serde_json::to_value(&lv).unwrap();
        assert_eq!(v["found"], false);
        assert!(v.get("version").is_none());
    }
}
