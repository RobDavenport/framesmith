# Framesmith Documentation

Status: active
Last reviewed: 2026-02-09

## Source of truth map

| Topic | Location |
|-------|----------|
| Repo constraints and invariants | [`../CLAUDE.md`](../CLAUDE.md) |
| Contributor and code-map reference | [`../AGENTS.md`](../AGENTS.md) |
| Project overview and quick start | [`../README.md`](../README.md) |
| Data formats (on-disk JSON) | [`data-formats.md`](data-formats.md) |
| Rules semantics (SSOT) | [`rules-spec.md`](rules-spec.md) |
| MCP server | [`mcp-server.md`](mcp-server.md) |
| ZX FSPK format | [`zx-fspack.md`](zx-fspack.md) |
| Runtime integration | [`runtime-guide.md`](runtime-guide.md) |
| Runtime API | [`runtime-api.md`](runtime-api.md) |
| CLI usage | [`cli.md`](cli.md) |
| Global states | [`global-states.md`](global-states.md) |
| Architecture overview | [`architecture.md`](architecture.md) |
| Troubleshooting | [`troubleshooting.md`](troubleshooting.md) |
| Character authoring flow | [`character-authoring-guide.md`](character-authoring-guide.md) |
| Movement reference | [`movement-reference.md`](movement-reference.md) |

## Reading order

- New to Framesmith: start with [`../README.md`](../README.md)
- Editing schema or files on disk: read [`data-formats.md`](data-formats.md)
- Changing validation/rules behavior: read [`rules-spec.md`](rules-spec.md)
- Integrating external tools/LLMs: read [`mcp-server.md`](mcp-server.md)
- Implementing export/runtime work: read [`zx-fspack.md`](zx-fspack.md) and [`runtime-guide.md`](runtime-guide.md)
- Understanding the system architecture: read [`architecture.md`](architecture.md)
- Debugging issues: read [`troubleshooting.md`](troubleshooting.md)

## Document set

| Document | Purpose |
|----------|---------|
| [`data-formats.md`](data-formats.md) | Canonical file layout and JSON data model |
| [`rules-spec.md`](rules-spec.md) | Rule matching, defaults, and validation behavior |
| [`mcp-server.md`](mcp-server.md) | MCP server setup and available operations |
| [`zx-fspack.md`](zx-fspack.md) | Binary pack format details |
| [`runtime-guide.md`](runtime-guide.md) | Runtime integration path |
| [`runtime-api.md`](runtime-api.md) | Runtime API reference |
| [`cli.md`](cli.md) | CLI commands and examples |
| [`global-states.md`](global-states.md) | Global state model and usage |
| [`character-authoring-guide.md`](character-authoring-guide.md) | Authoring workflow guidance |
| [`movement-reference.md`](movement-reference.md) | Movement and notation reference |
| [`architecture.md`](architecture.md) | System architecture and data pipeline overview |
| [`troubleshooting.md`](troubleshooting.md) | Common issues and solutions |
| [`design.md`](design.md) | Design rationale and roadmap notes |

## Plans

Implementation plans under `docs/plans/` are intentionally temporary.
When work is complete, update permanent docs in this folder and then remove the plan file.
