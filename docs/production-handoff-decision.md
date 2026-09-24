# Production Handoff Decision

Status: active; applies to crates 0.2.0 and FSPK v2.

## Decision

`fspk` v2 is the canonical binary runtime handoff. JSON remains the editable
authoring/debug format; a runtime does not need a JSON sidecar.

The binary contains the complete resolved `CharacterData` in fixed-width typed
nodes and a UTF-8 pool. `PackView::payload()` exposes it without allocation or
JSON parsing. `state_data(index)` uses the same input-then-id ordering as the
compiled tables; `state_id(index)` preserves resolved variant identity.

Legacy compact tables remain useful for the optional frame/cancel/resource/
collision helpers. They are **not** the full-fidelity source: use the typed
payload for movement, advanced hits/hurtboxes, custom effects, nested values,
precise numbers, and other engine-owned data. Preserving data does not implement
its gameplay behavior.

The editor defaults to FSPK; JSON Blob is an explicit debug export. Authoring
property numbers use IEEE binary64 (`f64`); JSON reloads enable round-trip float
parsing rather than changing a value by an ULP. Generic payload builders also
support signed/unsigned 64-bit integers and null, independently of the editor's
more constrained `PropertyValue` schema.

## Movement Policy

FSPK v2 preserves movement values. The engine owns movement application, easing,
gravity, floors/walls/corners, stage bounds, and any mutable velocity/accumulator
state needed for rollback. No renderer, coordinate system or 60-Hz clock is
required by the generic binary views.

The executable [`engine_payloads` example](../crates/framesmith-fspack/examples/engine_payloads.rs)
reads fractional travel and runs contrasting charge/reload policies entirely
from binary. It needs neither the editor nor a fighting-game schema.

## Example Pipeline

Canonical handoff (from the repository root):

```bash
cargo run --manifest-path src-tauri/Cargo.toml --bin framesmith-cli -- export --project . --character test_char --adapter fspk --out exports/test_char.fspk
cargo run --manifest-path crates/framesmith-runtime/Cargo.toml --example headless -- exports/test_char.fspk
cargo run --manifest-path crates/framesmith-fspack/Cargo.toml --features builder --example engine_payloads
```

Optional debug export:

```bash
cargo run --manifest-path src-tauri/Cargo.toml --bin framesmith-cli -- export --project . --character test_char --adapter json-blob --pretty --out exports/test_char.json
```

## Compatibility and Migration

- FSPK v1 uses header flags `0`; v2 uses `2` and requires both typed payload
  sections. Readers reject unknown flags and malformed/truncated references.
- The 0.2.0 reader still reads structurally valid v1 packs. `payload()` returns
  `None` for v1, never fabricated missing values. Invalid legacy packs must be
  re-exported rather than accepted through a lenient fallback.
- Re-export the original authoring project to recover v1-omitted fields. A v1
  binary alone cannot reconstruct them. Keep the original pack until the new
  consumer and pack have passed the game's own acceptance checks.
- Upgrade consumers to 0.2.0 for full fidelity. Old readers may ignore version
  flags and see only the legacy tables; that is not v2 support.
- `CharacterState.frame` and `instance_duration` are now `u16`. Do not persist
  native Rust struct memory as a portable binary format; version application
  snapshots explicitly. In-memory `Copy` snapshots still support deterministic
  replay. Resource initialization now reports unsupported capacity instead of
  silently truncating; failed cost payment leaves all balances unchanged.

See [the fidelity contract](export-fidelity-contract.md) and
[the binary layout](zx-fspack.md). Native Rust and the shipped WASM training
wrapper are the integration surfaces here; no Bevy, Godot, Unity, Unreal or
Nethercore engine adapter is shipped or certified by these examples.
