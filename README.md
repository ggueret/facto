# facto

**Stale models, fresh data.**

[![CI](https://github.com/ggueret/facto/actions/workflows/ci.yml/badge.svg)](https://github.com/ggueret/facto/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/facto-run.svg)](https://crates.io/crates/facto-run)
[![docs.rs](https://img.shields.io/docsrs/facto-core)](https://docs.rs/facto-core)
[![MSRV](https://img.shields.io/badge/MSRV-1.85-blue.svg)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

---

facto is a local [MCP](https://modelcontextprotocol.io) server that gives
AI coding agents **live lookups** across package registries (9), code
forges (3), runtime catalogs (14+), and lockfiles, so Claude Code,
Cursor, Copilot, or any MCP-capable agent stops recommending outdated
packages, deprecated runtimes, or metadata past its training cutoff.

No proxy, no telemetry. Your tokens, your machine, your data.

## Why facto?

- **Training data goes stale within weeks.** \
  Agents confidently recommend yanked crates, outdated npm packages,
  and deprecated runtimes.
- **Most "latest version" answers are one HTTPS call away.** \
  facto just makes the call for the agent,
  locally, with your tokens.
- **Discovery belongs in the agent loop.** \
  Search packages by popularity or find projects on forges by stars:
  fresh, ranked data rather than a guess from training.
- **Token-efficient by design.** \
  Structured JSON responses, no markdown
  fluff. Lockfiles are read from an absolute path rather than shuttled
  through the LLM context (often 100x fewer tokens than inlining a
  large `pnpm-lock.yaml`).

## Quickstart

Wire `facto mcp` into Claude Code using the **ephemeral uvx invocation**:
no install, auto-cached, always fetches the published version:

```bash
claude mcp add facto -- uvx facto.run mcp
```

That's it. The agent can now call `get_latest_version`, `search_repositories`,
`pin_action`, and [a dozen more tools](docs/tools.md) whenever it needs
current package data.

## Example

```
> User: What's the latest stable fastapi on PyPI?
> Agent (calls get_latest_version): 0.120.3, released 2026-03-28.
>
> User: Pin actions/checkout@v4 for me.
> Agent (calls pin_action): actions/checkout@34e1148... (v4)
>
> User: Find popular Rust HTTP client libraries on GitHub.
> Agent (calls search_repositories): hyper (14.2k stars), reqwest (10.4k stars), ...
```

Without facto, those answers come from the model's training cutoff,
possibly months behind, sometimes just wrong.

## Installation

### Via uvx (recommended for MCP wiring, zero-install)

Requires [uv](https://docs.astral.sh/uv/). Runs `facto` on the fly,
auto-caches by version:

```bash
# Ephemeral: what Claude Code / Cursor / Copilot wiring typically uses
uvx facto.run mcp
```

For a persistent install that puts `facto` on your `PATH`:

```bash
uv tool install facto.run
facto mcp
```

Pin a specific version (either form):

```bash
uvx 'facto.run==0.1.0' mcp
uv tool install 'facto.run==0.1.0'
```

### Via cargo

Requires a [Rust toolchain](https://rustup.rs):

```bash
cargo install facto-run
```

### Pre-built binary

Download the archive for your platform from the
[latest release](https://github.com/ggueret/facto/releases/latest),
extract, and move `facto` into a directory on your `PATH`:

```bash
# Linux x86_64 (adjust URL for your platform)
curl -L https://github.com/ggueret/facto/releases/latest/download/facto-x86_64-unknown-linux-gnu.tar.gz | tar xz
sudo mv facto /usr/local/bin/
```

### From source

```bash
git clone https://github.com/ggueret/facto.git
cd facto
cargo install --path facto-run
```

## How it works

facto runs as a local subprocess launched by your agent (Claude Code,
Cursor, Copilot, ...). They talk over MCP stdio JSON-RPC. facto makes
outbound HTTPS calls to public registries, forges, and runtime catalogs
using the tokens you provide via environment variables.

- **Transport**: stdio JSON-RPC (MCP) between agent and `facto mcp`
- **Upstreams**: 9 package registries, 3 code forges, 14 runtime catalogs
- **Credentials**: `FACTO_GITHUB_TOKEN` / `FACTO_GITLAB_TOKEN` from env only, never logged
- **Safety nets**: 5 s connect / 10 s total timeouts, 50 MB response cap per upstream

The binary never stores state and never calls home. The only files
it reads are (1) lockfile paths the agent explicitly passes to
`parse_lockfile` / `check_lockfile`, and (2) an optional
[`facto.toml`](#optional-factotoml) auto-loaded from the working
directory at launch (or from `$FACTO_CONFIG` if set). Both reads are
documented in [SECURITY.md](SECURITY.md#what-facto-does-on-the-filesystem).

## Privacy

facto runs locally and reads only what you explicitly pass it.

**Lockfile contents stay on your machine.** When you ask facto to audit a
lockfile, the agent passes only the absolute path; facto reads the file
itself and returns structured data. The lockfile body never transits
through the LLM context:

```
Inline approach (content transits LLM):
  lockfile body (~500k tokens) -> LLM context -> MCP -> registry

facto approach (path transits, content stays local):
  path (~30 tokens) -> LLM context -> MCP reads locally -> registry
```

For private monorepos with internal package names, private-registry URLs,
or sensitive dependency hashes, this matters. Your agent still benefits
from the audit without leaking the lockfile through its provider's
infrastructure.

**Token efficiency from the same mechanism.** Keeping lockfiles out of
the LLM context protects privacy *and* slashes token usage. A 2 MB
`pnpm-lock.yaml` weighs about 500 000 tokens, more than the full
context of most models. Via facto, the agent spends ~30 tokens on the
path and receives ~2 000 tokens of structured output. Across tool calls
this compounds: every response is compact JSON, no markdown prose to
reparse.

**Your credentials stay on your machine.** `FACTO_GITHUB_TOKEN` and
other credentials are read once from the environment and used only to
authenticate outbound calls to the relevant registry. They are never
logged, written to disk, or transmitted to any other host.

**No telemetry.** facto makes no calls home. The only outbound traffic is
to the public registries, forges, and runtime catalogs you are actively
querying. See [SECURITY.md](SECURITY.md) for the full threat model.

## Supply-chain hygiene

facto ships two primitives that help keep CI safe:

- **`pin_action`**: resolves a GitHub Action tag (e.g. `v4`) to a full
  commit SHA. Pinning prevents a compromised or remapped tag from
  silently flowing into your workflow.
- **`check_lockfile`**: concurrently audits every resolved dependency in
  a lockfile against the live registry, flagging outdated versions.

facto itself pins every GitHub Action in CI to a full commit SHA; see
[.github/workflows/](.github/workflows/). We plan [OSV.dev](https://osv.dev)
integration in v0.3 for explicit CVE correlation (see [ROADMAP.md](ROADMAP.md)).

## Tools

A high-level overview. See [docs/tools.md](docs/tools.md) for the full
reference with parameters, return shapes, and examples.

### Package facts

| Tool | Description |
|------|-------------|
| `get_latest_version` | Latest stable version of any package |
| `get_package` | Full metadata: description, license, authors, repository URL |
| `list_versions` | All published versions with release dates |

### Search & discovery

| Tool | Description |
|------|-------------|
| `search_packages` | Search packages by keyword; sort by relevance or popularity |
| `search_repositories` | Search forge projects by keyword; sort by stars |

### Lockfiles

| Tool | Description |
|------|-------------|
| `parse_lockfile` | Read a lockfile at an absolute path and return resolved dependencies |
| `check_lockfile` | Same, plus concurrent registry lookups to flag outdated deps |
| `discover_lockfiles` | Filter a list of paths down to recognised lockfiles (no I/O) |

### Runtime

| Tool | Description |
|------|-------------|
| `get_runtime_info` | Runtime version lifecycle, EOL dates, LTS status |

### Forge

| Tool | Description |
|------|-------------|
| `pin_action` | Resolve GitHub Action tag to commit SHA |
| `list_forge_releases` | List releases of a repo on a forge |

### Discovery

| Tool | Description |
|------|-------------|
| `list_registries` | List supported package registries |
| `list_forges` | List supported code forges |
| `list_runtimes` | List supported runtimes |

[Full reference ->](docs/tools.md)

### Supported upstreams

- **Registries**: PyPI, npm, crates.io, Go, RubyGems, Maven, NuGet,
  Packagist, Docker Hub
- **Forges**: GitHub, GitLab, Codeberg
- **Runtimes**: Python, Go, Rust, Node.js, Ruby, PHP, Java, .NET, Deno,
  Bun, Elixir, Kotlin, Perl, Scala
- **Lockfiles**: `uv.lock`, `poetry.lock`, `pdm.lock`, `Pipfile.lock`,
  `package-lock.json`, `pnpm-lock.yaml`, `Cargo.lock`

## Limitations

- **Search is not available on every registry.** `search_packages` works on
  npm, crates.io, RubyGems, Maven, NuGet, Packagist, and Docker Hub. PyPI and Go
  expose no public search API, so `search_packages` returns `not_supported`
  there. A dedicated PyPI search index is planned.

- **`get_latest_version` is not meaningful on Docker Hub.** Docker tags are
  free-form (not semver) and ordered by push date, so there is no reliable
  "latest"; the tool returns `not_supported`. Use `list_versions` to inspect the
  available tags. `get_package`, `search_packages`, and `list_versions` all work
  on Docker Hub.

## Works great with Context7

facto and [Context7](https://context7.com) are complementary pieces of
the AI coding agent ground-truth stack:

| | facto | Context7 |
|---|---|---|
| Question answered | *What's the current state of X?* | *How do I use X?* |
| Output | Version numbers, EOL dates, search results, metadata | Code examples, version-specific API docs |
| Data source | Live calls to registries, forges, runtime catalogs | Crawled and indexed library documentation |

Typical pipeline:

```
Agent: "Add Supabase auth to my Next.js app."
  |- facto.get_latest_version("supabase", "npm")      -> 2.48.1
  |- facto.get_runtime_info("nodejs", "22")           -> LTS, EOL 2027-04
  +- Context7.query-docs("/supabase/supabase", "auth") -> current API
```

Install both for full coverage:

```bash
claude mcp add facto -- uvx facto.run mcp
npx ctx7 setup
```

## Comparison

### Tools in the same category (package / registry MCP)

- [Artmann/package-registry-mcp](https://github.com/Artmann/package-registry-mcp):
  TypeScript, 4 registries, focused scope
- [loonghao/pypi-query-mcp-server](https://github.com/loonghao/pypi-query-mcp-server):
  Python, PyPI-only
- [sammcj/mcp-package-version](https://github.com/sammcj/mcp-package-version):
  Go, archived (superseded by mcp-devtools)
- [sammcj/mcp-devtools](https://github.com/sammcj/mcp-devtools):
  Go, 20+ tool multi-purpose server

facto is the only one covering package registries, code forges, runtime
lifecycle, and lockfile parsing in a single focused Rust binary with
zero-install via `uvx`.

### Tools we complement (not compete)

- [Context7](https://context7.com): version-specific library docs
- [endoflife.date](https://endoflife.date): upstream runtime EOL data
  (facto consumes it)
- [OSV.dev](https://osv.dev): vulnerability database (v0.3 integration planned)

## Configuration

For basic usage, no configuration is required. Unauthenticated calls work
everywhere with lower rate limits.

### Environment variables

```bash
export FACTO_GITHUB_TOKEN="ghp_..."    # GitHub API (higher rate limits, private repos)
export FACTO_GITLAB_TOKEN="glpat-..."  # GitLab API
```

### Optional `facto.toml`

Drop a `facto.toml` next to wherever you launch `facto mcp` (or point
`FACTO_CONFIG` at it) to tune defaults:

```toml
[timeouts]
per_registry_secs = 5
global_secs = 15

[registries]
enabled = ["pypi", "npm", "crates", "go"]  # opt-in subset

[forges]
enabled = ["github"]
gitlab_base_url = "https://gitlab.example.com"  # self-hosted GitLab
```

Tokens given via env variables always override values in the file.

## Upstream sources

facto stands on the shoulders of:

- **Package registries**: PyPI, npm, crates.io, Go Proxy, RubyGems, Maven
  Central, NuGet, Packagist, Docker Hub
- **Code forges**: GitHub, GitLab, Codeberg
- **Runtime lifecycle data**: [endoflife.date](https://endoflife.date),
  community-maintained, indispensable, consider supporting them
- **Protocol**: [Model Context Protocol](https://modelcontextprotocol.io)
  by Anthropic

## Crates

| Crate | Role |
|-------|------|
| [facto-core](facto-core/) | Shared library: models, registry / forge / runtime / lockfile clients |
| [facto-mcp](facto-mcp/) | MCP server library: `FactoMcp` struct, rmcp tool router, `serve_stdio()` |
| [facto-run](facto-run/) | Thin CLI binary that produces the `facto` executable |

## Development

```bash
git clone https://github.com/ggueret/facto.git
cd facto
make help               # list all available targets
make check              # fmt + clippy (fast)
make test               # unit tests (no network)
make test-integration   # full integration (calls real APIs, needs network)
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for details.
