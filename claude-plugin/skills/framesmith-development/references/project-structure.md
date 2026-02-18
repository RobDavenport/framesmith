# Framesmith Project Structure

## Top-Level Layout

```
framesmith/
  CLAUDE.md              # Claude Code guidelines
  AGENTS.md              # Agent reference (repo map, task routing, commands)
  package.json           # Node.js dependencies (SvelteKit, Threlte, etc.)
  src-tauri/             # Rust backend (Tauri app + CLI + MCP server)
  crates/                # Standalone Rust crates (runtime, FSPK reader)
  src/                   # SvelteKit frontend
  docs/                  # Documentation
```

## Rust Backend (`src-tauri/`)

```
src-tauri/
  src/
    main.rs                # Tauri entry point
    lib.rs                 # Library exports, Tauri command registration
    bin/
      framesmith.rs        # CLI binary (export, automation)
      mcp.rs               # MCP server binary
      generate_schema.rs   # JSON schema generation utility
    schema/                # Character data types (SSOT for data model)
      mod.rs               # Core types: State, Character, CancelTable, etc.
      hitbox.rs            # Hitbox/hurtbox types
      effects.rs           # Effect types
      assets.rs            # Asset types
    commands/              # Tauri IPC commands (frontend <-> backend bridge)
      mod.rs               # Command module exports
      character.rs         # Character CRUD operations
      project.rs           # Project open/save operations
      export.rs            # Export commands (FSPK, JSON)
    codegen/               # Export adapters (character data -> output format)
      mod.rs               # Adapter registry
      json_blob.rs         # JSON blob adapter
      fspk_format.rs       # FSPK format constants (section IDs, sizes)
      fspk/                # FSPK pack exporter (modular)
        mod.rs             # Top-level export entry point
        export.rs          # Character -> FSPK conversion
        sections.rs        # Section builders
        moves.rs           # State record packing
        properties.rs      # Property packing (CharacterProp12, SchemaProp8)
        builders.rs        # Section builder helpers
        packing.rs         # Binary packing utilities
        types.rs           # Internal export types
        utils.rs           # Shared utilities
    rules/                 # Rules system (defaults + validation)
      mod.rs               # RulesFile types
      validate.rs          # Shared validation pipeline (used by UI, CLI, MCP)
      apply.rs             # Default application
      matchers.rs          # Rule matching logic
      registry.rs          # Resource/event registry
      property_schema.rs   # Property schema validation
    mcp/                   # MCP server modules
      mod.rs               # MCP module exports
      handlers.rs          # Tool/resource handlers
      validation.rs        # MCP validation integration
      validators.rs        # Character validators
    variant/               # Variant/overlay system
      mod.rs               # Variant resolution, inheritance stripping
    globals/               # Global states
      mod.rs               # Project-wide global state management
```

## Standalone Crates (`crates/`)

```
crates/
  framesmith-fspack/       # no_std FSPK reader (used by games at runtime)
    src/
      lib.rs               # PackView entry point
      bytes.rs             # Byte-level reading utilities
      error.rs             # Error types (TooShort, InvalidMagic, OutOfBounds)
      view/                # Zero-copy section readers
        mod.rs             # Section dispatch
        property.rs        # CharacterProp / SchemaProp readers
        event.rs           # EventEmit / EventArg readers
        resource.rs        # ResourceDef reader
        state.rs           # StateRecord reader
        cancel.rs          # Cancel table readers
        hurtbox.rs         # HurtWindow reader
        hitbox.rs          # HitWindow reader
        schema.rs          # Schema section reader
      fixed/               # Fixed-point arithmetic (Q12.4, Q8.8)
        mod.rs
        fixed_q12_4.rs     # 12.4 fixed-point (hitbox coordinates)
        fixed_q8_8.rs      # 8.8 fixed-point (angles, radii)
  framesmith-runtime/      # Runtime crate (state machine, cancel logic, hit detection)
    src/
      lib.rs               # Public API (next_frame, check_hits, etc.)
      state.rs             # CharacterState (22 bytes, Copy)
      frame.rs             # Frame advancement logic
      cancel.rs            # Cancel resolution (denies, chains, tag rules)
      resource.rs          # Resource pool management
      collision/           # Hit detection (hitbox vs hurtbox overlap)
    tests/
      integration.rs       # FSPK roundtrip integration tests
  framesmith-runtime-wasm/ # WASM runtime (browser training mode)
    src/lib.rs             # WASM bindings (TrainingSession)
    tests/integration.rs   # WASM integration tests
```

## SvelteKit Frontend (`src/`)

```
src/
  routes/
    +page.svelte           # Main editor page
    training/+page.svelte  # Training mode page
  lib/
    views/                 # Main editor views
      CharacterOverview.svelte   # Character list, properties
      FrameDataTable.svelte      # Spreadsheet with type filtering
      StateEditor.svelte         # Form editing + animation preview
      CancelGraph.svelte         # Cancel relationship visualization
      GlobalsManager.svelte      # Project-wide global states
      TrainingMode.svelte        # WASM runtime testing + hitbox overlay
      editor/                    # Sub-editors
        PreconditionEditor.svelte
        CostEditor.svelte
    components/            # Reusable UI components
      Sidebar.svelte
      Header.svelte
      Toast.svelte
      preview/             # Animation preview
        SpritePreview.svelte
        GltfPreview.svelte
        BoxEditor.svelte
      training/            # Training mode UI
        TrainingHUD.svelte
        PlaybackControls.svelte
        TrainingViewport.svelte
        HitboxOverlay.svelte
    stores/                # Svelte 5 rune stores (.svelte.ts files)
      project.svelte.ts    # Project state
      character.svelte.ts  # Character state
      globals.svelte.ts    # Global states
      assets.svelte.ts     # Asset management
      toast.svelte.ts      # Toast notifications
    training/              # Training mode logic + tests
      TrainingSession.ts   # Session management
      MoveResolver.ts      # Move resolution
      InputBuffer.ts       # Input buffering
      *.test.ts            # 10 test files
    rendercore/            # Render engine + tests
      RenderCore.ts        # Core renderer
      sampling.ts          # Frame sampling
      loadSeq.ts           # Sequence loading
      actors/              # Actor implementations
      *.test.ts            # 6 test files
```

## Documentation (`docs/`)

```
docs/
  README.md                # Documentation index
  design.md                # Design philosophy and architecture
  data-formats.md          # On-disk JSON formats (SSOT)
  rules-spec.md            # Rules system specification (SSOT)
  zx-fspack.md             # FSPK binary format specification
  runtime-guide.md         # Runtime integration guide
  runtime-api.md           # Runtime API reference
  cli.md                   # CLI usage documentation
  mcp-server.md            # MCP server documentation
  global-states.md         # Global states documentation
  character-authoring-guide.md  # Character authoring guide
  movement-reference.md    # Movement system reference
  plans/                   # Implementation plans (removed when done)
```
