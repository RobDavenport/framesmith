# Framesmith Agent Reference

Engine-agnostic fighting game character authoring tool (Tauri + SvelteKit).

## Start Here

| Topic | Location |
|-------|----------|
| Claude Code guidelines | `CLAUDE.md` |
| Agent reference (this file) | `AGENTS.md` |
| Documentation index | `docs/README.md` |
| Data formats (SSOT) | `docs/data-formats.md` |
| Rules spec (SSOT) | `docs/rules-spec.md` |
| FSPK binary format | `docs/zx-fspack.md` |
| MCP server | `docs/mcp-server.md` |
| Runtime guide | `docs/runtime-guide.md` |
| Global states | `docs/global-states.md` |
| CLI | `docs/cli.md` |

## Repo Map

```
framesmith/
  src-tauri/                 # Rust backend
    src/
      main.rs                # Tauri entry point
      lib.rs                 # Library exports
      bin/
        mcp.rs               # MCP server binary
        framesmith.rs         # CLI binary (export, etc.)
        generate_schema.rs    # JSON schema generation
      schema/                # Character data types
        mod.rs               # Core types (State, Character, CancelTable, etc.)
        hitbox.rs            # Hitbox/hurtbox types
        effects.rs           # Effect types
        assets.rs            # Asset types
      commands/              # Tauri IPC commands
        mod.rs
        character.rs         # Character CRUD
        project.rs           # Project operations
        export.rs            # Export commands
      codegen/               # Export adapters
        mod.rs
        json_blob.rs         # JSON blob adapter
        fspk_format.rs       # FSPK format constants
        fspk/                # FSPK pack exporter (modular)
          mod.rs, export.rs, sections.rs, moves.rs,
          properties.rs, builders.rs, packing.rs, types.rs, utils.rs
      rules/                 # Rules system (defaults + validation)
        mod.rs               # RulesFile types
        validate.rs          # Shared validation pipeline
        apply.rs             # Default application
        matchers.rs          # Rule matching logic
        registry.rs          # Resource/event registry
        property_schema.rs   # Property schema validation
      mcp/                   # MCP server modules
        mod.rs, handlers.rs, validation.rs, validators.rs
      variant/               # Variant/overlay system
        mod.rs
      globals/               # Global states
        mod.rs
  crates/
    framesmith-fspack/       # no_std FSPK reader
      src/
        lib.rs, bytes.rs, error.rs
        view/                # Zero-copy section readers
          mod.rs, property.rs, event.rs, resource.rs,
          state.rs, cancel.rs, hurtbox.rs, hitbox.rs, schema.rs
        fixed/               # Fixed-point arithmetic
          mod.rs, fixed_q12_4.rs, fixed_q8_8.rs
    framesmith-runtime/      # Runtime crate (cancel resolution, state machine)
      src/lib.rs, state.rs, frame.rs, cancel.rs, resource.rs, collision/
      tests/integration.rs
    framesmith-runtime-wasm/  # WASM runtime (training mode)
      src/lib.rs
      tests/integration.rs
  src/                       # SvelteKit frontend
    routes/
      +page.svelte           # Main editor page
      training/+page.svelte  # Training mode page
    lib/
      views/                 # Main editor views
        CharacterOverview.svelte
        FrameDataTable.svelte
        StateEditor.svelte
        CancelGraph.svelte
        GlobalsManager.svelte
        TrainingMode.svelte
        editor/              # Sub-editors
          PreconditionEditor.svelte, CostEditor.svelte
      components/            # Reusable UI components
        Sidebar.svelte, Header.svelte, Toast.svelte, ...
        preview/             # Animation preview
          SpritePreview.svelte, GltfPreview.svelte, BoxEditor.svelte
        training/            # Training mode UI
          TrainingHUD.svelte, PlaybackControls.svelte,
          TrainingViewport.svelte, HitboxOverlay.svelte, ...
      stores/                # Svelte 5 rune stores (.svelte.ts)
        project.svelte.ts, character.svelte.ts, globals.svelte.ts,
        assets.svelte.ts, toast.svelte.ts
      training/              # Training mode logic + tests
        TrainingSession.ts, MoveResolver.ts, InputBuffer.ts, ...
        *.test.ts            # 10 test files
      rendercore/            # Render engine + tests
        RenderCore.ts, sampling.ts, loadSeq.ts, ...
        actors/              # Actor implementations
        *.test.ts            # 6 test files
  docs/
    README.md, design.md, data-formats.md, rules-spec.md,
    zx-fspack.md, mcp-server.md, runtime-guide.md, runtime-api.md,
    cli.md, global-states.md, character-authoring-guide.md,
    movement-reference.md
    plans/                   # Implementation plans (removed when done)
```

## Task-Type Routing

| Task type | Start files | Docs |
|-----------|------------|------|
| Add/change state field | `schema/mod.rs` | `docs/data-formats.md` |
| Add/change hitbox/hurtbox | `schema/hitbox.rs` | `docs/data-formats.md` |
| Change FSPK export | `codegen/fspk/`, `fspk_format.rs` | `docs/zx-fspack.md` |
| Change FSPK reader | `crates/framesmith-fspack/src/view/` | `docs/zx-fspack.md` |
| Rules / validation | `rules/validate.rs`, `rules/mod.rs` | `docs/rules-spec.md` |
| Add Tauri command | `commands/*.rs` then wire in `lib.rs` | -- |
| Frontend view | `src/lib/views/*.svelte` | -- |
| Frontend store | `src/lib/stores/*.svelte.ts` | -- |
| MCP server | `mcp/handlers.rs`, `bin/mcp.rs` | `docs/mcp-server.md` |
| Cancel system | `schema/mod.rs` (CancelTable), `variant/` | `docs/data-formats.md` |
| Training mode (logic) | `src/lib/training/` | `docs/runtime-guide.md` |
| Training mode (UI) | `src/lib/components/training/`, `src/routes/training/` | -- |
| Render engine | `src/lib/rendercore/` | -- |
| Global states | `globals/mod.rs`, `src/lib/components/Global*.svelte` | `docs/global-states.md` |
| CLI / export | `bin/framesmith.rs`, `commands/export.rs` | `docs/cli.md` |

## Quick Commands

```bash
# Install (from framesmith/)
npm install

# Development (from framesmith/)
npm run tauri dev                # Launches Tauri + SvelteKit dev server

# Production build (from framesmith/)
npm run tauri build

# Rust tests (from framesmith/src-tauri/)
cargo test                       # All backend + schema + codegen tests
cargo clippy --all-targets       # Lint (0 warnings required)

# TypeScript (from framesmith/)
npm run check                    # svelte-check + tsc
npm run test:run                 # vitest (training, rendercore tests)

# MCP server (from framesmith/src-tauri/)
cargo run --bin mcp -- --characters-dir ../characters

# CLI export (from framesmith/src-tauri/)
cargo run --bin framesmith -- export --all --project .. --out-dir ../exports
```

## Editor Views

| View | File | Purpose |
|------|------|---------|
| Character Overview | `views/CharacterOverview.svelte` | Character list, properties |
| Frame Data Table | `views/FrameDataTable.svelte` | Spreadsheet with type filtering |
| State Editor | `views/StateEditor.svelte` | Form editing + animation preview |
| Cancel Graph | `views/CancelGraph.svelte` | Cancel relationship visualization |
| Globals Manager | `views/GlobalsManager.svelte` | Project-wide global states |
| Training Mode | `views/TrainingMode.svelte` | WASM runtime testing + hitbox overlay |

## Export Adapters

| Adapter | Output | Code | Docs |
|---------|--------|------|------|
| json-blob | Single JSON file | `codegen/json_blob.rs` | -- |
| zx-fspack | `.fspk` binary | `codegen/fspk/` | `docs/zx-fspack.md` |

## Testing

### Rust (from `src-tauri/`)
- Inline `#[cfg(test)]` modules in most `.rs` files
- `cargo test` runs all

### TypeScript (from project root)
- Training: `src/lib/training/*.test.ts` (10 files)
- Rendercore: `src/lib/rendercore/*.test.ts` (6 files)
- `npm run test:run` runs all via vitest

### Integration
- FSPK roundtrip: `crates/framesmith-runtime/tests/integration.rs`
- WASM runtime: `crates/framesmith-runtime-wasm/tests/integration.rs`

## MCP Server

Full docs: `docs/mcp-server.md`

```bash
# Build
cd src-tauri && cargo build --bin mcp

# Run
cd src-tauri && cargo run --bin mcp -- --characters-dir ../characters
```
