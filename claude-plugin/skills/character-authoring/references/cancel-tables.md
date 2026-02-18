# Framesmith Cancel Tables Reference

The cancel table defines all allowed state transitions for a character. It lives
at `characters/<id>/cancel_table.json`.

## Cancel Table JSON Structure

```json
{
  "tag_rules": [
    { "from": "normal", "to": "special", "on": ["hit", "block"] }
  ],
  "chains": {
    "5L": ["5L", "5M"],
    "5M": ["5H"]
  },
  "special_cancels": ["5L", "5M", "5H"],
  "super_cancels": ["5H"],
  "jump_cancels": ["5H"],
  "deny": {
    "2H": ["66"]
  }
}
```

## Tag-Based Rules (Recommended)

Tag rules are the primary cancel mechanism. They match on state tags rather than
explicit input names, making them composable and maintainable.

### Structure

```json
{
  "from": "<tag-or-any>",
  "to": "<tag-or-any>",
  "on": "<condition>"
}
```

### Fields

| Field | Type | Description |
|-------|------|-------------|
| `from` | string | Source state must have this tag. `"any"` = wildcard. |
| `to` | string | Target state must have this tag. `"any"` = wildcard. |
| `on` | string or string[] | When cancel is allowed (see conditions below). |

### Conditions

| Condition | Meaning |
|-----------|---------|
| `"always"` | Cancel allowed at any time during the state |
| `"hit"` | Only after the state has hit the opponent |
| `"block"` | Only after the state was blocked |
| `"whiff"` | Only if the state did not connect |
| `["hit", "block"]` | Array = OR logic (hit or block) |

### Evaluation

Cancel checks follow this priority order:

1. **Deny rules** -- if a deny exists for the from/to pair, cancel is blocked
2. **Explicit chains** -- checked first for exact input-to-input routes
3. **Tag rules** -- evaluated in order; first matching rule wins

A tag rule matches when:
- The source state has the `from` tag (or `from` is `"any"`)
- The target state has the `to` tag (or `to` is `"any"`)
- The current game condition matches `on`

### Common Tag Patterns

**System states cancel freely:**
```json
{ "from": "system", "to": "any", "on": "always" }
```

**Movement cancels into anything on complete:**
```json
{ "from": "movement", "to": "any", "on": "always" }
```

**Normal-to-special cancel (gatling):**
```json
{ "from": "normal", "to": "special", "on": ["hit", "block"] }
```

**Special-to-super cancel:**
```json
{ "from": "special", "to": "super", "on": ["hit", "block"] }
```

## Chain Cancels (Explicit Routes)

Chains define exact input-to-input cancel routes. Use these for target combos,
rekka sequences, and other specific move-to-move relationships.

```json
{
  "chains": {
    "5L": ["5L", "5M"],
    "5M": ["5H"],
    "236K": ["236K_K"],
    "236K_K": ["236K_K_K"]
  }
}
```

Each key is the source state input. The value is an array of allowed target inputs.
Chains are checked before tag rules, so they can override tag-based behavior.

### Rekka Example

A 3-hit rekka sequence where each hit chains to the next:

```json
{
  "chains": {
    "236P": ["236P_2"],
    "236P_2": ["236P_3"]
  }
}
```

The states `236P_2` and `236P_3` should have `parent: "236P"` and
`parent: "236P_2"` respectively.

### Target Combo Example

Specific normal-to-normal routes beyond the standard chain:

```json
{
  "chains": {
    "5M": ["6H"],
    "2M": ["5H"]
  }
}
```

## Category Cancels (Legacy)

These fields provide a simpler (but less flexible) cancel model. They list which
states are eligible for each cancel category.

```json
{
  "special_cancels": ["5L", "5M", "5H", "2L", "2M"],
  "super_cancels": ["5H", "2H"],
  "jump_cancels": ["5H"]
}
```

| Field | Meaning |
|-------|---------|
| `special_cancels` | These states can cancel into special moves |
| `super_cancels` | These states can cancel into super moves |
| `jump_cancels` | These states can cancel into jump |

These can coexist with tag rules. Tag rules are the recommended approach for new
projects; legacy fields are supported for backward compatibility.

## Cancel Deny Rules

Deny rules explicitly block specific transitions, overriding all other cancel
mechanisms. Denies are checked first and take absolute priority.

```json
{
  "deny": {
    "2H": ["66", "44"],
    "236236P": ["j.8"]
  }
}
```

Each key is the source state input. The value is an array of target inputs that
are blocked from that source, regardless of tag rules or chains.

### Use Cases

- Prevent sweeps from dash-canceling: `"2H": ["66", "44"]`
- Prevent supers from jump-canceling: `"236236P": ["j.8"]`
- Block degenerate loops: `"5L": ["5L"]` (if self-chain would be broken)

## Complete Cancel Table Examples

### Standard Fighting Game

```json
{
  "tag_rules": [
    { "from": "system", "to": "any", "on": "always" },
    { "from": "movement", "to": "any", "on": "always" },
    { "from": "normal", "to": "special", "on": ["hit", "block"] },
    { "from": "normal", "to": "super", "on": ["hit", "block"] },
    { "from": "special", "to": "super", "on": ["hit", "block"] }
  ],
  "chains": {
    "5L": ["5L", "5M", "2L"],
    "5M": ["5H"],
    "2L": ["2L", "5M"],
    "j.L": ["j.L", "j.M"],
    "j.M": ["j.H"]
  },
  "deny": {}
}
```

### Minimal Movement-Only

For a character that only needs movement cancel rules (no attacks yet):

```json
{
  "tag_rules": [
    { "from": "system", "to": "any", "on": "always" }
  ],
  "chains": {
    "0_idle": ["0_walk_forward", "0_walk_backward", "1_crouch", "66", "44"],
    "0_walk_forward": ["0_idle", "0_walk_backward", "1_crouch", "66"],
    "0_walk_backward": ["0_idle", "0_walk_forward", "1_crouch", "44"],
    "1_crouch": ["0_idle", "0_walk_forward", "0_walk_backward"]
  },
  "deny": {}
}
```

### Tag-Based with Deny Overrides

```json
{
  "tag_rules": [
    { "from": "system", "to": "any", "on": "always" },
    { "from": "normal", "to": "special", "on": ["hit", "block"] },
    { "from": "normal", "to": "super", "on": ["hit", "block"] },
    { "from": "special", "to": "super", "on": ["hit", "block"] },
    { "from": "normal", "to": "normal", "on": ["hit", "block"] }
  ],
  "chains": {
    "236K": ["236K_K"],
    "236K_K": ["236K_K_K"]
  },
  "deny": {
    "2H": ["66", "44"],
    "5H": ["5L"]
  }
}
```

## Tag Assignment Guidelines

For cancel rules to work, states need appropriate tags. Common patterns:

| State Type | Recommended Tags |
|------------|-----------------|
| Idle, walk, crouch | `system`, `movement` |
| Dashes | `movement` |
| Standing normals | `normal`, `light`/`medium`/`heavy` |
| Crouching normals | `normal`, `low` |
| Air normals | `normal`, `aerial` |
| Specials (236/214) | `special` |
| Supers (236236) | `super` |
| Throws | `throw` |
| Hitstun, blockstun | `system`, `reaction` |

Tags are lowercase alphanumeric + underscores only (validated by the Tag newtype).
