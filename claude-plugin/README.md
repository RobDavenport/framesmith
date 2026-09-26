# Framesmith AI Plugin

AI coding assistant plugin for Framesmith — the engine-agnostic fighting game character authoring tool.

The Framesmith checkout is required: [AGENTS.md](../AGENTS.md) owns repo workflow and invariants, while the skills route to canonical docs and source. Load only the skill/reference needed for the task; bundled reference paths are kept as links rather than duplicate schema/API manuals.

## Skills

- **character-authoring** — State definitions, hitboxes, hurtboxes, cancel tables, one-file-per-state JSON format
- **framesmith-development** — Project setup, Tauri + SvelteKit architecture, CLI, testing, repo structure
- **fspk-export** — FSPK binary format, zero-copy parsing, runtime APIs, game engine integration

## Agents

These are opt-in Claude Code compatibility roles, not automatic delegation instructions. They inherit the host's selected model and require an explicit user request plus host permission. Astra can do the same work directly using the skills; installing the plugin does not select an Astra model or change reasoning effort.

- **character-designer** — Designs complete characters: states, frame data, cancel tables, hitboxes
- **state-debugger** — Diagnoses validation errors, export failures, and runtime issues

## Installation

### Claude Code

Add to your settings (`.claude/settings.local.json`):

```json
{
  "enabledPlugins": {
    "framesmith@framesmith/claude-plugin": true
  }
}
```

### Other AI Coding Assistants

Read the repo's [AGENTS.md](../AGENTS.md), then open only the relevant `SKILL.md`. Resolve linked docs against the Framesmith checkout; do not paste the whole plugin into the system prompt:

- `skills/character-authoring/SKILL.md`
- `skills/framesmith-development/SKILL.md`
- `skills/fspk-export/SKILL.md`

## Index

See [INDEX.md](INDEX.md) for a complete catalog of skills, agents, and trigger keywords.

## License

MIT
