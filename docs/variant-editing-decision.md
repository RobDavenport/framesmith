# Variant Editing Decision

Status: active
Last reviewed: 2026-05-23

## Decision

Overlay-aware variant editing is explicitly deferred for the first production
target. Variant overlays remain JSON-authored files, while the editor may load
and inspect resolved variants as read-only states.

This is an accepted production constraint for the first target, not an
unresolved data-loss bug. The current safe behavior is:

- Variant overlay files are authored under `characters/{id}/states/`, such as
  `characters/test_char/states/5H~level2.json`.
- Loading a character resolves each overlay against its base state and gives
  the resolved state a unique `id`.
- The State Editor and Frame Data Table select resolved variants by `id`, not
  only by gameplay `input`.
- Saving a resolved variant through the State Editor, MCP `update_move`, or
  backend `save_move` is rejected because the loaded value is a resolved
  snapshot, not the original overlay patch.
- Exports continue to include resolved variants. `json-blob` is the canonical
  first production handoff; `fspk` v1 is an optional compact runtime subset.

## Supported Workflow

Create a base state:

```json
{
  "input": "5H",
  "name": "Standing Heavy",
  "startup": 12,
  "active": 4,
  "recovery": 24,
  "damage": 80
}
```

Create an overlay file at `characters/test_char/states/5H~level2.json`:

```json
{
  "input": "5H",
  "base": "5H",
  "name": "Standing Heavy Level 2",
  "damage": 100,
  "meter_gain": 12
}
```

Reload the project, inspect the resolved state as `5H~level2`, and export:

```bash
cargo run --bin framesmith-cli -- export --project .. --character test_char --adapter json-blob --pretty --out ../exports/test_char.json
cargo run --bin framesmith-cli -- export --project .. --character test_char --adapter fspk --out ../exports/test_char.fspk
```

## Why This Is Deferred

Overlay-aware editing is not just a form-save feature. To be safe, the editor
must know which fields are inherited, which fields are explicitly overridden,
and when an inherited value should be removed. Writing the resolved state back
to `5H~level2.json` would duplicate base data into the overlay and could make a
future base-state balance change silently stop affecting the variant.

The first production target can accept JSON overlay authoring because:

- The runtime handoff receives resolved output and does not consume inheritance.
- Data-loss protection is already enforced by selection identity and save
  blocking.
- Variant overlays are small, readable JSON files that can be reviewed in
  source control.
- The canonical production handoff is `json-blob`, so no FSPK v1 limitation
  blocks variant testing or gameplay integration.

## Reopen Criteria

Implement overlay-aware variant editing before a production release if any of
these become true:

- The target game needs non-programmer designers to edit variant overlays
  without touching JSON.
- The team needs to compare inherited and overridden fields visually in the
  editor.
- Variant patches need field-level delete semantics beyond the current JSON
  overlay behavior.
- A target-game review rejects JSON-only variant authoring as an unacceptable
  workflow risk.

When reopened, the implementation must:

- Open the overlay file directly instead of the resolved snapshot.
- Show inherited, overridden, and deleted fields distinctly.
- Save only the overlay diff.
- Include tests proving a fully resolved state cannot be serialized back into an
  overlay patch.

## Acceptance For First Target

For the first production target, variant authoring is accepted when all of
these stay true:

- Resolved variants are selectable and inspectable by unique `id`.
- Resolved variants cannot overwrite base state files or overlay files through
  `save_move`.
- The JSON overlay workflow is documented in `data-formats.md`.
- Export tests continue to prove resolved variants appear in production
  handoff output.
