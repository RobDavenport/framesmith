---
name: framesmith-development
description: >-
  Use when developing or debugging Framesmith code. Route to the affected
  Tauri, Svelte, CLI, MCP or runtime path and its existing checks.
license: MIT
compatibility: Requires the Framesmith checkout; Node.js, Rust and Tauri prerequisites for app work.
metadata:
  author: nethercore-systems
  version: "1.1.0"
---

# Framesmith Development

Use the repo's [AGENTS.md](../../../AGENTS.md) for workflow, invariants, task routing and verification. This skill adds no separate approval, delegation or full-suite requirement.

Trace the relevant path before editing:

- Editor: Svelte view → rune store → registered Tauri command → shared schema/rules.
- CLI/MCP: binary or handler → shared commands → validation/defaults → export adapter.
- Training: TypeScript input/session → WASM bindings → Rust runtime → render mapping.
- FSPK: schema → exporter/format constants → zero-copy reader → runtime consumers.

Read only the reference needed for the current task:

- [CLI reference](references/cli-reference.md): tested export invocation and command docs.
- [Project structure](references/project-structure.md): code map and architecture.
- [Character authoring](../character-authoring/SKILL.md): data changes rather than application code.
- [FSPK export](../fspk-export/SKILL.md): binary layout and runtime handoff changes.
