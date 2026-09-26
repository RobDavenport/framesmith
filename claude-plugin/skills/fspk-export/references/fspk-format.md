# FSPK Binary Format Reference

The maintained specification is [docs/zx-fspack.md](../../../../docs/zx-fspack.md): container/section headers, record layouts, fixed point, string/asset keys and properties.

Verify byte-level changes against both [writer constants](../../../../src-tauri/src/codegen/fspk_format.rs) and the [zero-copy reader](../../../../crates/framesmith-fspack/src/). Use [fspk_roundtrip.rs](../../../../src-tauri/tests/fspk_roundtrip.rs) to exercise exported bytes through the reader, and the [fidelity contract](../../../../docs/export-fidelity-contract.md) for what each adapter preserves or omits.

Do not maintain a second offset/size table in the plugin.
