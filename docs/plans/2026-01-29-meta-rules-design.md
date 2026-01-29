# Meta Rules System Design

**Status:** Draft
**Date:** 2026-01-29

REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

## Overview

A rule-based system for defining project-wide and per-character defaults and validation constraints. Eliminates copy-paste of common patterns like gatling chains, cancel rules, and charge times.

## File Structure

```
framesmith.rules.json           # Project-wide (required)
characters/
  glitch/
    rules.json                  # Character overrides (optional)
    moves/
      ...
```

## Basic Format

```json
{
  "version": 1,
  "apply": [
    { "match": { ... }, "set": { ... } }
  ],
  "validate": [
    { "match": { ... }, "require": { ... }, "severity": "error" }
  ]
}
```

## Rule Evaluation Order

1. Project `apply` rules run first (top to bottom)
2. Character `apply` rules run second (can override)
3. Move's explicit values always win (never overwritten by rules)
4. After final values computed, `validate` rules check constraints

## Inheritance & Override Behavior

When a character rule has the same `match` criteria as a project rule, the character rule completely replaces it. Non-conflicting rules from both files apply.

---

## Match Syntax

### Basic Matchers

```json
{
  "match": {
    "type": "normal",
    "button": "L",
    "guard": "mid",
    "tags": ["projectile"]
  }
}
```

All conditions must be true (AND logic). Use arrays for OR within a field:

```json
{ "match": { "type": ["normal", "special"] } }
```

### Input Pattern Matching (Glob-Style)

| Pattern | Matches | Use case |
|---------|---------|----------|
| `236*` | `236P`, `236K`, `236PP` | All QCF moves |
| `[*]*` | `[4]6P`, `[2]8K` | All charge moves |
| `*P` | `5P`, `236P`, `623P` | All punch enders |
| `5?` | `5L`, `5M`, `5H` | All standing normals |
| `236236*` | `236236P`, `236236K` | All double-QCF (supers) |

### Match-All Rule

```json
{ "match": {} }
```

### Tag Matching

```json
{ "match": { "tags": ["invincible"] } }      // Has "invincible" tag
{ "match": { "tags": ["rekka", "ender"] } }  // Has BOTH tags
```

---

## Apply Rules (Setting Properties)

### Cancel Properties

```json
{
  "match": { "type": "normal" },
  "set": {
    "cancel_into": ["special", "super"],
    "cancel_on": ["hit", "block"]
  }
}
```

### Gatling Chains

```json
{
  "match": { "button": "L" },
  "set": {
    "chain_to": ["M", "H"],
    "chain_on": ["hit", "block", "whiff"]
  }
}
```

### Charge Moves

```json
{
  "match": { "input": "[*]*" },
  "set": {
    "charge_frames": 45
  }
}
```

### Meter Defaults

```json
{
  "match": { "type": "normal" },
  "set": {
    "meter_gain": { "hit": 5, "whiff": 2 }
  }
}
```

### Frame Data Defaults

```json
{
  "match": {},
  "set": {
    "hitstop": 8,
    "pushback": { "hit": 5, "block": 8 }
  }
}
```

### Rule Stacking

Multiple rules can stack (later rules override earlier):

```json
{
  "apply": [
    { "match": {}, "set": { "hitstop": 8 } },
    { "match": { "type": "special" }, "set": { "hitstop": 10 } },
    { "match": { "tags": ["super"] }, "set": { "hitstop": 15 } }
  ]
}
```

---

## Validation Rules

### Basic Validation

```json
{
  "validate": [
    {
      "match": { "type": "normal" },
      "require": { "startup": { "min": 4 } },
      "severity": "error",
      "message": "Normals cannot be faster than 4f"
    }
  ]
}
```

### Numeric Constraints

```json
{ "require": { "startup": { "min": 4 } } }           // >= 4
{ "require": { "startup": { "max": 20 } } }          // <= 20
{ "require": { "startup": { "min": 4, "max": 20 } }} // 4-20
{ "require": { "damage": { "min": 1 } } }            // Must deal damage
```

### Existence Checks

```json
{ "require": { "meter_cost": { "exists": true } } }  // Field must be set
{ "require": { "animation": { "exists": true } } }   // Must have animation
```

### Value Matching

```json
{ "require": { "type": { "equals": "super" } } }     // Must be super type
{ "require": { "guard": { "in": ["mid", "low"] } } } // Must be mid or low
```

### Severity Levels

- `"error"` - Blocks export, shows red in UI
- `"warning"` - Allows export, shows yellow in UI

---

## Complete Example

### Project-wide rules (`framesmith.rules.json`)

```json
{
  "version": 1,
  "apply": [
    {
      "match": {},
      "set": { "hitstop": 8, "pushback": { "hit": 5, "block": 8 } }
    },
    {
      "match": { "type": "normal" },
      "set": { "cancel_into": ["special", "super"], "cancel_on": ["hit", "block"] }
    },
    {
      "match": { "button": "L" },
      "set": { "chain_to": ["M"], "chain_on": ["hit", "block", "whiff"] }
    },
    {
      "match": { "button": "M" },
      "set": { "chain_to": ["H"], "chain_on": ["hit", "block"] }
    },
    {
      "match": { "input": "[*]*" },
      "set": { "charge_frames": 45 }
    }
  ],
  "validate": [
    {
      "match": { "type": "normal" },
      "require": { "startup": { "min": 4 } },
      "severity": "error",
      "message": "No normal faster than 4f"
    },
    {
      "match": { "type": "super" },
      "require": { "meter_cost": { "min": 50 } },
      "severity": "error"
    },
    {
      "match": {},
      "require": { "animation": { "exists": true } },
      "severity": "warning",
      "message": "Move missing animation reference"
    }
  ]
}
```

### Character override (`characters/glitch/rules.json`)

```json
{
  "version": 1,
  "apply": [
    {
      "match": { "type": "normal" },
      "set": { "cancel_into": ["special"] }
    }
  ]
}
```

*Glitch's normals can't cancel into supers directly - overrides project rule.*
