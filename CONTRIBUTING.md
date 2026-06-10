# Contributing to facto

Thanks for your interest in contributing!

## Development setup

```bash
git clone https://github.com/ggueret/facto.git
cd facto
make help        # list all available targets
make check       # fmt + clippy (fast)
```

Requirements: Rust 1.85+ (edition 2024). Optional tools: `cargo-audit` and
`cargo-deny` for the `make audit` / `make deny` targets; `uv` for `make wheel`.

## Making changes

1. Fork the repo and create a branch from `main`
2. Make your changes
3. Run `make check`
4. Run `make test`
5. Commit using [Conventional Commits](https://www.conventionalcommits.org/) format
6. Open a pull request

## Testing

```bash
# Fast pre-commit
make check

# Full CI pipeline locally (fmt + clippy + tests + docs + audit + deny)
make ci

# Unit tests only (no network)
make test

# Integration tests (calls real upstream APIs, requires network)
make test-integration
```

Integration tests spawn `facto` with the `mcp` subcommand as a child process
via rmcp client and call each tool against real registries/forges. They are
`#[ignore]` by default and skipped in CI to avoid flaky network failures.

For GitHub-related tests, set `FACTO_GITHUB_TOKEN` for higher rate limits:

```bash
export FACTO_GITHUB_TOKEN="ghp_..."
```

## Commit messages

Use [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/):

```
feat: add support for Hex registry
fix(pypi): normalize names with consecutive dots
docs: update installation instructions
```

## Adding a new registry

1. Create `facto-core/src/registries/yourregistry.rs`
2. Implement the `Registry` trait
3. Add to `facto-core/src/registries/mod.rs`
4. Add to the manager factory in `registries/manager.rs`
5. Add integration tests in `facto-run/tests/test_packages.rs`

## Code style

- `cargo fmt` for formatting
- `cargo clippy` for lints
- No unsafe code in production (tests may use unsafe for env manipulation)
- Error types use `thiserror`
- Library functions return `Result`, never panic

## AI coding assistants

Contributions involving AI coding assistants (Claude, Codex, Cursor, Copilot,
and similar) are welcome under the same rules as any other contribution. See
[`AGENTS.md`](AGENTS.md) for the short version and
[`docs/coding-assistants.md`](docs/coding-assistants.md) for the full
guidance on attribution, human responsibility, and prohibited behaviours.

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
