# Framesmith AI Plugin

AI coding assistant plugin for Framesmith — the engine-agnostic fighting game character authoring tool.

## Skills

- **character-authoring** — State definitions, hitboxes, hurtboxes, cancel tables, one-file-per-state JSON format
- **framesmith-development** — Project setup, Tauri + SvelteKit architecture, CLI, testing, repo structure
- **fspk-export** — FSPK binary format, zero-copy parsing, runtime APIs, game engine integration

## Agents

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

Open the relevant `SKILL.md` files and paste the guidance into your assistant:

- `skills/character-authoring/SKILL.md`
- `skills/framesmith-development/SKILL.md`
- `skills/fspk-export/SKILL.md`

## Index

See [INDEX.md](INDEX.md) for a complete catalog of skills, agents, and trigger keywords.

## License

MIT
