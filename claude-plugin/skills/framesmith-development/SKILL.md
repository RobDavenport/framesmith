---
name: framesmith-development
description: >-
  Development workflow for Framesmith, the fighting game character authoring
  tool. Covers project setup, Tauri + SvelteKit architecture, CLI usage,
  testing (Rust and TypeScript), and repo structure. Use when building features,
  fixing bugs, or understanding the Framesmith codebase.
license: MIT
compatibility: Requires Node.js, Rust toolchain, and Tauri prerequisites.
metadata:
  author: nethercore-systems
  version: "1.0.0"
---

# Framesmith Development

Engine-agnostic fighting game character authoring tool built with Tauri (Rust backend) and SvelteKit (TypeScript/Svelte 5 frontend).

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Desktop framework | Tauri (Rust backend + webview frontend) |
| Frontend | SvelteKit + Svelte 5 (runes) + TypeScript |
| 3D rendering | Threlte / Three.js (sprite preview, GLTF preview, training mode) |
| Data format | JSON (directory-based, one file per state) |
| Binary export | FSPK (zero-copy, no_std, fixed-size records) |

## Quick Commands

All commands assume you are in the `framesmith/` directory unless noted.

```bash
# Install dependencies
npm install

# Development (launches Tauri + SvelteKit dev server)
npm run tauri dev

# Production build
npm run tauri build

# Rust tests (from framesmith/src-tauri/)
cargo test
cargo clippy --all-targets    # 0 warnings required

# TypeScript checks (from framesmith/)
npm run check                 # svelte-check + tsc
npm run test:run              # vitest (training, rendercore tests)

# CLI export (from framesmith/src-tauri/)
cargo run --bin framesmith -- export --project .. --all --out-dir ../exports

# MCP server (from framesmith/src-tauri/)
cargo run --bin mcp -- --characters-dir ../characters
```

## Task-Type Routing

Use this table to find the right files for your task.

| Task type | Start files | Docs |
|-----------|------------|------|
| Add/change state field | `schema/mod.rs` | `docs/data-formats.md` |
| Add/change hitbox/hurtbox | `schema/hitbox.rs` | `docs/data-formats.md` |
| Change FSPK export | `codegen/fspk/`, `fspk_format.rs` | `docs/zx-fspack.md` |
| Change FSPK reader | `crates/framesmith-fspack/src/view/` | `docs/zx-fspack.md` |
| Rules / validation | `rules/validate.rs`, `rules/mod.rs` | `docs/rules-spec.md` |
| Add Tauri command | `commands/*.rs`, wire in `lib.rs` | -- |
| Frontend view | `src/lib/views/*.svelte` | -- |
| Frontend store | `src/lib/stores/*.svelte.ts` | -- |
| MCP server | `mcp/handlers.rs`, `bin/mcp.rs` | `docs/mcp-server.md` |
| Cancel system | `schema/mod.rs` (CancelTable), `variant/` | `docs/data-formats.md` |
| Training mode (logic) | `src/lib/training/` | `docs/runtime-guide.md` |
| Training mode (UI) | `src/lib/components/training/`, `src/routes/training/` | -- |
| Render engine | `src/lib/rendercore/` | -- |
| Global states | `globals/mod.rs`, `src/lib/components/Global*.svelte` | `docs/global-states.md` |
| CLI / export | `bin/framesmith.rs`, `commands/export.rs` | `docs/cli.md` |

## Architectural Invariants

These rules are non-negotiable. Violating them causes subtle bugs or format incompatibilities.

1. **FSPK = fixed-size records, zero-copy.** Never use variable-length encoding (MessagePack, JSON) in FSPK sections. All records are fixed-width structs read directly from the buffer.

2. **State files = one per file.** Each state lives at `states/<input>.json`. Never combine multiple states into one file.

3. **FSPK writer and reader must stay in sync.** The writer (`codegen/fspk/`) and reader (`crates/framesmith-fspack/`) must agree on every byte offset. Change one, update the other.

4. **Validation is shared.** UI save, CLI export, and MCP server all use `rules/validate.rs`. Do not create parallel validation paths.

5. **PropertyValue nested types are flattened at export.** `Object`/`Array` values become dot-path keys (e.g., `movement.distance`) in FSPK. They do not survive as nested structures.

## Change-Impact Map

When modifying one area, check the dependent areas listed here.

| If you change... | Also update/check... |
|---|---|
| `schema/mod.rs` (State, Character structs) | `codegen/fspk/`, `codegen/json_blob.rs`, `mcp/handlers.rs`, `docs/data-formats.md` |
| `rules/mod.rs` (RulesFile struct) | `docs/rules-spec.md`, `mcp/validation.rs` |
| `codegen/fspk_format.rs` (format constants) | `crates/framesmith-fspack/src/view/` (reader must match writer) |
| Cancel table schema | `schema/mod.rs`, `docs/data-formats.md`, `src/lib/views/CancelGraph.svelte` |
| Tauri commands (`commands/`) | Frontend stores (`src/lib/stores/`) that call them |
| PropertyValue variants | FSPK property packing (`codegen/fspk/properties.rs`) |

## Verification Commands

Run these before every commit.

```bash
# Rust (from framesmith/src-tauri/)
cargo clippy --all-targets        # Expect: 0 warnings
cargo test                        # Expect: all pass

# TypeScript (from framesmith/)
npm run check                     # svelte-check + tsc. Expect: 0 errors
npm run test:run                  # vitest. Expect: all pass
```

## Code Standards

### Rust

- No clippy warnings. No `#[allow(dead_code)]`.
- Use `saturating_add`/`checked_add` for overflow-prone arithmetic.
- Comments explain WHY, not WHAT.

### TypeScript / Svelte

- ARIA roles on interactive elements.
- No `@ts-expect-error` without explanation.
- Stores use Svelte 5 runes: files are `.svelte.ts`, use `$state()` / `$derived()`.

### General

- Prefer explicit over implicit. No magic numbers.
- Test utilities: `tests/common/` (Rust), `src/lib/training/*.test.ts` / `src/lib/rendercore/*.test.ts` (TS).

## Common Pitfalls

- **Tags:** lowercase alphanumeric + underscores only. `Tag::new()` validates; uppercase/spaces will fail.
- **State `input` = filename.** Filesystem-hostile characters cause platform-specific failures.
- **`base` field is authoring-only.** Variant inheritance is stripped during export.
- **Nested PropertyValue:** `Object`/`Array` values are flattened to dot-paths at FSPK export.

## Key References

| Topic | Location |
|-------|----------|
| Data formats (SSOT) | `docs/data-formats.md` |
| Rules spec (SSOT) | `docs/rules-spec.md` |
| FSPK binary format | `docs/zx-fspack.md` |
| Runtime integration | `docs/runtime-guide.md` |
| MCP server | `docs/mcp-server.md` |
| Canonical Rust types | `src-tauri/src/schema/mod.rs` |
| Repo map + agent ref | `AGENTS.md` |
| CLI reference | `references/cli-reference.md` (this skill) |
| Project structure | `references/project-structure.md` (this skill) |

## Testing

### Rust (from `src-tauri/`)

- Inline `#[cfg(test)]` modules in most `.rs` files
- Integration tests: `crates/framesmith-runtime/tests/integration.rs`
- WASM integration: `crates/framesmith-runtime-wasm/tests/integration.rs`

### TypeScript (from project root)

- Training: `src/lib/training/*.test.ts` (10 files)
- Rendercore: `src/lib/rendercore/*.test.ts` (6 files)
- Run all: `npm run test:run`

## Export Adapters

| Adapter | Output | Code | Docs |
|---------|--------|------|------|
| `json-blob` | Single JSON file | `codegen/json_blob.rs` | -- |
| `zx-fspack` | `.fspk` binary | `codegen/fspk/` | `docs/zx-fspack.md` |
