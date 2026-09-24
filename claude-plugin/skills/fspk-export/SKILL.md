---
name: fspk-export
description: >-
  Use when exporting FSPK or integrating its runtime. Check field fidelity,
  binary layout and the consuming engine's responsibilities.
license: MIT
compatibility: Requires the Framesmith checkout and CLI/app; runtime uses framesmith-fspack and framesmith-runtime.
metadata:
  author: nethercore-systems
  version: "1.1.0"
---

# FSPK Export and Runtime Integration

Follow [AGENTS.md](../../../AGENTS.md) for format invariants and verification. Check the requested fields against the [fidelity contract](../../../docs/export-fidelity-contract.md): FSPK v2 is the canonical binary handoff; its typed payload preserves resolved data. Legacy helper tables remain a subset; engine-owned behavior still needs its data.

## Export

For an FSPK export, from the repo root:

```bash
cd src-tauri
cargo run --bin framesmith-cli -- export --project .. --character test_char --out ../exports/test_char.fspk
```

The CLI defaults to `fspk`; `zx-fspack` is a compatibility alias. Use `--adapter json-blob` with a `.json` output only for an authoring/debug handoff. See the [CLI](../../../docs/cli.md) for batch export and directory options. Validation errors must fail export; do not suppress them to obtain a pack.

## Read only the relevant contract

- Byte layout, section records, fixed point, string/asset keys: [format reference](references/fspk-format.md).
- Loading, frame advancement, cancels, collision, resources, rollback or WASM: [runtime integration](references/runtime-integration.md).
- Field preservation or writer/reader changes: update the fidelity contract and exercise the existing roundtrip test through `PackView`, then the affected runtime consumer.

Resolve asset keys at load time, handle pack parse errors, and keep engine-owned movement, resource deltas and rollback state explicit in the integration. Do not infer mechanic support from the presence of an authored JSON field.
