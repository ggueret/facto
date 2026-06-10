//! Lockfile parsing for supported ecosystems.
//!
//! The public API is three functions: [`detect_format`] to identify a
//! lockfile by its filename, [`parse`] to extract dependencies from its
//! content given a known format, and [`detect_and_parse`] as a convenience
//! that combines the two.
//!
//! facto-core never performs filesystem I/O. Callers are responsible for
//! reading the file and passing its content as a string.

pub mod models;
mod npm;
mod pipfile;
mod pnpm;
mod toml_common;

pub use models::{
    CheckedDep, DepGroup, LockfileFormat, ParseError, ParseResult, ParseWarning, ResolvedDep,
};

/// Detect the lockfile format from a filename or file path.
///
/// Returns `None` when the filename does not match any known lockfile.
/// The input may be a bare filename or a path; only the basename is used.
/// Matching is case-sensitive: `Cargo.lock` matches but `cargo.lock` does not.
pub fn detect_format(filename: &str) -> Option<LockfileFormat> {
    let base = match filename.rfind(['/', '\\']) {
        Some(i) => &filename[i + 1..],
        None => filename,
    };
    match base {
        "uv.lock" => Some(LockfileFormat::UvLock),
        "poetry.lock" => Some(LockfileFormat::PoetryLock),
        "pdm.lock" => Some(LockfileFormat::PdmLock),
        "Pipfile.lock" => Some(LockfileFormat::PipfileLock),
        "package-lock.json" => Some(LockfileFormat::NpmLock),
        "pnpm-lock.yaml" => Some(LockfileFormat::PnpmLock),
        "Cargo.lock" => Some(LockfileFormat::CargoLock),
        _ => None,
    }
}

/// Parse a lockfile's content given an already-known format.
///
/// Individual malformed entries become `ParseWarning`s; the returned
/// `ParseResult` still contains the successfully extracted deps. A fatal
/// error (e.g. invalid TOML/JSON) causes a panic-free empty result with a
/// single warning. Use [`detect_and_parse`] if you want a `Result` on
/// structural failures instead.
pub fn parse(content: &str, format: LockfileFormat) -> ParseResult {
    match format {
        LockfileFormat::UvLock => toml_common::parse_uv(content),
        LockfileFormat::PoetryLock => toml_common::parse_poetry(content),
        LockfileFormat::PdmLock => toml_common::parse_pdm(content),
        LockfileFormat::CargoLock => toml_common::parse_cargo(content),
        LockfileFormat::PipfileLock => pipfile::parse(content),
        LockfileFormat::NpmLock => npm::parse(content),
        LockfileFormat::PnpmLock => pnpm::parse(content),
    }
}

/// Detect the format from `filename` and parse `content`.
///
/// Returns `Err(ParseError::UnknownFormat(..))` if the filename is not
/// recognised. Structural parse errors surface via the inner parser and
/// are reported as warnings on the returned `ParseResult`.
pub fn detect_and_parse(filename: &str, content: &str) -> Result<ParseResult, ParseError> {
    let format =
        detect_format(filename).ok_or_else(|| ParseError::UnknownFormat(filename.to_string()))?;
    Ok(parse(content, format))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_format_python() {
        assert_eq!(detect_format("uv.lock"), Some(LockfileFormat::UvLock));
        assert_eq!(
            detect_format("poetry.lock"),
            Some(LockfileFormat::PoetryLock)
        );
        assert_eq!(detect_format("pdm.lock"), Some(LockfileFormat::PdmLock));
        assert_eq!(
            detect_format("Pipfile.lock"),
            Some(LockfileFormat::PipfileLock)
        );
    }

    #[test]
    fn test_detect_format_node() {
        assert_eq!(
            detect_format("package-lock.json"),
            Some(LockfileFormat::NpmLock)
        );
        assert_eq!(
            detect_format("pnpm-lock.yaml"),
            Some(LockfileFormat::PnpmLock)
        );
    }

    #[test]
    fn test_detect_format_rust() {
        assert_eq!(detect_format("Cargo.lock"), Some(LockfileFormat::CargoLock));
    }

    #[test]
    fn test_detect_format_handles_paths() {
        assert_eq!(
            detect_format("/tmp/project/uv.lock"),
            Some(LockfileFormat::UvLock)
        );
        assert_eq!(
            detect_format("subdir/Cargo.lock"),
            Some(LockfileFormat::CargoLock)
        );
    }

    #[test]
    fn test_detect_format_unknown() {
        assert_eq!(detect_format("requirements.txt"), None);
        assert_eq!(detect_format("pyproject.toml"), None);
        assert_eq!(detect_format(""), None);
        // Matching is case-sensitive: lowercase Cargo.lock must NOT match.
        assert_eq!(detect_format("cargo.lock"), None);
    }

    #[test]
    fn test_parse_returns_format_in_result() {
        let result = parse("", LockfileFormat::UvLock);
        assert_eq!(result.format, LockfileFormat::UvLock);
    }

    #[test]
    fn test_detect_and_parse_unknown_returns_error() {
        let err = detect_and_parse("unknown.file", "").unwrap_err();
        assert!(matches!(err, ParseError::UnknownFormat(_)));
    }

    #[test]
    fn test_detect_and_parse_known_format_ok() {
        let result = detect_and_parse("uv.lock", "").unwrap();
        assert_eq!(result.format, LockfileFormat::UvLock);
    }

    const UV_LOCK_SAMPLE: &str = r#"
version = 1
requires-python = ">=3.11"

[[package]]
name = "fastapi"
version = "0.115.8"
source = { registry = "https://pypi.org/simple" }

[[package]]
name = "pydantic"
version = "2.10.6"
source = { registry = "https://pypi.org/simple" }

[[package]]
name = "pytest"
version = "8.3.5"
source = { registry = "https://pypi.org/simple" }
"#;

    #[test]
    fn test_parse_uv_lock_extracts_all_packages() {
        let result = parse(UV_LOCK_SAMPLE, LockfileFormat::UvLock);
        assert_eq!(result.format, LockfileFormat::UvLock);
        assert_eq!(result.deps.len(), 3);
        assert_eq!(result.warnings, vec![]);

        let fastapi = &result.deps[0];
        assert_eq!(fastapi.name, "fastapi");
        assert_eq!(fastapi.version, "0.115.8");
        assert_eq!(fastapi.registry, "pypi");
        assert_eq!(fastapi.group, DepGroup::Main);
    }

    #[test]
    fn test_parse_uv_lock_invalid_toml_yields_warning() {
        let result = parse("not valid = = toml", LockfileFormat::UvLock);
        assert!(result.deps.is_empty());
        assert_eq!(result.warnings.len(), 1);
        assert!(result.warnings[0].message.contains("invalid TOML"));
    }

    #[test]
    fn test_parse_uv_lock_package_missing_version_skipped() {
        let sample = r#"
[[package]]
name = "brokenpkg"
"#;
        let result = parse(sample, LockfileFormat::UvLock);
        assert!(result.deps.is_empty());
        assert_eq!(result.warnings.len(), 1);
        assert!(result.warnings[0].message.contains("no version"));
    }

    const POETRY_LOCK_SAMPLE: &str = r#"
[[package]]
name = "fastapi"
version = "0.115.8"
description = "FastAPI framework"
category = "main"
optional = false
python-versions = ">=3.8"

[[package]]
name = "pytest"
version = "8.3.5"
description = "pytest: simple powerful testing"
category = "dev"
optional = false
python-versions = ">=3.8"

[[package]]
name = "black"
version = "24.10.0"
description = "Code formatter"
category = "dev"
optional = false
python-versions = ">=3.9"
"#;

    #[test]
    fn test_parse_poetry_lock_distinguishes_main_and_dev() {
        let result = parse(POETRY_LOCK_SAMPLE, LockfileFormat::PoetryLock);
        assert_eq!(result.format, LockfileFormat::PoetryLock);
        assert_eq!(result.deps.len(), 3);

        assert_eq!(result.deps[0].name, "fastapi");
        assert_eq!(result.deps[0].group, DepGroup::Main);
        assert_eq!(result.deps[0].registry, "pypi");

        assert_eq!(result.deps[1].name, "pytest");
        assert_eq!(result.deps[1].group, DepGroup::Dev);

        assert_eq!(result.deps[2].name, "black");
        assert_eq!(result.deps[2].group, DepGroup::Dev);
    }

    #[test]
    fn test_parse_poetry_lock_v2_without_category_defaults_to_main() {
        let sample = r#"
[[package]]
name = "requests"
version = "2.32.3"
"#;
        let result = parse(sample, LockfileFormat::PoetryLock);
        assert_eq!(result.deps.len(), 1);
        assert_eq!(result.deps[0].name, "requests");
        assert_eq!(result.deps[0].group, DepGroup::Main);
        assert_eq!(result.deps[0].registry, "pypi");
    }

    const PDM_LOCK_SAMPLE: &str = r#"
[[package]]
name = "fastapi"
version = "0.115.8"
summary = "FastAPI framework"
groups = ["default"]

[[package]]
name = "pytest"
version = "8.3.5"
summary = "pytest testing"
groups = ["dev"]

[[package]]
name = "shared"
version = "1.0.0"
summary = "shared between default and dev"
groups = ["default", "dev"]
"#;

    #[test]
    fn test_parse_pdm_lock_uses_groups_field() {
        let result = parse(PDM_LOCK_SAMPLE, LockfileFormat::PdmLock);
        assert_eq!(result.format, LockfileFormat::PdmLock);
        assert_eq!(result.deps.len(), 3);

        assert_eq!(result.deps[0].name, "fastapi");
        assert_eq!(result.deps[0].group, DepGroup::Main);

        assert_eq!(result.deps[1].name, "pytest");
        assert_eq!(result.deps[1].group, DepGroup::Dev);

        // A dep in both "default" and "dev" is Main (default wins).
        assert_eq!(result.deps[2].name, "shared");
        assert_eq!(result.deps[2].group, DepGroup::Main);
    }

    const CARGO_LOCK_SAMPLE: &str = r#"
version = 3

[[package]]
name = "serde"
version = "1.0.210"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "fake_checksum"

[[package]]
name = "local-crate"
version = "0.1.0"

[[package]]
name = "tokio"
version = "1.44.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "other_checksum"
"#;

    #[test]
    fn test_parse_cargo_lock_skips_path_dependencies() {
        let result = parse(CARGO_LOCK_SAMPLE, LockfileFormat::CargoLock);
        assert_eq!(result.format, LockfileFormat::CargoLock);
        // local-crate (no source) is a workspace/path dep and gets skipped.
        assert_eq!(result.deps.len(), 2);
        assert_eq!(result.deps[0].name, "serde");
        assert_eq!(result.deps[0].version, "1.0.210");
        assert_eq!(result.deps[0].registry, "crates");
        assert_eq!(result.deps[0].group, DepGroup::Unknown);

        assert_eq!(result.deps[1].name, "tokio");
        assert_eq!(result.deps[1].group, DepGroup::Unknown);

        // One warning for the skipped local-crate entry.
        assert_eq!(result.warnings.len(), 1);
        assert!(result.warnings[0].message.contains("local-crate"));
    }

    const PIPFILE_LOCK_SAMPLE: &str = r#"{
    "_meta": {
        "hash": {"sha256": "abc"},
        "pipfile-spec": 6,
        "requires": {"python_version": "3.11"}
    },
    "default": {
        "fastapi": {
            "version": "==0.115.8",
            "hashes": ["sha256:abc"]
        },
        "pydantic": {
            "version": "==2.10.6",
            "hashes": ["sha256:def"]
        }
    },
    "develop": {
        "pytest": {
            "version": "==8.3.5",
            "hashes": ["sha256:ghi"]
        }
    }
}"#;

    #[test]
    fn test_parse_pipfile_lock_splits_default_and_develop() {
        let result = parse(PIPFILE_LOCK_SAMPLE, LockfileFormat::PipfileLock);
        assert_eq!(result.format, LockfileFormat::PipfileLock);
        assert_eq!(result.deps.len(), 3);

        // Order preserved: default first, then develop.
        let fastapi = result.deps.iter().find(|d| d.name == "fastapi").unwrap();
        assert_eq!(fastapi.version, "0.115.8");
        assert_eq!(fastapi.registry, "pypi");
        assert_eq!(fastapi.group, DepGroup::Main);

        let pytest = result.deps.iter().find(|d| d.name == "pytest").unwrap();
        assert_eq!(pytest.version, "8.3.5");
        assert_eq!(pytest.group, DepGroup::Dev);
    }

    #[test]
    fn test_parse_pipfile_lock_invalid_json_warning() {
        let result = parse("{not json", LockfileFormat::PipfileLock);
        assert!(result.deps.is_empty());
        assert_eq!(result.warnings.len(), 1);
        assert!(result.warnings[0].message.contains("invalid JSON"));
    }

    const PACKAGE_LOCK_SAMPLE: &str = r#"{
    "name": "myproject",
    "version": "1.0.0",
    "lockfileVersion": 3,
    "requires": true,
    "packages": {
        "": {
            "name": "myproject",
            "version": "1.0.0",
            "dependencies": {
                "express": "^4.18.0"
            }
        },
        "node_modules/express": {
            "version": "4.21.2",
            "resolved": "https://registry.npmjs.org/express/-/express-4.21.2.tgz",
            "integrity": "sha512-fake"
        },
        "node_modules/typescript": {
            "version": "5.4.5",
            "resolved": "https://registry.npmjs.org/typescript/-/typescript-5.4.5.tgz",
            "integrity": "sha512-fake",
            "dev": true
        }
    }
}"#;

    #[test]
    fn test_parse_package_lock_extracts_non_root_entries() {
        let result = parse(PACKAGE_LOCK_SAMPLE, LockfileFormat::NpmLock);
        assert_eq!(result.format, LockfileFormat::NpmLock);
        assert_eq!(result.deps.len(), 2);

        let express = result.deps.iter().find(|d| d.name == "express").unwrap();
        assert_eq!(express.version, "4.21.2");
        assert_eq!(express.registry, "npm");
        assert_eq!(express.group, DepGroup::Main);

        let ts = result.deps.iter().find(|d| d.name == "typescript").unwrap();
        assert_eq!(ts.group, DepGroup::Dev);
    }

    #[test]
    fn test_parse_package_lock_scoped_package() {
        let sample = r#"{
    "lockfileVersion": 3,
    "packages": {
        "node_modules/@types/node": {
            "version": "20.11.0",
            "dev": true
        }
    }
}"#;
        let result = parse(sample, LockfileFormat::NpmLock);
        assert_eq!(result.deps.len(), 1);
        assert_eq!(result.deps[0].name, "@types/node");
        assert_eq!(result.deps[0].group, DepGroup::Dev);
    }

    const PNPM_LOCK_SAMPLE: &str = r#"lockfileVersion: '9.0'

settings:
  autoInstallPeers: true

importers:
  .:
    dependencies:
      express:
        specifier: ^4.18.0
        version: 4.21.2
    devDependencies:
      typescript:
        specifier: ^5.4.0
        version: 5.4.5
      '@types/node':
        specifier: ^20.11.0
        version: 20.11.0

packages:

  express@4.21.2:
    resolution: {integrity: sha512-fake}
    engines: {node: '>=0.10.0'}

  typescript@5.4.5:
    resolution: {integrity: sha512-fake}
    engines: {node: '>=14.17'}

  '@types/node@20.11.0':
    resolution: {integrity: sha512-fake}

snapshots:

  express@4.21.2: {}

  typescript@5.4.5: {}

  '@types/node@20.11.0': {}
"#;

    #[test]
    fn test_parse_pnpm_lock_extracts_packages_as_unknown_group() {
        let result = parse(PNPM_LOCK_SAMPLE, LockfileFormat::PnpmLock);
        assert_eq!(result.format, LockfileFormat::PnpmLock);
        assert_eq!(result.deps.len(), 3);

        let express = result.deps.iter().find(|d| d.name == "express").unwrap();
        assert_eq!(express.version, "4.21.2");
        assert_eq!(express.registry, "npm");
        // pnpm v9+ no longer records dev/main on individual package
        // entries -- that context lives under `importers`. For v1 we
        // report every dep as Unknown rather than parsing importers.
        assert_eq!(express.group, DepGroup::Unknown);

        let ts = result.deps.iter().find(|d| d.name == "typescript").unwrap();
        assert_eq!(ts.group, DepGroup::Unknown);

        let types_node = result
            .deps
            .iter()
            .find(|d| d.name == "@types/node")
            .unwrap();
        assert_eq!(types_node.version, "20.11.0");
        assert_eq!(types_node.group, DepGroup::Unknown);
    }

    #[test]
    fn test_parse_pnpm_lock_invalid_yaml_yields_warning() {
        // Unterminated flow sequence is unambiguously invalid YAML.
        let result = parse("[1, 2, 3", LockfileFormat::PnpmLock);
        assert!(result.deps.is_empty());
        assert_eq!(result.warnings.len(), 1);
        assert!(result.warnings[0].message.contains("invalid YAML"));
    }
}
