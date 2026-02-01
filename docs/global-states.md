# Global States

Global states are shared state definitions that live at the project level and can be opted into by any character.

## Use Cases

- **System mechanics**: Burst, Roman Cancel, hard knockdown
- **Common states**: Idle, walk, crouch, jump
- **Character archetypes**: Shared specials for similar characters

## Project Structure

```
project/
  globals/
    states/
      burst.json
      idle.json
      walk_forward.json
  characters/
    ryu/
      globals.json      # Which globals this character uses
      states/
        5L.json         # Local states
```

## Character Opt-In

Characters opt into globals via `globals.json`:

```json
{
  "includes": [
    { "state": "burst", "as": "burst" },
    { "state": "idle", "as": "idle", "override": { "animation": "ryu_idle" } },
    { "state": "walk_forward", "as": "66" }
  ]
}
```

### Fields

- `state`: Name of the global state file (without `.json`)
- `as`: Input name for this character (allows renaming)
- `override`: Optional field overrides

## Override Behavior

Overrides use **shallow merge** semantics:

- Top-level fields are replaced entirely
- Arrays are replaced (not concatenated)
- Nested objects are replaced as units
- `null` removes a field

Example:
```json
// Global: { "damage": 100, "tags": ["normal", "ground"] }
// Override: { "damage": 80 }
// Result: { "damage": 80, "tags": ["normal", "ground"] }
```

## Validation

The system validates:

- Global state exists
- Alias doesn't conflict with local states
- No duplicate aliases
- Unknown override fields generate warnings

## MCP Tools

- `list_global_states`: List all project globals
- `get_global_state`: Get a specific global
- `create_global_state`: Create new global
- `update_global_state`: Update existing global
- `delete_global_state`: Delete (checks references)

## UI

- **Globals Manager**: Browse and edit project-wide globals
- **Character Globals Editor**: Configure includes per character
- **State List**: Shows a globe indicator for global-derived states
