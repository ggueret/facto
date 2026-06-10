use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Config {
    pub cache: CacheTtlConfig,
    pub timeouts: TimeoutConfig,
    pub registries: RegistriesConfig,
    pub forges: ForgesConfig,
    pub runtimes: RuntimesConfig,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct CacheTtlConfig {
    pub package_secs: u64,
    pub search_secs: u64,
    pub forge_secs: u64,
    pub releases_secs: u64,
    pub runtimes_secs: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct TimeoutConfig {
    pub per_registry_secs: u64,
    pub global_secs: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct RegistriesConfig {
    pub enabled: Vec<String>,
}

#[derive(Clone, Deserialize)]
#[serde(default)]
pub struct ForgesConfig {
    pub enabled: Vec<String>,
    pub github_token: Option<String>,
    pub gitlab_token: Option<String>,
    pub gitlab_base_url: String,
}

impl std::fmt::Debug for ForgesConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ForgesConfig")
            .field("enabled", &self.enabled)
            .field("github_token", &self.github_token.as_ref().map(|_| "***"))
            .field("gitlab_token", &self.gitlab_token.as_ref().map(|_| "***"))
            .field("gitlab_base_url", &self.gitlab_base_url)
            .finish()
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct RuntimesConfig {
    pub enabled: Vec<String>,
}

impl Default for CacheTtlConfig {
    fn default() -> Self {
        Self {
            package_secs: 900,
            search_secs: 300,
            forge_secs: 600,
            releases_secs: 1800,
            runtimes_secs: 21600,
        }
    }
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            per_registry_secs: 5,
            global_secs: 15,
        }
    }
}

impl Default for RegistriesConfig {
    fn default() -> Self {
        Self {
            enabled: vec![
                "pypi".into(),
                "npm".into(),
                "crates".into(),
                "go".into(),
                "rubygems".into(),
                "maven".into(),
                "nuget".into(),
                "packagist".into(),
                "dockerhub".into(),
            ],
        }
    }
}

impl Default for ForgesConfig {
    fn default() -> Self {
        Self {
            enabled: vec!["github".into(), "gitlab".into(), "codeberg".into()],
            github_token: None,
            gitlab_token: None,
            gitlab_base_url: "https://gitlab.com".to_string(),
        }
    }
}

impl Default for RuntimesConfig {
    fn default() -> Self {
        Self {
            enabled: vec![
                "python".into(),
                "go".into(),
                "rust".into(),
                "nodejs".into(),
                "ruby".into(),
                "php".into(),
                "java".into(),
                "dotnet".into(),
                "deno".into(),
                "bun".into(),
                "elixir".into(),
                "kotlin".into(),
                "perl".into(),
                "scala".into(),
            ],
        }
    }
}

impl Config {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let path_str = std::env::var("FACTO_CONFIG").unwrap_or_else(|_| "facto.toml".to_string());
        let path = Path::new(&path_str);

        let mut config = if path.exists() {
            let content = std::fs::read_to_string(path)?;
            toml::from_str(&content)?
        } else {
            Self::default()
        };

        config.apply_env_overrides();
        config
            .validate()
            .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<(), String> {
        if !self.forges.gitlab_base_url.starts_with("https://") {
            return Err(format!(
                "gitlab_base_url must use HTTPS, got: {}",
                self.forges.gitlab_base_url
            ));
        }
        Ok(())
    }

    pub fn apply_env_overrides(&mut self) {
        if let Ok(token) = std::env::var("FACTO_GITHUB_TOKEN") {
            self.forges.github_token = Some(token);
        }
        if let Ok(token) = std::env::var("FACTO_GITLAB_TOKEN") {
            self.forges.gitlab_token = Some(token);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.timeouts.per_registry_secs, 5);
        assert!(config.registries.enabled.contains(&"pypi".to_string()));
        assert!(config.forges.enabled.contains(&"github".to_string()));
    }

    #[test]
    fn test_env_overrides_tokens() {
        // SAFETY: test runs single-threaded via cargo test -- --test-threads=1
        unsafe {
            std::env::set_var("FACTO_GITHUB_TOKEN", "gh-test-token");
            std::env::set_var("FACTO_GITLAB_TOKEN", "gl-test-token");
        }

        let mut config = Config::default();
        config.apply_env_overrides();

        assert_eq!(
            config.forges.github_token,
            Some("gh-test-token".to_string())
        );
        assert_eq!(
            config.forges.gitlab_token,
            Some("gl-test-token".to_string())
        );

        unsafe {
            std::env::remove_var("FACTO_GITHUB_TOKEN");
            std::env::remove_var("FACTO_GITLAB_TOKEN");
        }
    }

    #[test]
    fn test_debug_redacts_tokens() {
        let config = ForgesConfig {
            enabled: vec!["github".into()],
            github_token: Some("ghp_secret123".to_string()),
            gitlab_token: Some("glpat-secret456".to_string()),
            gitlab_base_url: "https://gitlab.com".to_string(),
        };
        let debug = format!("{:?}", config);
        assert!(
            !debug.contains("ghp_secret123"),
            "GitHub token leaked in Debug output"
        );
        assert!(
            !debug.contains("glpat-secret456"),
            "GitLab token leaked in Debug output"
        );
        assert!(debug.contains("***"), "Tokens should be redacted as ***");
    }

    #[test]
    fn test_parse_partial_toml() {
        let toml_str = r#"
            [timeouts]
            per_registry_secs = 10
        "#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.timeouts.per_registry_secs, 10);
        assert_eq!(config.timeouts.global_secs, 15);
    }

    #[test]
    fn test_gitlab_base_url_must_be_https() {
        let mut config = Config::default();
        config.forges.gitlab_base_url = "http://evil.com".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_gitlab_base_url_allows_https() {
        let mut config = Config::default();
        config.forges.gitlab_base_url = "https://gitlab.example.com".to_string();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_gitlab_base_url_default_is_valid() {
        let config = Config::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_default_config_cache() {
        let config = Config::default();
        assert_eq!(config.cache.package_secs, 900);
    }
}
