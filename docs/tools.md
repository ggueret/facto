# Tools Reference

facto exposes the following tools over MCP. All tools are callable by any
MCP-compatible agent (Claude Code, Cursor, Copilot, ...) and return
structured JSON.

Unless explicitly noted, every tool is **read-only**, **stateless**, and
makes at most one outbound HTTPS call per upstream queried.

- [Package intelligence](#package-intelligence)
  - [`get_package`](#get_package)
  - [`get_latest_version`](#get_latest_version)
  - [`list_versions`](#list_versions)
- [Search & discovery](#search--discovery)
  - [`search_packages`](#search_packages)
  - [`search_repositories`](#search_repositories)
- [Lockfile auditing](#lockfile-auditing)
  - [`parse_lockfile`](#parse_lockfile)
  - [`check_lockfile`](#check_lockfile)
  - [`discover_lockfiles`](#discover_lockfiles)
- [Runtime lifecycle](#runtime-lifecycle)
  - [`get_runtime_info`](#get_runtime_info)
- [Forge](#forge)
  - [`pin_action`](#pin_action)
  - [`list_forge_releases`](#list_forge_releases)
- [Discovery](#discovery)
  - [`list_registries`](#list_registries)
  - [`list_forges`](#list_forges)
  - [`list_runtimes`](#list_runtimes)

---

## Package intelligence

### `get_package`

Full metadata for a package: description, license, authors, homepage,
repository URL, latest version. Returns a `PackageLookup` envelope so the
caller can distinguish "package exists" from "name is free on this registry"
without relying on an error path.

**Parameters**

| Name | Type | Required | Description |
|------|------|:--:|-------------|
| `name` | string | yes | Package name |
| `registry` | string | yes | Registry ID (see [`list_registries`](#list_registries)) |

**Returns**: a `PackageLookup` object:

```
{
  "found": true | false,
  "name": "<name>",
  "registry": "<registry>",
  "package": { ... }   // present only when found is true
}
```

When `found` is `true`, `package` contains a `PackageInfo` object with
`name`, `registry`, and the following optional fields (omitted when absent):
`latest_version`, `description`, `license`, `homepage`, `repository`,
`authors`, `updated_at` (RFC 3339 UTC), `keywords`, `classifiers`,
`requires_python`.

When `found` is `false`, `package` is omitted entirely. This indicates the
name does not exist on that registry (i.e. it is free).

**Example: found**

```json
{
  "tool": "get_package",
  "arguments": { "name": "requests", "registry": "pypi" }
}
```

```json
{
  "found": true,
  "name": "requests",
  "registry": "pypi",
  "package": {
    "name": "requests",
    "registry": "pypi",
    "latest_version": "2.32.5",
    "description": "Python HTTP for Humans.",
    "license": "Apache-2.0",
    "homepage": "https://requests.readthedocs.io",
    "repository": "https://github.com/psf/requests",
    "authors": ["Kenneth Reitz"],
    "keywords": ["http", "requests"]
  }
}
```

**Example: not found**

```json
{
  "tool": "get_package",
  "arguments": { "name": "totally-free-name-xyz", "registry": "pypi" }
}
```

```json
{
  "found": false,
  "name": "totally-free-name-xyz",
  "registry": "pypi"
}
```

---

### `get_latest_version`

Return the latest stable version of a package on a given registry.

**Parameters**

| Name | Type | Required | Description |
|------|------|:--:|-------------|
| `name` | string | yes | Package name as published on the registry |
| `registry` | string | yes | Registry ID (see [`list_registries`](#list_registries)) |

**Returns**: a `VersionInfo` object: `{ version, released_at, prerelease }`.
`released_at` is RFC 3339 UTC and may be omitted if the registry does not
expose a release date. `prerelease` is always `false` for this tool (only
stable versions are returned). Returns the literal string
`"no stable version found"` if every published version is a prerelease.

**Example**

```json
{
  "tool": "get_latest_version",
  "arguments": { "name": "fastapi", "registry": "pypi" }
}
```

```json
{
  "version": "0.120.3",
  "released_at": "2026-03-28T14:22:10Z",
  "prerelease": false
}
```

---

### `list_versions`

All published versions of a package with release dates and prerelease flags,
sorted newest first.

**Parameters**

| Name | Type | Required | Description |
|------|------|:--:|-------------|
| `name` | string | yes | Package name |
| `registry` | string | yes | Registry ID |
| `stable_only` | bool | no | Filter out prereleases. Defaults to `false` |

**Returns**: array of `VersionInfo` objects: `{ version, released_at, prerelease }`.
`released_at` is omitted when the registry does not expose a release date.

---

## Search & discovery

### `search_packages`

Search packages by keyword on a given registry. Returns hits with download
counts and keywords where the registry provides them.

**Parameters**

| Name | Type | Required | Description |
|------|------|:--:|-------------|
| `query` | string | yes | Free-text search query |
| `registry` | string | yes | Registry ID |
| `sort` | string | no | `"relevance"` (default) or `"popularity"` (by downloads) |
| `limit` | integer | no | Max results to return (default 20, max 100) |

**Returns**: array of `SearchResult` objects. Each entry always carries
`name`; the following fields are omitted when absent:
`description`, `latest_version`, `downloads`, `keywords`.

`downloads` is registry-relative: lifetime total for most registries,
monthly for npm. When `sort` is `"popularity"`, results are ordered by
`downloads` descending; entries without a download count appear last.

**Example**

```json
{
  "tool": "search_packages",
  "arguments": { "query": "http client", "registry": "pypi", "sort": "popularity", "limit": 3 }
}
```

```json
[
  {
    "name": "requests",
    "description": "Python HTTP for Humans.",
    "latest_version": "2.32.5",
    "downloads": 1200000000,
    "keywords": ["http", "requests"]
  },
  {
    "name": "httpx",
    "description": "The next generation HTTP client.",
    "latest_version": "0.28.1",
    "downloads": 300000000
  },
  {
    "name": "urllib3",
    "latest_version": "2.4.0",
    "downloads": 2500000000
  }
]
```

---

### `search_repositories`

Search for repositories/projects on a code forge by keyword.

**Parameters**

| Name | Type | Required | Description |
|------|------|:--:|-------------|
| `query` | string | yes | Free-text search query |
| `forge` | string | yes | Forge ID (see [`list_forges`](#list_forges)) |
| `sort` | string | no | `"stars"` (default), `"updated"`, or `"relevance"` |
| `language` | string | no | Best-effort language filter (honoured by GitHub; ignored by other forges) |
| `limit` | integer | no | Max results to return (default 20, max 100) |

**Returns**: array of `RepositorySearchResult` objects. Each entry always
carries `forge`, `full_name`, `owner`, `name`, `stars`, and `url`;
the following fields are omitted when absent:
`description`, `language`, `topics`, `updated_at`.

**Example**

```json
{
  "tool": "search_repositories",
  "arguments": { "query": "http client", "forge": "github", "language": "rust", "limit": 2 }
}
```

```json
[
  {
    "forge": "github",
    "full_name": "hyperium/hyper",
    "owner": "hyperium",
    "name": "hyper",
    "description": "An HTTP library for Rust",
    "stars": 14200,
    "language": "Rust",
    "topics": ["http", "rust", "async"],
    "url": "https://github.com/hyperium/hyper",
    "updated_at": "2026-05-10T08:30:00Z"
  },
  {
    "forge": "github",
    "full_name": "seanmonstar/reqwest",
    "owner": "seanmonstar",
    "name": "reqwest",
    "description": "An easy and powerful Rust HTTP Client",
    "stars": 10400,
    "language": "Rust",
    "url": "https://github.com/seanmonstar/reqwest",
    "updated_at": "2026-04-22T11:00:00Z"
  }
]
```

---

## Lockfile auditing

facto reads lockfile contents **locally** when the agent passes an absolute
path. The lockfile body never transits through the LLM context.
See [Privacy](../README.md#privacy) in the README.

### `parse_lockfile`

Parse a lockfile at a given filesystem path and return the resolved
dependencies (name, version, registry, group). No network calls.

**Parameters**

| Name | Type | Required | Description |
|------|------|:--:|-------------|
| `path` | string | yes | Absolute path to a recognised lockfile |

Supported basenames: `uv.lock`, `poetry.lock`, `pdm.lock`, `Pipfile.lock`,
`package-lock.json`, `pnpm-lock.yaml`, `Cargo.lock`.

**Returns**: `{ format, deps, warnings }`. `deps` is an array of
`ResolvedDep` objects: `{ name, version, registry, group }`.
`group` is a tagged object: `{ "kind": "main" }`, `{ "kind": "dev" }`,
`{ "kind": "build" }`, `{ "kind": "optional", "name": "<group>" }`, or
`{ "kind": "unknown" }`.

---

### `check_lockfile`

Same as `parse_lockfile`, plus a concurrent registry lookup to flag
outdated dependencies. Hits the upstream registries (pypi, npm, crates)
directly, no intermediate service.

**Parameters**

| Name | Type | Required | Description |
|------|------|:--:|-------------|
| `path` | string | yes | Absolute path to a recognised lockfile |

**Returns**: `{ format, deps, warnings }`. Each dep is a `CheckedDep`
object. The fields `name`, `current`, `registry`, and `group` are always
present; the following fields are omitted when absent:
`latest` (omitted if the lookup failed), `outdated` (omitted if `latest`
is absent), `error` (omitted when the lookup succeeded).

**Example**

```json
{
  "tool": "check_lockfile",
  "arguments": { "path": "/home/user/myproject/uv.lock" }
}
```

```json
{
  "format": "uv_lock",
  "deps": [
    {
      "name": "fastapi",
      "current": "0.100.0",
      "latest": "0.120.3",
      "outdated": true,
      "registry": "pypi",
      "group": { "kind": "main" }
    },
    {
      "name": "httpx",
      "current": "0.28.1",
      "latest": "0.28.1",
      "outdated": false,
      "registry": "pypi",
      "group": { "kind": "main" }
    },
    {
      "name": "some-private-pkg",
      "current": "1.0.0",
      "registry": "pypi",
      "group": { "kind": "dev" },
      "error": "404 Not Found"
    }
  ],
  "warnings": []
}
```

---

### `discover_lockfiles`

Filter a list of paths down to the subset that match a known lockfile
filename, tagged with format and registry. Does no I/O: classification is
by basename only, typically fed from a client-side filesystem glob.

**Parameters**

| Name | Type | Required | Description |
|------|------|:--:|-------------|
| `paths` | array of strings | yes | Paths to classify (max 1024) |

**Returns**: `{ lockfiles: [{ path, format, registry }] }`.

---

## Runtime lifecycle

### `get_runtime_info`

Lifecycle information for a language runtime: versions, EOL dates, LTS
status. Data sourced from [endoflife.date](https://endoflife.date).

**Parameters**

| Name | Type | Required | Description |
|------|------|:--:|-------------|
| `runtime` | string | yes | Runtime ID (see [`list_runtimes`](#list_runtimes)) |

**Returns**: a `RuntimeInfo` object:

```
{
  "id": "python",
  "name": "Python",
  "latest_stable": "3.13.2",
  "latest_cycle": "3.13",
  "changelog_url": "https://...",        // optional
  "versions": [
    {
      "cycle": "3.13",
      "latest": "3.13.2",
      "release_date": "2024-10-07",       // optional, YYYY-MM-DD
      "latest_release_date": "2025-02-04",// optional, YYYY-MM-DD
      "eol": "2029-10",                   // date or boolean
      "lts": false,
      "support": "2027-04"                // optional, date or boolean
    }
  ]
}
```

`eol` and `support` are untagged: either a `YYYY-MM-DD` date string
or a boolean (`true` = EOL reached, `false` = still supported).

---

## Forge

### `pin_action`

Resolve a GitHub Action reference (tag or branch) to a full commit SHA.
Essential for supply-chain-safe workflows: pinning to a SHA prevents tag
remapping attacks.

**Parameters**

| Name | Type | Required | Description |
|------|------|:--:|-------------|
| `action` | string | yes | Action identifier in `owner/repo` format, e.g. `actions/checkout` |
| `tag` | string | no | Tag or version to resolve, e.g. `v4`, `v4.2.0`. Defaults to the latest stable release |

**Returns**: an `ActionPin` object: `{ pinned, action, tag, commit_sha, url }`.
`pinned` is the ready-to-paste pinned reference (`owner/repo@sha # tag`),
`commit_sha` is the resolved commit SHA, and `url` (optional, omitted when
unavailable) points to the corresponding commit page on the forge.

**Example**

```json
{
  "tool": "pin_action",
  "arguments": { "action": "actions/checkout", "tag": "v4" }
}
```

```json
{
  "pinned": "actions/checkout@34e114876b0b11c390a56381ad16ebd13914f8d5 # v4",
  "action": "actions/checkout",
  "tag": "v4",
  "commit_sha": "34e114876b0b11c390a56381ad16ebd13914f8d5",
  "url": "https://github.com/actions/checkout/commit/34e114876b0b11c390a56381ad16ebd13914f8d5"
}
```

---

### `list_forge_releases`

List releases of a repository on a code forge with tags, dates,
prerelease/draft flags, and downloadable assets.

**Parameters**

| Name | Type | Required | Description |
|------|------|:--:|-------------|
| `owner` | string | yes | Repository owner |
| `repo` | string | yes | Repository name |
| `forge` | string | yes | Forge ID (see [`list_forges`](#list_forges)) |
| `limit` | integer | no | Max releases (default 20, max 100) |

**Returns**: array of `ReleaseInfo` objects. Each entry carries `tag`,
`prerelease`, `draft`, and `html_url`; `name` and `published_at` (RFC 3339
UTC) are omitted when absent; `assets` is omitted when empty. Each asset
carries `name`, `size`, `download_url`, `download_count`, and
`content_type`.

---

## Discovery

### `list_registries`

List all supported package registry IDs.

**Parameters**: none.

**Returns**: array of `{ id, name }`.

---

### `list_forges`

List all supported code forge IDs.

**Parameters**: none.

**Returns**: array of `{ id, name }`.

---

### `list_runtimes`

List all supported runtime catalog IDs.

**Parameters**: none.

**Returns**: array of `{ id, name }`.

---

## Error handling

Every tool returns JSON-RPC errors with structured `McpError` messages on:

- Invalid parameters (missing required field, wrong type, invalid enum)
- Unknown registry / forge / runtime ID
- Upstream timeout (5 s connect, 10 s total)
- Upstream HTTP error (propagated with status code)
- Response exceeding the 50 MB cap

For `parse_lockfile` and `check_lockfile`, additional validation errors:

- Path is relative (must be absolute)
- Path contains null bytes
- Basename does not match a recognised lockfile
- Path is a symlink (rejected by design)
- File exceeds the 4 MiB cap
- File content is not valid UTF-8
