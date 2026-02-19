# Framesmith Troubleshooting

**Status:** Active
**Last reviewed:** 2026-02-20

Common issues organized by category. Each entry lists the symptom, cause, and fix.

## Build Errors

### Rust compilation fails with feature or syntax errors

**Symptom:** `cargo build` fails with unexpected syntax errors or missing feature gates.

**Cause:** Framesmith requires Rust stable 1.75 or newer. Older toolchains lack required language features.

**Fix:** Update Rust with `rustup update stable`. Verify with `rustc --version`.

---

### Node/npm errors during frontend build

**Symptom:** `npm install` or `npm run check` fails with module resolution or syntax errors.

**Cause:** Framesmith requires Node.js 18 or newer. SvelteKit 2 and Svelte 5 use modern JS features.

**Fix:** Update Node.js to 18+ (LTS recommended). Verify with `node --version`.

---

### WASM build fails

**Symptom:** `wasm-pack build` fails or the WASM module cannot be found at runtime.

**Cause:** `wasm-pack` is not installed, or the `wasm32-unknown-unknown` target is missing.

**Fix:**
1. Install wasm-pack: `cargo install wasm-pack`
2. Add the WASM target: `rustup target add wasm32-unknown-unknown`
3. Build: `cd crates/framesmith-runtime-wasm && wasm-pack build --target web`

---

### Tauri build fails on Linux (missing system dependencies)

**Symptom:** Build errors referencing `webkit2gtk`, `libappindicator`, or `libsoup`.

**Cause:** Tauri requires system GTK/WebKit libraries that are not installed by default.

**Fix:** Install the required system packages. On Ubuntu/Debian:
```bash
sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev libappindicator3-dev librsvg2-dev patchelf
```
See the [Tauri prerequisites documentation](https://v2.tauri.app/start/prerequisites/) for other distributions.

## Validation Errors

### "resource not in registry" or "event not in registry"

**Symptom:** Validation rejects a state that references a resource or event by name.

**Cause:** The resource or event is not declared in the `registry` block of `framesmith.rules.json` (or the character-level `rules.json`). When a registry is defined, all resource and event references are checked against it.

**Fix:** Add the missing name to the appropriate registry section:
```json
{
  "registry": {
    "resources": ["heat", "ammo"],
    "events": {
      "your_event": { "contexts": ["on_hit"], "args": {} }
    }
  }
}
```

---

### "invalid tag" on save or export

**Symptom:** Validation error mentioning an invalid tag value.

**Cause:** Tags must be lowercase alphanumeric characters and underscores only. Uppercase letters, spaces, hyphens, and special characters are rejected by `Tag::new()`.

**Fix:** Rename the tag to use only `[a-z0-9_]`. For example, change `"Heavy-Attack"` to `"heavy_attack"`.

---

### "type mismatch" in property values

**Symptom:** Export or validation fails with a type mismatch on a property.

**Cause:** A property value does not match the type declared in the rules property schema. For example, a number was expected but a string was provided.

**Fix:** Check the `property_schema` section of your rules file for the expected type, and correct the property value in the state or character JSON.

---

### "unknown property" when rules define a property schema

**Symptom:** Validation warns about an unknown property key.

**Cause:** The rules file has a `property_schema` section that enumerates allowed property names. A property in the character or state JSON is not listed.

**Fix:** Either add the property to the schema in your rules file, or remove the unrecognized property from the data.

## Export Errors

### Missing required fields during export

**Symptom:** Export fails with errors about missing `animation`, `guard`, or other fields.

**Cause:** The FSPK exporter requires certain fields to be present on every state. These may have been left empty during authoring.

**Fix:** Fill in the missing fields on the affected states. Check the export error output for the specific state and field names. Common required fields: `startup`, `active`, `recovery`, `guard`, `animation`.

---

### Unsupported hitbox shapes in FSPK v1

**Symptom:** Export warning or error about hitbox shape types.

**Cause:** FSPK v1 only supports AABB (axis-aligned bounding box) hitboxes natively. Circle or polygon shapes defined in the `hits[]` multi-hit model are not supported by the current binary format.

**Fix:** Use AABB hitboxes (`{ "x", "y", "w", "h" }`) for states that will be exported to FSPK. The `hitboxes[]` and `hurtboxes[]` top-level arrays use AABB format by default.

---

### Nested property values not appearing in FSPK output

**Symptom:** Properties with `Object` or `Array` values in the JSON are missing or have unexpected keys in the exported binary.

**Cause:** Nested `Object` and `Array` property values are flattened to dot-path keys at export time. `{"movement": {"distance": 80}}` becomes `{"movement.distance": 80}`. `{"effects": ["spark", 2]}` becomes `{"effects.0": "spark", "effects.1": 2}`.

**Fix:** This is expected behavior. Read flattened properties using their dot-path keys in the runtime. See [architecture.md](architecture.md) for details on the flattening convention.

## Training Mode

### WASM module fails to load

**Symptom:** Training mode shows a loading error or blank viewport. Browser console shows a WASM instantiation failure.

**Cause:** The WASM module has not been built, or the built module is incompatible with the browser.

**Fix:**
1. Build the WASM module: `cd crates/framesmith-runtime-wasm && wasm-pack build --target web`
2. Verify the `.wasm` file exists in the output directory.
3. Check the browser console for specific errors (e.g., missing `WebAssembly.instantiateStreaming` support).

---

### Input not detected in training mode

**Symptom:** Training mode loads but does not respond to keyboard or gamepad input.

**Cause:** Input detection depends on the browser's focus state and gamepad API. The training viewport must have focus to receive keyboard events. Gamepad input requires a prior user interaction (browser security policy).

**Fix:**
1. Click inside the training viewport to ensure it has focus.
2. For gamepads: press a button on the controller before starting training mode (this satisfies the browser's gamepad activation requirement).
3. Check browser developer tools console for input-related warnings.

---

### Browser compatibility issues

**Symptom:** Training mode works in one browser but not another.

**Cause:** The WASM runtime requires `WebAssembly.instantiateStreaming` and modern ES module support. Some older browsers or restrictive environments may not support these.

**Fix:** Use a recent version of Chrome, Firefox, or Edge. Safari 15+ should also work. Ensure the page is served over HTTP(S), not `file://` (WASM loading requires proper MIME types).

## MCP Server

### Connection fails or no response

**Symptom:** The MCP client cannot connect to the Framesmith MCP server, or gets no response after connecting.

**Cause:** The MCP server uses stdio transport. The binary path or arguments in `.mcp.json` may be incorrect, or the binary has not been built.

**Fix:**
1. Build the binary: `cd src-tauri && cargo build --release --bin mcp`
2. Verify the `command` path in `.mcp.json` points to the built binary.
3. Test manually: run the binary directly and check for startup errors.

---

### "characters directory not found" on startup

**Symptom:** The MCP server exits with an error about a missing characters directory.

**Cause:** The `--characters-dir` argument (or `FRAMESMITH_CHARACTERS_DIR` environment variable) points to a directory that does not exist.

**Fix:** Verify the path. In `.mcp.json`, paths are relative to the project root (where `.mcp.json` lives). Check that the `characters/` directory exists at the resolved path.

---

### Validation errors from create_state or update_state

**Symptom:** The MCP `create_state` or `update_state` tool returns `INVALID_PARAMS`.

**Cause:** The state data fails validation against project rules. The MCP server runs the same validation pipeline as the UI and CLI.

**Fix:** Read the error details in the response. Common causes: missing required fields, invalid tag format, resource references not in the registry. See the Validation Errors section above for specific fixes.

## See Also

- [Architecture](architecture.md) -- system overview and data pipeline
- [Data Formats](data-formats.md) -- on-disk JSON schema
- [Rules Spec](rules-spec.md) -- validation and defaults system
- [MCP Server](mcp-server.md) -- MCP setup and tools
- [Runtime Guide](runtime-guide.md) -- runtime integration and debugging
