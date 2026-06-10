# facto.run

The `facto` CLI shipped via PyPI.

`facto` is a local [MCP](https://modelcontextprotocol.io) server that feeds
real-time package ecosystem data (latest versions, release dates, runtime
EOL, lockfile audits) to AI coding agents (Claude Code, Cursor, Copilot,
...). It stops your agent from recommending outdated packages, deprecated
libraries, or metadata past its training cutoff.

## Install

```bash
# Ephemeral (recommended for MCP wiring, zero-install)
uvx facto.run mcp

# Persistent: puts `facto` on your PATH
uv tool install facto.run
facto mcp
```

Pin a specific version:

```bash
uvx 'facto.run==0.1.0' mcp
```

## Wire into Claude Code

```bash
claude mcp add facto -- uvx facto.run mcp
```

## Documentation

Tool reference, threat model, configuration, and source code live in the
main repository:

  https://github.com/ggueret/facto

## License

MIT
