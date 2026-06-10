# Changelog

All notable changes to facto are documented in this file. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project follows
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0-alpha.1] - 2026-06-08

First public release. facto is a local [MCP](https://modelcontextprotocol.io)
server that gives AI coding agents live lookups across package registries, code
forges, runtime catalogs, and lockfiles, so they stop recommending outdated
packages, deprecated runtimes, or metadata past their training cutoff.

### Added

- MCP server over stdio exposing 14 read-only tools, with structured `isError`
  results (`{error, message, isRetryable}`) and read-only / open-world tool
  annotations.
- Package facts across 9 registries (PyPI, npm, crates.io, Go, RubyGems, Maven,
  NuGet, Packagist, Docker Hub): `get_package`, `get_latest_version`,
  `list_versions`.
- Discovery: `search_packages` (popularity-ranked) and `search_repositories`
  across 3 forges (GitHub, GitLab, Codeberg).
- `pin_action`, resolving a GitHub Action reference to a commit SHA for
  supply-chain pinning.
- `get_runtime_info`, with end-of-life and LTS data across 14+ runtime catalogs
  (Python, Node.js, Go, Rust, Ruby, PHP, Java, .NET, and more).
- Lockfile parsing and auditing across 7 formats (Cargo.lock, package-lock.json,
  pnpm-lock.yaml, Pipfile.lock, poetry.lock, pdm.lock, uv.lock): `parse_lockfile`,
  `check_lockfile`, `discover_lockfiles`.
- `list_forge_releases`, plus catalog discovery via `list_registries`,
  `list_forges`, and `list_runtimes`.
- `facto` binary, distributed on crates.io and PyPI.

[0.1.0-alpha.1]: https://github.com/ggueret/facto/releases/tag/v0.1.0-alpha.1
