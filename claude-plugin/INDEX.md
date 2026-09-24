# framesmith — Skills & Agents Index

Start with [AGENTS.md](../AGENTS.md); select only the skill/reference needed for the task. Reference files link to the maintained repository docs.

## Skills

| Name | Trigger Keywords | References |
|------|-----------------|------------|
| [character-authoring](skills/character-authoring/SKILL.md) | states, hitboxes, hurtboxes, cancel tables, frame data | 2 |
| [framesmith-development](skills/framesmith-development/SKILL.md) | Tauri, SvelteKit, CLI, export, testing | 2 |
| [fspk-export](skills/fspk-export/SKILL.md) | FSPK, binary format, runtime, zero-copy, PackView | 2 |

## Agents

Opt-in roles only: explicit user request and host delegation permission required. No automatic model routing.

| Name | Model | Purpose |
|------|-------|---------|
| [character-designer](agents/character-designer.md) | inherit | Author the requested character/move data |
| [state-debugger](agents/state-debugger.md) | inherit | Diagnose validation errors, export failures, and runtime issues |
