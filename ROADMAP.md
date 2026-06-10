# Roadmap

facto stays focused on **live external facts for AI coding agents**.
No UI component libraries, no document processing, no general filesystem
access. This roadmap reflects that scope discipline.

## v0.1 (initial public release)

- Package facts and version history across 9 registries: PyPI, npm,
  crates.io, Go, RubyGems, Maven Central, NuGet, Packagist, Docker Hub
- Package search: registry packages ranked by popularity, plus repository
  search across 3 code forges (GitHub, GitLab, Codeberg)
- Forge release listing for any public repository
- Lockfile audit across 7 formats with local path read (no LLM-side content
  transfer): `uv.lock`, `poetry.lock`, `pdm.lock`, `Pipfile.lock`,
  `package-lock.json`, `pnpm-lock.yaml`, `Cargo.lock`
- Runtime EOL and support status via endoflife.date (14 runtime catalogs)
- GitHub Action pinning to commit SHA (`pin_action`)

## v0.2 (next milestone)

- **Domain / RDAP lookup**: check domain availability, registration data,
  expiration, extending the "external facts" scope to the DNS layer
- Additional registry integrations based on community request

## v0.3 (future)

- **OSV.dev integration**: vulnerability lookup per package version,
  strengthening the supply-chain story
- Deeper lockfile audit: security advisories per dependency

## Explicitly out of scope

facto will not absorb these roles; install dedicated tools for each:

- Library documentation -> [Context7](https://context7.com)
- General filesystem operations -> MCP filesystem server
- PDF / document processing -> dedicated MCP tools
- Chat memory, knowledge graphs, agent orchestration -> specialist tools

The value of facto is being **the best factual layer, nothing more**.

## Non-goals for the foreseeable future

- No proxy / gateway mode (facto is always a local process)
- No persistent storage / database (facto is stateless)
- No write operations on registries or forges (facto is read-only)
- No AI / LLM inside facto (facto *serves* AI agents, it doesn't embed one)
