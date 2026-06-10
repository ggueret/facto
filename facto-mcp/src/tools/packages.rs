use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::*;
use rmcp::tool_router;
use rmcp::{ErrorData as McpError, tool};
use schemars::JsonSchema;
use serde::Deserialize;

use super::{check_params, tool_error_result, unknown_id};
use crate::FactoMcp;

// --- Parameter structs ---

#[derive(Debug, Deserialize, JsonSchema)]
struct GetPackageParams {
    /// Package name to look up
    name: String,
    /// Registry ID (e.g. "pypi", "npm", "crates")
    registry: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ListVersionsParams {
    /// Package name to list versions for
    name: String,
    /// Registry ID (e.g. "pypi", "npm", "crates")
    registry: String,
    /// If true, only return stable (non-prerelease) versions; defaults to false
    stable_only: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct GetLatestVersionParams {
    /// Package name to query
    name: String,
    /// Registry ID (e.g. "pypi", "npm", "crates")
    registry: String,
}

// --- Tool implementations ---

#[tool_router(router = tool_router_packages_inner, vis = "pub")]
impl FactoMcp {
    #[tool(
        description = "Get package metadata from a registry: name, description, latest version, license, homepage, repository URL, and authors. Use this instead of relying on training data when you need accurate, current package information.",
        annotations(read_only_hint = true)
    )]
    async fn get_package(
        &self,
        Parameters(p): Parameters<GetPackageParams>,
    ) -> Result<CallToolResult, McpError> {
        if let Some(err) = check_params(&[(&p.name, "name"), (&p.registry, "registry")]) {
            return Ok(err);
        }

        let registry = match self.registries.get_registry(&p.registry) {
            Some(r) => r,
            None => return Ok(unknown_id("registry", &p.registry, "list_registries")),
        };

        let lookup = match registry.get_package(&p.name).await {
            Ok(info) => facto_core::models::PackageLookup {
                found: true,
                name: p.name,
                registry: p.registry,
                package: Some(info),
            },
            Err(facto_core::registries::RegistryError::NotFound) => {
                facto_core::models::PackageLookup {
                    found: false,
                    name: p.name,
                    registry: p.registry,
                    package: None,
                }
            }
            Err(e) => return Ok(tool_error_result(e)),
        };

        Ok(CallToolResult::success(vec![Content::json(lookup)?]))
    }

    #[tool(
        description = "List all published versions of a package with release dates and prerelease flags, sorted by semver (newest first). Use the stable_only parameter to filter out prereleases. Use this when you need to know what versions exist or when a specific version was released.",
        annotations(read_only_hint = true)
    )]
    async fn list_versions(
        &self,
        Parameters(p): Parameters<ListVersionsParams>,
    ) -> Result<CallToolResult, McpError> {
        if let Some(err) = check_params(&[(&p.name, "name"), (&p.registry, "registry")]) {
            return Ok(err);
        }

        let registry = match self.registries.get_registry(&p.registry) {
            Some(r) => r,
            None => return Ok(unknown_id("registry", &p.registry, "list_registries")),
        };

        let stable_only = p.stable_only.unwrap_or(false);
        let lookup = match registry.get_versions(&p.name).await {
            Ok(mut versions) => {
                if stable_only {
                    versions.retain(|v| !v.prerelease);
                }
                let latest = versions
                    .iter()
                    .find(|v| !v.prerelease)
                    .map(|v| v.version.clone());
                facto_core::models::VersionList {
                    found: true,
                    name: p.name,
                    registry: p.registry,
                    latest,
                    versions,
                }
            }
            Err(facto_core::registries::RegistryError::NotFound) => {
                facto_core::models::VersionList {
                    found: false,
                    name: p.name,
                    registry: p.registry,
                    latest: None,
                    versions: Vec::new(),
                }
            }
            Err(e) => return Ok(tool_error_result(e)),
        };

        Ok(CallToolResult::success(vec![Content::json(lookup)?]))
    }

    #[tool(
        description = "Get the latest stable version of a package. Use this instead of relying on training data when recommending or pinning a package version. Your memorized version numbers are likely outdated.",
        annotations(read_only_hint = true)
    )]
    async fn get_latest_version(
        &self,
        Parameters(p): Parameters<GetLatestVersionParams>,
    ) -> Result<CallToolResult, McpError> {
        if let Some(err) = check_params(&[(&p.name, "name"), (&p.registry, "registry")]) {
            return Ok(err);
        }

        let registry = match self.registries.get_registry(&p.registry) {
            Some(r) => r,
            None => return Ok(unknown_id("registry", &p.registry, "list_registries")),
        };

        if !registry.supports_latest_version() {
            return Ok(tool_error_result(
                facto_core::registries::RegistryError::NotSupported,
            ));
        }

        let lookup = match registry.get_versions(&p.name).await {
            Ok(versions) => facto_core::models::LatestVersion {
                found: true,
                name: p.name,
                registry: p.registry,
                version: versions.into_iter().find(|v| !v.prerelease),
            },
            Err(facto_core::registries::RegistryError::NotFound) => {
                facto_core::models::LatestVersion {
                    found: false,
                    name: p.name,
                    registry: p.registry,
                    version: None,
                }
            }
            Err(e) => return Ok(tool_error_result(e)),
        };

        Ok(CallToolResult::success(vec![Content::json(lookup)?]))
    }
}

#[cfg(test)]
mod tests {
    use facto_core::models::{PackageInfo, PackageLookup};

    #[test]
    fn package_lookup_not_found_shape() {
        let lookup = PackageLookup {
            found: false,
            name: "nope".into(),
            registry: "pypi".into(),
            package: None,
        };
        let v = serde_json::to_value(&lookup).unwrap();
        assert_eq!(v["found"], false);
        assert_eq!(v["name"], "nope");
        assert_eq!(v["registry"], "pypi");
        assert!(v.get("package").is_none());
    }

    #[test]
    fn package_lookup_found_shape() {
        let info = PackageInfo {
            name: "requests".into(),
            registry: "pypi".into(),
            latest_version: Some("2.32.5".into()),
            description: None,
            license: None,
            homepage: None,
            repository: None,
            authors: Vec::new(),
            updated_at: None,
            keywords: Vec::new(),
            classifiers: Vec::new(),
            requires_python: None,
        };
        let lookup = PackageLookup {
            found: true,
            name: "requests".into(),
            registry: "pypi".into(),
            package: Some(info),
        };
        let v = serde_json::to_value(&lookup).unwrap();
        assert_eq!(v["found"], true);
        assert_eq!(v["package"]["latest_version"], "2.32.5");
    }
}
