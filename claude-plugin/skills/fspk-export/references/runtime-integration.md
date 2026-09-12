# Framesmith Runtime Integration

Read only the section needed for the current integration:

- [Runtime guide](../../../../docs/runtime-guide.md): pack loading, asset handles, frame loop and rollback.
- [Runtime API](../../../../docs/runtime-api.md): state/input/result types, cancels, collision, resources and feature flags.
- [Combat coverage](../../../../docs/combat-coverage.md): runtime versus engine-owned mechanics.
- [Training contract](../../../../docs/training-scenario-contract.md): WASM/browser scenarios and resource/reaction policy.

Verify exact signatures in [framesmith-runtime/src/lib.rs](../../../../crates/framesmith-runtime/src/lib.rs) or [framesmith-runtime-wasm/src/lib.rs](../../../../crates/framesmith-runtime-wasm/src/lib.rs) before writing integration code. Keep runnable patterns in the canonical docs and existing crate integration tests rather than copying structs, byte-size claims or browser loops here.
