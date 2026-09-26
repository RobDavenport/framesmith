# framesmith-fspack

Engine-independent, unaligned-safe, allocation-free Rust views over FSPK binary
payloads. Default features are empty (`no_std`, no dependencies). Version 0.2.0
reads valid v1 packs and full-fidelity v2 typed payloads; this is a source version,
not a claim that it has been published to a registry.

- `PackView::parse(&bytes)?` validates section layouts and nested references.
- `pack.payload()` returns v2 typed data, or `None` for v1. Values support null,
  bool, i64, u64, f64, UTF-8, arrays and sorted-key objects. Use `get`, `at`,
  `children` and typed `as_*` accessors; no JSON parser or allocation is required.
- `pack.state_data(index)` and `state_id(index)` preserve authored data and
  resolved identity alongside optional compiled helper tables.
- With `alloc`, `OwnedPack::new(bytes)?` validates owned bytes once; `view()`
  borrows them without reparsing. Bytes and cached offsets cannot be mutated.
- `std` enables `alloc` and legacy float helpers. `builder` additionally uses
  the existing serde_json value model **at authoring time** to encode typed
  records; it does not put JSON text into the binary or into default dependencies.

From the repository root:

```bash
cargo test --manifest-path crates/framesmith-fspack/Cargo.toml --all-features
cargo run --manifest-path crates/framesmith-fspack/Cargo.toml --features builder --example engine_payloads
```

The executable example runs charge/reload policies and reads fractional movement
without a combat schema, editor, Tauri, renderer, wall clock or engine adapter.
FSPK is not tied to Nethercore. Generic payloads need no compiled state tables.

Limits: at most 32 sections, u32 file/string lengths, typed-tree depth at most 64.
Legacy helper records have their own narrower ranges and quantization; they are
not the canonical full-data source. Engine behavior and application snapshots
remain the consumer's responsibility. Re-export original sources to recover
fields omitted by v1; never try to invent them from a v1 binary.
