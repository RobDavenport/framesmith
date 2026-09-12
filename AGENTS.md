# Framesmith Agent Guide

Engine-agnostic fighting game character authoring: Tauri/Rust, SvelteKit/Svelte 5/TypeScript, Threlte/Three.js previews, JSON source data.

This is the canonical repo guidance for Astra and other assistants. `CLAUDE.md` imports it; plugin skills route to topic references. Paths and commands below are relative to this repository unless a `cd` is shown.

## Working contract

- Carry authorized implementation through a verified result. Infer routine reversible details; ask only when missing information or authorization materially changes the outcome. Complete independent authorized work before asking.
- User instructions and host permissions govern scope; this file owns repo workflow, and topic docs own their domain contracts. If guidance causes a pause or conflict, cite the exact file and rule rather than silently broadening or abandoning the task.
- Keep implementation, diagnosis and verification with the current owner. Plugin agent definitions are opt-in, not instructions to delegate. Do not change model, reasoning effort, provider or host configuration as part of ordinary repo work.
- Inspect `git status` in this repo and preserve unrelated changes. Use the existing checkout; no reset, stash, extra worktree, publish or release unless the task calls for it.
- Trace the affected flow and shared callers before editing. Reuse existing code and dependencies; fix the shared cause with the smallest complete diff. Avoid speculative abstractions, installs and unrelated cleanup.
- Read only the references relevant to the task. Lead the final response with the result, checks actually run and any remaining blocker; omit routine tool narration and unrequested design essays.

## Invariants

- One state per JSON file under `characters/<id>/states/`. Base-state filenames match `input`; variant overlays use names such as `5H~level2.json`, with a unique resolved `id` distinct from gameplay `input`.
- Resolved variants are read-only snapshots. Never save them over a base or overlay file; edit the overlay itself. `base` is authoring-only and resolved/stripped by the loader before the normal export path. See [variant editing policy](docs/variant-editing-decision.md).
- UI save, CLI export and MCP validation share `src-tauri/src/rules/validate.rs`. Preserve shared validation and filesystem/path guards; do not add parallel validation paths.
- FSPK uses fixed-width records with zero-copy, unaligned-safe reads. Do not replace record sections with JSON/MessagePack. Keep the writer, format constants and reader byte layout in sync.
- FSPK v2 typed payloads preserve nested values, empty containers and literal keys. Only the legacy compact property tables flatten to dot paths; they are not the full-fidelity source.
- FSPK v2 is the canonical binary handoff; JSON remains authoring/debug data. Preserve v1 read compatibility, full typed-payload fidelity and engine ownership of behavior. Follow the [handoff decision](docs/production-handoff-decision.md) and [fidelity contract](docs/export-fidelity-contract.md); data preservation is not implementation of a mechanic.
- Tags use lowercase alphanumeric/underscores (`Tag::new()` validates). Use checked/saturating arithmetic where overflow is possible, explain non-obvious constants, and keep Rust free of clippy warnings without `#[allow(dead_code)]`. Preserve accessible controls/ARIA, explained TypeScript suppressions and `.svelte.ts` rune stores (`$state`/`$derived`).

## Task routing and change impact

| Task | Start and trace | Relevant reference |
|---|---|---|
| State, character, hitbox or effect fields | `src-tauri/src/schema/` → `src/lib/types.ts`, exporters and MCP handlers | [Data formats](docs/data-formats.md), [fidelity contract](docs/export-fidelity-contract.md) |
| Rules/defaults/validation | `src-tauri/src/rules/` → UI saves, CLI exports, `src-tauri/src/mcp/validation.rs` | [Rules spec](docs/rules-spec.md) |
| Export/FSPK layout or properties | `src-tauri/src/commands/export.rs`, `src-tauri/src/codegen/` ↔ `crates/framesmith-fspack/src/` | [FSPK format](docs/zx-fspack.md), [fidelity contract](docs/export-fidelity-contract.md) |
| Editor/Tauri IPC | `src/lib/views/`, `src/lib/components/`, `src/lib/stores/` ↔ `src-tauri/src/commands/`, command registration in `src-tauri/src/lib.rs` | [Architecture](docs/architecture.md) |
| Cancels/variants/globals | `src-tauri/src/schema/mod.rs`, `src-tauri/src/variant/`, `src-tauri/src/globals/` → editor and runtime consumers | [Data formats](docs/data-formats.md), [global states](docs/global-states.md) |
| Runtime simulation | `crates/framesmith-runtime/src/`, `crates/framesmith-runtime-wasm/src/` | [Runtime guide](docs/runtime-guide.md), [API](docs/runtime-api.md), [mechanic ownership](docs/combat-coverage.md) |
| Training/preview | `src/lib/training/`, `src/lib/rendercore/`, `src/lib/views/TrainingMode.svelte`, `src/routes/training/` → WASM runtime | [Training contract](docs/training-scenario-contract.md) |
| CLI/MCP | `src-tauri/src/bin/framesmith.rs`, `src-tauri/src/bin/mcp.rs`, `src-tauri/src/mcp/handlers.rs` → shared commands | [CLI](docs/cli.md), [MCP](docs/mcp-server.md) |

Other topics: [documentation index](docs/README.md). A route is an impact map, not a requirement to edit every listed file.

## Verification and stop condition

Choose checks for the changed behavior and its actual consumers. Leave one fault-revealing regression check for non-trivial logic; reuse existing tests. Documentation-only edits need link/frontmatter/command checks, not a full app build or tests that merely repeat prose.

| Change | Relevant checks |
|---|---|
| TypeScript/Svelte | `npm run check`; `npm run test:run -- <affected.test.ts>` |
| Rust backend | `cargo test --manifest-path src-tauri/Cargo.toml <filter>`; `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` |
| Export/schema | `cargo test --manifest-path src-tauri/Cargo.toml --test export_fidelity_contract --test fspk_roundtrip` |
| Documented CLI export commands | `cargo test --manifest-path src-tauri/Cargo.toml --test docs_cli_examples` |
| Runtime crates | `cargo test --manifest-path crates/framesmith-runtime/Cargo.toml` or the affected crate's manifest |
| Editor/training behavior | Affected tests under `tests/e2e/` via `npm run test:e2e -- --grep <scenario>` plus the real app path when the claim depends on native IPC/filesystem behavior |

Generated WASM bindings must exist before frontend checks (`npm run wasm:build` if absent or affected). Runtime-WASM integration tests need the exported `exports/test_char.fspk` fixture; follow the order in [CI](.github/workflows/ci.yml). Do not reinstall dependencies or regenerate unchanged outputs just to run a prose check.

Complete required checks, then stop broadening/repeating them unless new changes, failures or a concrete unresolved risk justify it. Report failures and untested boundaries honestly. Local task checks do not replace existing CI, branch-protection or [release-runbook](docs/release-runbook.md) gates.

## Run the tool

```bash
# From the repo root, with dependencies installed
npm run tauri dev

# From src-tauri/; CLI export and MCP are separate binaries
cd src-tauri
cargo run --bin framesmith-cli -- export --project .. --all --out-dir ../exports
cargo run --bin mcp -- --characters-dir ../characters
```

For setup and packaging, use [README.md](README.md). Prompting basis: [Astra guide](https://developers.openai.com/api/docs/guides/latest-model.md), especially instruction conflicts, follow-through and bounded verification. API feature examples in that guide do not configure this repository or authorize host changes.
