---
name: character-authoring
description: >-
  Use when authoring Framesmith character data. Work with state JSON,
  hit/hurt/push boxes, variants, resources and cancel rules.
license: MIT
compatibility: Requires the Framesmith checkout and app, CLI or MCP for validation.
metadata:
  author: nethercore-systems
  version: "1.1.0"
---

# Framesmith Character Authoring

Follow [AGENTS.md](../../../AGENTS.md) for scope and invariants. Read the target project's `framesmith.rules.json`, character files and affected states before authoring; reuse its notation, tags, resource names and conventions.

1. Identify the requested move set or edit. A single-move request does not require a new character or a complete balance pass.
2. Use [data formats](references/data-formats.md) for the fields being changed and [cancel tables](references/cancel-tables.md) for transition rules. Adapt the checked-in [test character](../../../characters/test_char/) rather than inventing a schema.
3. Keep one file per state. Preserve variant overlay identity and the read-only resolved-state guard; use the linked variant policy for overlay edits.
4. Check frame windows, tag/target references, resource definitions and the selected adapter's supported fields through existing validation. Exercise the affected route in training when claiming in-app behavior. Passing validation does not establish balance or game feel.

Read [global states](../../../docs/global-states.md), [movement](../../../docs/movement-reference.md) or the [authoring guide](../../../docs/character-authoring-guide.md) only for those mechanics. The [handoff decision](../../../docs/production-handoff-decision.md) governs which authored fields reach the game.
