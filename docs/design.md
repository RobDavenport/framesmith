# Framesmith Design Notes

**Status:** Active
**Last reviewed:** 2026-01-30

This document describes Framesmith’s intended shape and the current implementation boundaries. For the on-disk JSON formats, use `docs/data-formats.md` (canonical for file layout) and `docs/rules-spec.md` (canonical for rules semantics).

## Overview

Framesmith is an engine-agnostic fighting game character authoring tool. It edits character data on disk as a directory of JSON files and exports that data into runtime-friendly formats.

The core idea is “git-friendly authoring”:

- One character = one folder
- One move = one file
- Cancels live in a central table for easy visualization
- Validation and defaults are configurable via rules files

## Current Product Scope

Implemented (today):

- Desktop editor (Tauri + Svelte)
- Projects (open/create): a folder containing `framesmith.rules.json` and `characters/`
- Character management: create/clone/delete
- Move management: create, edit, save
- Views: Character Overview, Frame Data Table, Move Editor, Cancel Graph
- Rules system: apply defaults + validate moves; optional registry for resources/events
- Export adapters:
  - `json-blob` (single JSON blob)
  - `zx-fspack` (compact binary pack)
- MCP server for programmatic workflows and LLM integration

Not implemented (yet):

- Animation preview / hitbox overlay editing (UI currently shows placeholders)
- Interactive editing of cancel routes (graph is visualization-only)
- Breakpoint Rust adapter (`breakpoint-rust` is currently a stub)

## Data Model

Canonical Rust types:

- Character: `src-tauri/src/schema/mod.rs`
- Rules: `src-tauri/src/rules/mod.rs`
- ZX FSPK format constants: `src-tauri/src/codegen/zx_fspack_format.rs`

On disk, a character folder is primarily:

- `character.json`
- `cancel_table.json`
- `moves/*.json`
- optional `rules.json` (character-level rules overrides)

See `docs/data-formats.md` for details.

## Rules + Validation

Rules files do two things:

1. Apply defaults (`apply[]`) to moves based on match criteria.
2. Validate (`validate[]`) moves based on constraints.

Optionally, `registry` provides:

- Known resource IDs
- Known notification event IDs with arg schemas and allowed contexts

This registry is used for “no silent errors” validation when moves reference resources/events.

Canonical semantics are in `docs/rules-spec.md`.

## Export System

Exports are run through the same validation/defaulting pipeline used by saving:

- Load project rules (`<project>/framesmith.rules.json`) and character rules (`<project>/characters/<id>/rules.json`)
- Validate character resources against the merged registry
- Validate moves (built-in + rules)
- Apply defaults
- Export the resolved character

Adapters:

- `json-blob`: emits a single JSON blob containing resolved character + moves.
- `zx-fspack`: emits a `.fspk` binary pack for constrained runtimes (see `docs/zx-fspack.md`).
- `breakpoint-rust`: reserved name; currently returns an error.

## MCP Server

The MCP server (`src-tauri/src/bin/mcp.rs`) exposes tools for listing characters, reading/writing moves, inspecting rules schema, and more.

See `docs/mcp-server.md`.

## Design Principles

- Favor explicit data over implicit conventions.
- Keep authoring formats diffable and merge-friendly.
- Keep validation centralized and reusable (UI, exporter, MCP).
- Keep runtime formats compact and parsing-friendly.
