# Security Policy

## Supported Versions

The latest minor release is supported with security fixes. Older versions
receive fixes only for critical vulnerabilities.

## Reporting a Vulnerability

Email: **security@facto.run**

Do NOT open a public issue for security vulnerabilities. You should receive
an acknowledgement within 48 hours and a remediation plan within 7 days for
confirmed reports.

## Scope

In scope:

- `facto-core`, `facto-mcp`, `facto-run` (this repository)
- GitHub Actions workflows in `.github/workflows/`

Out of scope:

- Vulnerabilities in upstream registries / forges that facto queries
- Third-party MCP clients (Claude Code, Cursor, Copilot, ...)
- Issues in `rmcp`, `reqwest`, or other direct dependencies (report upstream,
  we will pick up the fix)

## Threat Model

### What facto does on the filesystem

facto touches the local filesystem in exactly two places:

#### 1. Lockfile reads (agent-driven, per-call)

facto reads a lockfile **only** when the MCP client explicitly passes an
absolute path via `parse_lockfile` or `check_lockfile`. The lockfile
basename must match a recognised filename (`uv.lock`, `Cargo.lock`,
`pnpm-lock.yaml`, etc.).

Enforced guardrails:

- Path must be absolute; relative paths are rejected (facto has no concept
  of "current project")
- Null bytes in the path are rejected
- Basename must match a whitelisted lockfile filename
- The final path component must be a regular file; symlinks, directories,
  FIFOs, sockets and devices are rejected. `symlink_metadata` is used
  rather than `metadata` so that a symlink is detected as a symlink and
  not silently followed to its target. Prevents an attacker with write
  access to a queried directory from planting a malicious symlink and
  exfiltrating arbitrary files via the parser error path.
- File size is checked via `symlink_metadata` before reading (4 MiB cap),
  so an attacker pointing at a 100 GiB file cannot DoS the process
- Content must be valid UTF-8 (non-UTF-8 rejected)

#### 2. Config read (one-shot, at process startup)

At launch, `facto mcp` reads an optional `facto.toml` config file:

- From the path in `$FACTO_CONFIG` if that environment variable is set
- Otherwise from `facto.toml` in the working directory the binary was
  launched from
- If neither file exists, defaults are used and no read occurs

The file is read once at startup and parsed via `serde` + `toml`. If the
file is missing, the default config is used silently. If parsing fails,
the process exits before serving any MCP request. The config file
controls cache TTLs, request timeouts, the registry/forge/runtime allow
lists, and a `gitlab_base_url` (which must be HTTPS or startup is
rejected). Tokens supplied via environment variables (`FACTO_GITHUB_TOKEN`,
`FACTO_GITLAB_TOKEN`) always override values in the file.

There is no other filesystem access: no recursive scanning, no caching
to disk, no log file, no write operations.

### What facto does on the network

facto makes outbound HTTPS calls to package registries, code forges, and
runtime catalogs. Users provide their own tokens via environment variables
(`FACTO_GITHUB_TOKEN`, `FACTO_GITLAB_TOKEN`). Tokens are:

- Read once at process startup from the environment
- Used only to authenticate outbound calls to the relevant upstream host
- Never logged, written to disk, or transmitted to any other destination

Response-level guardrails:

- Aggressive timeouts: 5 s connect, 10 s total per request
- Response size capped at 50 MB per upstream response, to prevent a
  compromised upstream from OOM-ing the editor hosting facto
- No response is persisted to disk

### What facto does NOT do

- No telemetry, analytics, or phone-home of any kind
- No filesystem exploration, auto-discovery, or directory scanning
- No LLM-side content transfer of lockfile bodies (paths only)
- No local network scanning or port probing
- No privileged operations or sudo escalation
- No write operations on registries, forges, or the local filesystem

## Supply-Chain Hardening

facto is itself an object of supply-chain concern, since it advises agents
about supply-chain safety. Hardening measures in place:

- All GitHub Actions in CI are pinned to full commit SHAs (dogfooding the
  `pin_action` tool)
- `cargo-audit` and `cargo-deny` run on every PR and on main
- `deny.toml` enforces the license allowlist, yanked-crate denial, and
  single-source policy
- No `unsafe` blocks in production code (tests may use `unsafe` for
  environment-variable manipulation only)
- MSRV is pinned at Rust 1.85 and checked in CI
- Release binaries are built in a clean CI environment, not from local
  developer machines

## Responsible Disclosure

We will credit reporters in the release notes for accepted reports, unless
the reporter asks to remain anonymous. CVE assignment is requested for
high-severity issues. Coordinated disclosure timelines are negotiated on a
per-issue basis, with a default of 90 days from report to public disclosure.
