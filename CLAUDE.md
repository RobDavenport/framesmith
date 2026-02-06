# Framesmith (Claude Code Guidelines)

Engine-agnostic fighting game character authoring tool. Tauri (Rust) + SvelteKit (TypeScript).

## Tech Stack

- **Desktop framework:** Tauri (Rust backend + web frontend)
- **Frontend:** SvelteKit + Svelte 5 (runes) + TypeScript
- **3D rendering:** Threlte / Three.js (sprite + GLTF preview, training mode)
- **Data format:** JSON (directory-based, one file per state)

## Key References

| Topic | Location |
|-------|----------|
| Data formats (SSOT) | `docs/data-formats.md` |
| Rules spec (SSOT) | `docs/rules-spec.md` |
| FSPK binary format | `docs/zx-fspack.md` |
| Runtime integration | `docs/runtime-guide.md` |
| MCP server | `docs/mcp-server.md` |
| Canonical Rust types | `src-tauri/src/schema/mod.rs` |
| Repo map + agent reference | `AGENTS.md` |

## Architectural Invariants

- **FSPK = fixed-size records, zero-copy.** Never use variable-length encoding (MessagePack, JSON) in FSPK sections.
- **State files = one per file.** `states/<input>.json`. Never combine multiple states.
- **FSPK writer and reader must stay in sync.** `codegen/fspk/` (writer) and `crates/framesmith-fspack/` (reader) must agree on format.
- **Validation is shared.** UI save, CLI export, and MCP server all use `rules/validate.rs`. Do not create parallel validation paths.
- **PropertyValue nested types are flattened at export.** `Object`/`Array` values become dot-path keys (e.g., `movement.distance`) in FSPK. They do not survive as nested structures.

## Change-Impact Map

| If you change... | Also update/check... |
|---|---|
| `schema/mod.rs` (State, Character structs) | `codegen/fspk/`, `codegen/json_blob.rs`, `mcp/handlers.rs`, `docs/data-formats.md` |
| `rules/mod.rs` (RulesFile struct) | `docs/rules-spec.md`, `mcp/validation.rs` |
| `codegen/fspk_format.rs` (format constants) | `crates/framesmith-fspack/src/view/` (reader must match writer) |
| Cancel table schema | `schema/mod.rs`, `docs/data-formats.md`, `src/lib/views/CancelGraph.svelte` |
| Tauri commands (`commands/`) | Frontend stores (`src/lib/stores/`) that call them |
| PropertyValue variants | FSPK property packing (`codegen/fspk/properties.rs`) |

## Common Pitfalls

- **Tags:** lowercase alphanumeric + underscores only. `Tag::new()` validates; uppercase/spaces fail.
- **State `input` = filename.** Filesystem-hostile characters cause platform-specific failures.
- **`base` field is authoring-only.** Variant inheritance is stripped during export.
- **Nested PropertyValue:** `Object`/`Array` values are flattened to dot-paths at FSPK export. Don't expect nested structures in the binary.
- **Stores use Svelte 5 runes:** Files are `.svelte.ts`, not `.ts`. Use `$state()` / `$derived()`.

## Verification (run before every commit)

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
- `saturating_add`/`checked_add` for overflow-prone arithmetic.
- Comments explain WHY, not WHAT.

### TypeScript/Svelte
- ARIA roles on interactive elements.
- No `@ts-expect-error` without explanation.

### General
- Prefer explicit over implicit. No magic numbers.
- Test utilities: `tests/common/` (Rust), `src/lib/training/*.test.ts` / `src/lib/rendercore/*.test.ts` (TS).
