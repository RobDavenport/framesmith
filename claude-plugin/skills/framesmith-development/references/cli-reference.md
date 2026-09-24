# Framesmith CLI Reference

The maintained command reference is [docs/cli.md](../../../../docs/cli.md). The executable is `framesmith-cli`; `framesmith` is the desktop app.

From the repo root:

```bash
cd src-tauri
cargo run --bin framesmith-cli -- export --project .. --character test_char --out ../exports/test_char.fspk
```

This invocation is covered by `src-tauri/tests/docs_cli_examples.rs`. Read the canonical CLI page for builds, adapters, batch export and directory/environment overrides; use `--help` for supported options. The separate [MCP server](../../../../docs/mcp-server.md) has its own setup and tools.
