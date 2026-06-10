# AI Coding Assistants

Guidance for AI tools (and humans using AI assistance) when contributing
to facto. See [`docs/coding-assistants.md`](docs/coding-assistants.md) for
the full version of this guide.

## Process

AI-assisted contributions follow the same rules as any other contribution:

- Read [`CONTRIBUTING.md`](CONTRIBUTING.md) for setup, testing, and style.
- Run `make check && make test` locally before opening a pull request.
- Commit messages follow
  [Conventional Commits v1.0.0](https://www.conventionalcommits.org/en/v1.0.0/).

## Human responsibility

The human submitter is responsible for:

- Reviewing all AI-generated code before committing.
- Verifying the change actually works: run the tests, don't trust
  "all green" claims without evidence.
- Ensuring compatibility with facto's MIT license.
- The accuracy of the commit message and PR description.

Don't ship code you can't explain.

## Attribution

When an AI tool materially contributed to a patch, add an `Assisted-by`
trailer at the end of the commit message:

```
Assisted-by: AGENT_NAME:MODEL_VERSION [TOOL1] [TOOL2]
```

Where:

- `AGENT_NAME` is the tool or framework (`Claude`, `Codex`, `Cursor`...).
- `MODEL_VERSION` is the specific model (`claude-opus-4-7`, `gpt-5`...).
- `[TOOL1] [TOOL2]` are optional analysis tools used (e.g., `clippy`,
  `cargo-audit`, `cargo-deny`). Basic tooling (git, cargo, uv, editors)
  is not listed.

Example:

```
Assisted-by: Claude:claude-opus-4-7
```

## Prohibited

AI agents must not:

- Set `Author:` or `Committer:` fields, these identify humans.
- Add `Signed-off-by:` on behalf of anyone.
- Include session metadata, internal reasoning, or memory dumps in commit
  messages, PR descriptions, or committed files.
- Open issues or pull requests without explicit authorization.
