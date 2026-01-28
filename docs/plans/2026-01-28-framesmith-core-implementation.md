# Framesmith Core Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement the core Framesmith editor with character loading, frame data table, move editing, and export functionality.

**Architecture:** Tauri desktop app with Rust backend for file I/O and export codegen, Svelte 5 frontend with reactive stores. Character data stored as JSON directory structure. Four main views: Character Overview, Frame Data Table, Move Editor, Cancel Graph.

**Tech Stack:** Tauri 2, Svelte 5, TypeScript, Threlte (Three.js), Rust with serde_json

---

## Phase 1: Foundation (File I/O + Basic UI Shell)

### Task 1: Create Sample Character Data

**Files:**
- Create: `characters/glitch/character.json`
- Create: `characters/glitch/moves/5L.json`
- Create: `characters/glitch/moves/5M.json`
- Create: `characters/glitch/moves/5H.json`
- Create: `characters/glitch/cancel_table.json`
- Create: `characters/glitch/hurtboxes.json`
- Create: `characters/glitch/assets.json`

**Step 1: Create character.json**

```json
{
  "id": "glitch",
  "name": "GLITCH",
  "archetype": "rushdown",
  "health": 1000,
  "walk_speed": 4.5,
  "back_walk_speed": 3.2,
  "jump_height": 120,
  "jump_duration": 45,
  "dash_distance": 80,
  "dash_duration": 18
}
```

**Step 2: Create 5L.json (Standing Light)**

```json
{
  "input": "5L",
  "name": "Standing Light",
  "startup": 7,
  "active": 3,
  "recovery": 8,
  "damage": 30,
  "hitstun": 17,
  "blockstun": 11,
  "hitstop": 6,
  "guard": "mid",
  "hitboxes": [
    { "frames": [7, 9], "box": { "x": 0, "y": -40, "w": 30, "h": 16 } }
  ],
  "hurtboxes": [
    { "frames": [0, 6], "box": { "x": -10, "y": -60, "w": 30, "h": 60 } },
    { "frames": [7, 17], "box": { "x": 0, "y": -55, "w": 35, "h": 55 } }
  ],
  "pushback": { "hit": 2, "block": 2 },
  "meter_gain": { "hit": 5, "whiff": 2 },
  "animation": "stand_light"
}
```

**Step 3: Create 5M.json (Standing Medium)**

```json
{
  "input": "5M",
  "name": "Standing Medium",
  "startup": 10,
  "active": 4,
  "recovery": 14,
  "damage": 60,
  "hitstun": 20,
  "blockstun": 14,
  "hitstop": 8,
  "guard": "mid",
  "hitboxes": [
    { "frames": [10, 13], "box": { "x": 5, "y": -50, "w": 40, "h": 20 } }
  ],
  "hurtboxes": [
    { "frames": [0, 27], "box": { "x": -10, "y": -60, "w": 35, "h": 60 } }
  ],
  "pushback": { "hit": 4, "block": 3 },
  "meter_gain": { "hit": 8, "whiff": 4 },
  "animation": "stand_medium"
}
```

**Step 4: Create 5H.json (Standing Heavy)**

```json
{
  "input": "5H",
  "name": "Standing Heavy",
  "startup": 14,
  "active": 5,
  "recovery": 20,
  "damage": 90,
  "hitstun": 24,
  "blockstun": 18,
  "hitstop": 10,
  "guard": "mid",
  "hitboxes": [
    { "frames": [14, 18], "box": { "x": 10, "y": -55, "w": 50, "h": 25 } }
  ],
  "hurtboxes": [
    { "frames": [0, 38], "box": { "x": -12, "y": -65, "w": 40, "h": 65 } }
  ],
  "pushback": { "hit": 6, "block": 5 },
  "meter_gain": { "hit": 12, "whiff": 6 },
  "animation": "stand_heavy"
}
```

**Step 5: Create cancel_table.json**

```json
{
  "chains": {
    "5L": ["5L", "5M"],
    "5M": ["5H"]
  },
  "special_cancels": ["5L", "5M", "5H"],
  "super_cancels": ["5H"],
  "jump_cancels": ["5H"]
}
```

**Step 6: Create hurtboxes.json**

```json
{
  "stand": { "x": -15, "y": -70, "w": 30, "h": 70 },
  "crouch": { "x": -18, "y": -45, "w": 36, "h": 45 },
  "airborne": { "x": -12, "y": -55, "w": 24, "h": 55 }
}
```

**Step 7: Create assets.json**

```json
{
  "mesh": null,
  "textures": {},
  "animations": {}
}
```

**Step 8: Commit**

```bash
git add characters/
git commit -m "$(cat <<'EOF'
feat: add sample character data for glitch

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 2: Implement Rust Character Loading Commands

**Files:**
- Modify: `src-tauri/src/lib.rs`
- Create: `src-tauri/src/commands.rs`

**Step 1: Create commands.rs with load_characters**

```rust
use crate::schema::{Character, Move, CancelTable};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, serde::Serialize)]
pub struct CharacterData {
    pub character: Character,
    pub moves: Vec<Move>,
    pub cancel_table: CancelTable,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CharacterSummary {
    pub id: String,
    pub name: String,
    pub archetype: String,
    pub move_count: usize,
}

#[tauri::command]
pub fn list_characters(characters_dir: String) -> Result<Vec<CharacterSummary>, String> {
    let path = Path::new(&characters_dir);
    if !path.exists() {
        return Ok(vec![]);
    }

    let mut summaries = vec![];
    for entry in fs::read_dir(path).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let char_path = entry.path();
        if !char_path.is_dir() {
            continue;
        }

        let char_file = char_path.join("character.json");
        if !char_file.exists() {
            continue;
        }

        let content = fs::read_to_string(&char_file).map_err(|e| e.to_string())?;
        let character: Character = serde_json::from_str(&content).map_err(|e| e.to_string())?;

        let moves_dir = char_path.join("moves");
        let move_count = if moves_dir.exists() {
            fs::read_dir(&moves_dir)
                .map(|dir| dir.filter(|e| e.is_ok()).count())
                .unwrap_or(0)
        } else {
            0
        };

        summaries.push(CharacterSummary {
            id: character.id.clone(),
            name: character.name,
            archetype: character.archetype,
            move_count,
        });
    }

    Ok(summaries)
}

#[tauri::command]
pub fn load_character(characters_dir: String, character_id: String) -> Result<CharacterData, String> {
    let char_path = Path::new(&characters_dir).join(&character_id);
    if !char_path.exists() {
        return Err(format!("Character '{}' not found", character_id));
    }

    // Load character.json
    let char_file = char_path.join("character.json");
    let content = fs::read_to_string(&char_file).map_err(|e| e.to_string())?;
    let character: Character = serde_json::from_str(&content).map_err(|e| e.to_string())?;

    // Load all moves
    let moves_dir = char_path.join("moves");
    let mut moves = vec![];
    if moves_dir.exists() {
        for entry in fs::read_dir(&moves_dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let move_path = entry.path();
            if move_path.extension().map(|e| e == "json").unwrap_or(false) {
                let content = fs::read_to_string(&move_path).map_err(|e| e.to_string())?;
                let mv: Move = serde_json::from_str(&content).map_err(|e| e.to_string())?;
                moves.push(mv);
            }
        }
    }

    // Load cancel table
    let cancel_file = char_path.join("cancel_table.json");
    let cancel_table: CancelTable = if cancel_file.exists() {
        let content = fs::read_to_string(&cancel_file).map_err(|e| e.to_string())?;
        serde_json::from_str(&content).map_err(|e| e.to_string())?
    } else {
        CancelTable {
            chains: HashMap::new(),
            special_cancels: vec![],
            super_cancels: vec![],
            jump_cancels: vec![],
        }
    };

    Ok(CharacterData {
        character,
        moves,
        cancel_table,
    })
}
```

**Step 2: Update lib.rs to register commands**

```rust
pub mod codegen;
pub mod commands;
pub mod schema;

use commands::{list_characters, load_character};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![list_characters, load_character])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

**Step 3: Run `cargo check` to verify compilation**

Run: `cd src-tauri && cargo check`
Expected: Compilation succeeds with no errors

**Step 4: Commit**

```bash
git add src-tauri/src/
git commit -m "$(cat <<'EOF'
feat: add Tauri commands for character loading

Implements list_characters and load_character commands for
reading character data from the filesystem.

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 3: Create TypeScript Types and Character Store

**Files:**
- Create: `src/lib/types.ts`
- Create: `src/lib/stores/character.svelte.ts`

**Step 1: Create types.ts**

```typescript
export interface Character {
  id: string;
  name: string;
  archetype: string;
  health: number;
  walk_speed: number;
  back_walk_speed: number;
  jump_height: number;
  jump_duration: number;
  dash_distance: number;
  dash_duration: number;
}

export interface Move {
  input: string;
  name: string;
  startup: number;
  active: number;
  recovery: number;
  damage: number;
  hitstun: number;
  blockstun: number;
  hitstop: number;
  guard: "high" | "mid" | "low" | "unblockable";
  hitboxes: FrameHitbox[];
  hurtboxes: FrameHitbox[];
  pushback: Pushback;
  meter_gain: MeterGain;
  animation: string;
}

export interface FrameHitbox {
  frames: [number, number];
  box: Rect;
}

export interface Rect {
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface Pushback {
  hit: number;
  block: number;
}

export interface MeterGain {
  hit: number;
  whiff: number;
}

export interface CancelTable {
  chains: Record<string, string[]>;
  special_cancels: string[];
  super_cancels: string[];
  jump_cancels: string[];
}

export interface CharacterData {
  character: Character;
  moves: Move[];
  cancel_table: CancelTable;
}

export interface CharacterSummary {
  id: string;
  name: string;
  archetype: string;
  move_count: number;
}
```

**Step 2: Create character.svelte.ts store**

```typescript
import { invoke } from "@tauri-apps/api/core";
import type { CharacterData, CharacterSummary, Move } from "$lib/types";

// Reactive state using Svelte 5 runes
let characterList = $state<CharacterSummary[]>([]);
let currentCharacter = $state<CharacterData | null>(null);
let selectedMoveInput = $state<string | null>(null);
let loading = $state(false);
let error = $state<string | null>(null);

const CHARACTERS_DIR = "characters";

export function getCharacterList() {
  return characterList;
}

export function getCurrentCharacter() {
  return currentCharacter;
}

export function getSelectedMove(): Move | null {
  if (!currentCharacter || !selectedMoveInput) return null;
  return currentCharacter.moves.find((m) => m.input === selectedMoveInput) ?? null;
}

export function getSelectedMoveInput() {
  return selectedMoveInput;
}

export function isLoading() {
  return loading;
}

export function getError() {
  return error;
}

export async function loadCharacterList(): Promise<void> {
  loading = true;
  error = null;
  try {
    characterList = await invoke<CharacterSummary[]>("list_characters", {
      charactersDir: CHARACTERS_DIR,
    });
  } catch (e) {
    error = String(e);
  } finally {
    loading = false;
  }
}

export async function selectCharacter(characterId: string): Promise<void> {
  loading = true;
  error = null;
  selectedMoveInput = null;
  try {
    currentCharacter = await invoke<CharacterData>("load_character", {
      charactersDir: CHARACTERS_DIR,
      characterId,
    });
  } catch (e) {
    error = String(e);
    currentCharacter = null;
  } finally {
    loading = false;
  }
}

export function selectMove(input: string): void {
  selectedMoveInput = input;
}

export function clearSelection(): void {
  currentCharacter = null;
  selectedMoveInput = null;
}
```

**Step 3: Run `npm run check` to verify TypeScript**

Run: `npm run check`
Expected: Type check passes with no errors

**Step 4: Commit**

```bash
git add src/lib/
git commit -m "$(cat <<'EOF'
feat: add TypeScript types and character store

Adds complete type definitions matching Rust schema and
reactive Svelte 5 store for character state management.

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 4: Create App Shell with Navigation

**Files:**
- Modify: `src/routes/+page.svelte`
- Create: `src/lib/components/Sidebar.svelte`
- Create: `src/lib/components/Header.svelte`
- Create: `src/app.css`

**Step 1: Create app.css with base styles**

```css
:root {
  --bg-primary: #1a1a2e;
  --bg-secondary: #16213e;
  --bg-tertiary: #0f3460;
  --text-primary: #eaeaea;
  --text-secondary: #a0a0a0;
  --accent: #e94560;
  --accent-hover: #ff6b6b;
  --border: #2a2a4a;
  --success: #4ade80;
  --warning: #fbbf24;

  font-family: "Inter", -apple-system, BlinkMacSystemFont, sans-serif;
  font-size: 14px;
  line-height: 1.5;
  color: var(--text-primary);
  background-color: var(--bg-primary);
}

* {
  box-sizing: border-box;
  margin: 0;
  padding: 0;
}

body {
  overflow: hidden;
}

.app-container {
  display: grid;
  grid-template-columns: 220px 1fr;
  grid-template-rows: 48px 1fr;
  height: 100vh;
  width: 100vw;
}

.header {
  grid-column: 1 / -1;
  background: var(--bg-secondary);
  border-bottom: 1px solid var(--border);
  display: flex;
  align-items: center;
  padding: 0 16px;
  gap: 16px;
}

.sidebar {
  background: var(--bg-secondary);
  border-right: 1px solid var(--border);
  overflow-y: auto;
  padding: 12px;
}

.main-content {
  overflow: auto;
  padding: 16px;
}

button {
  background: var(--bg-tertiary);
  color: var(--text-primary);
  border: 1px solid var(--border);
  padding: 8px 16px;
  border-radius: 4px;
  cursor: pointer;
  font-size: 13px;
  transition: all 0.15s ease;
}

button:hover {
  background: var(--accent);
  border-color: var(--accent);
}

button:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

input, select {
  background: var(--bg-primary);
  color: var(--text-primary);
  border: 1px solid var(--border);
  padding: 6px 10px;
  border-radius: 4px;
  font-size: 13px;
}

input:focus, select:focus {
  outline: none;
  border-color: var(--accent);
}
```

**Step 2: Create Header.svelte**

```svelte
<script lang="ts">
  interface Props {
    currentView: string;
    onViewChange: (view: string) => void;
  }

  let { currentView, onViewChange }: Props = $props();

  const views = [
    { id: "overview", label: "Overview" },
    { id: "frame-data", label: "Frame Data" },
    { id: "move-editor", label: "Move Editor" },
    { id: "cancel-graph", label: "Cancel Graph" },
  ];
</script>

<header class="header">
  <h1 class="title">Framesmith</h1>
  <nav class="nav">
    {#each views as view}
      <button
        class="nav-btn"
        class:active={currentView === view.id}
        onclick={() => onViewChange(view.id)}
      >
        {view.label}
      </button>
    {/each}
  </nav>
</header>

<style>
  .title {
    font-size: 18px;
    font-weight: 600;
    color: var(--accent);
  }

  .nav {
    display: flex;
    gap: 4px;
    margin-left: 24px;
  }

  .nav-btn {
    background: transparent;
    border: none;
    padding: 8px 12px;
    color: var(--text-secondary);
  }

  .nav-btn:hover {
    color: var(--text-primary);
    background: transparent;
  }

  .nav-btn.active {
    color: var(--accent);
    border-bottom: 2px solid var(--accent);
    border-radius: 0;
  }
</style>
```

**Step 3: Create Sidebar.svelte**

```svelte
<script lang="ts">
  import {
    getCharacterList,
    getCurrentCharacter,
    loadCharacterList,
    selectCharacter,
    isLoading,
  } from "$lib/stores/character.svelte";
  import { onMount } from "svelte";

  onMount(() => {
    loadCharacterList();
  });

  const characterList = $derived(getCharacterList());
  const currentCharacter = $derived(getCurrentCharacter());
  const loading = $derived(isLoading());
</script>

<aside class="sidebar">
  <div class="section">
    <h2 class="section-title">Characters</h2>
    {#if loading}
      <p class="loading">Loading...</p>
    {:else if characterList.length === 0}
      <p class="empty">No characters found</p>
    {:else}
      <ul class="character-list">
        {#each characterList as char}
          <li>
            <button
              class="character-btn"
              class:active={currentCharacter?.character.id === char.id}
              onclick={() => selectCharacter(char.id)}
            >
              <span class="name">{char.name}</span>
              <span class="meta">{char.archetype} · {char.move_count} moves</span>
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
</aside>

<style>
  .section-title {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-secondary);
    margin-bottom: 8px;
  }

  .loading, .empty {
    color: var(--text-secondary);
    font-size: 13px;
  }

  .character-list {
    list-style: none;
  }

  .character-btn {
    width: 100%;
    text-align: left;
    background: transparent;
    border: none;
    padding: 8px 10px;
    border-radius: 4px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .character-btn:hover {
    background: var(--bg-tertiary);
  }

  .character-btn.active {
    background: var(--accent);
  }

  .name {
    font-weight: 500;
  }

  .meta {
    font-size: 11px;
    color: var(--text-secondary);
  }

  .character-btn.active .meta {
    color: var(--text-primary);
  }
</style>
```

**Step 4: Update +page.svelte**

```svelte
<script lang="ts">
  import "$lib/../app.css";
  import Header from "$lib/components/Header.svelte";
  import Sidebar from "$lib/components/Sidebar.svelte";
  import { getCurrentCharacter } from "$lib/stores/character.svelte";

  let currentView = $state("overview");

  function handleViewChange(view: string) {
    currentView = view;
  }

  const currentCharacter = $derived(getCurrentCharacter());
</script>

<div class="app-container">
  <Header {currentView} onViewChange={handleViewChange} />
  <Sidebar />
  <main class="main-content">
    {#if !currentCharacter}
      <div class="placeholder">
        <p>Select a character from the sidebar to begin editing.</p>
      </div>
    {:else if currentView === "overview"}
      <h2>{currentCharacter.character.name}</h2>
      <p>Overview view coming soon...</p>
    {:else if currentView === "frame-data"}
      <p>Frame Data view coming soon...</p>
    {:else if currentView === "move-editor"}
      <p>Move Editor view coming soon...</p>
    {:else if currentView === "cancel-graph"}
      <p>Cancel Graph view coming soon...</p>
    {/if}
  </main>
</div>

<style>
  .placeholder {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-secondary);
  }
</style>
```

**Step 5: Run `npm run tauri dev` to test**

Run: `npm run tauri dev`
Expected: App opens, sidebar shows "GLITCH" character, clicking loads it

**Step 6: Commit**

```bash
git add src/
git commit -m "$(cat <<'EOF'
feat: add app shell with navigation and sidebar

Implements main layout with header navigation between views
and sidebar showing character list with selection.

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

## Phase 2: Frame Data Table View

### Task 5: Implement Frame Data Table View

**Files:**
- Create: `src/lib/views/FrameDataTable.svelte`
- Modify: `src/routes/+page.svelte`

**Step 1: Create FrameDataTable.svelte**

```svelte
<script lang="ts">
  import { getCurrentCharacter, selectMove } from "$lib/stores/character.svelte";
  import type { Move } from "$lib/types";

  interface Props {
    onEditMove: (input: string) => void;
  }

  let { onEditMove }: Props = $props();

  const characterData = $derived(getCurrentCharacter());
  const moves = $derived(characterData?.moves ?? []);

  let sortColumn = $state<keyof Move | "total" | "advantage_hit" | "advantage_block">("input");
  let sortDirection = $state<"asc" | "desc">("asc");
  let filterType = $state<string>("all");

  const filterOptions = [
    { value: "all", label: "All Moves" },
    { value: "normal", label: "Normals" },
    { value: "special", label: "Specials" },
  ];

  function isSpecialMove(input: string): boolean {
    return /\d{3,}/.test(input); // Contains 3+ consecutive digits (motion input)
  }

  function getTotal(move: Move): number {
    return move.startup + move.active + move.recovery;
  }

  function getAdvantageHit(move: Move): number {
    return move.hitstun - move.recovery;
  }

  function getAdvantageBlock(move: Move): number {
    return move.blockstun - move.recovery;
  }

  const filteredMoves = $derived.by(() => {
    let filtered = moves;
    if (filterType === "normal") {
      filtered = moves.filter((m) => !isSpecialMove(m.input));
    } else if (filterType === "special") {
      filtered = moves.filter((m) => isSpecialMove(m.input));
    }
    return filtered;
  });

  const sortedMoves = $derived.by(() => {
    return [...filteredMoves].sort((a, b) => {
      let aVal: number | string;
      let bVal: number | string;

      if (sortColumn === "total") {
        aVal = getTotal(a);
        bVal = getTotal(b);
      } else if (sortColumn === "advantage_hit") {
        aVal = getAdvantageHit(a);
        bVal = getAdvantageHit(b);
      } else if (sortColumn === "advantage_block") {
        aVal = getAdvantageBlock(a);
        bVal = getAdvantageBlock(b);
      } else {
        aVal = a[sortColumn];
        bVal = b[sortColumn];
      }

      if (typeof aVal === "string" && typeof bVal === "string") {
        return sortDirection === "asc" ? aVal.localeCompare(bVal) : bVal.localeCompare(aVal);
      }
      return sortDirection === "asc" ? Number(aVal) - Number(bVal) : Number(bVal) - Number(aVal);
    });
  });

  function toggleSort(column: typeof sortColumn) {
    if (sortColumn === column) {
      sortDirection = sortDirection === "asc" ? "desc" : "asc";
    } else {
      sortColumn = column;
      sortDirection = "asc";
    }
  }

  function handleRowClick(move: Move) {
    selectMove(move.input);
    onEditMove(move.input);
  }

  function formatAdvantage(value: number): string {
    return value >= 0 ? `+${value}` : String(value);
  }
</script>

<div class="frame-data-container">
  <div class="toolbar">
    <select bind:value={filterType}>
      {#each filterOptions as option}
        <option value={option.value}>{option.label}</option>
      {/each}
    </select>
    <span class="count">{filteredMoves.length} moves</span>
  </div>

  <div class="table-wrapper">
    <table class="frame-table">
      <thead>
        <tr>
          <th class="sortable" onclick={() => toggleSort("input")}>
            Input {sortColumn === "input" ? (sortDirection === "asc" ? "↑" : "↓") : ""}
          </th>
          <th class="sortable" onclick={() => toggleSort("name")}>
            Name {sortColumn === "name" ? (sortDirection === "asc" ? "↑" : "↓") : ""}
          </th>
          <th class="sortable num" onclick={() => toggleSort("startup")}>
            Startup {sortColumn === "startup" ? (sortDirection === "asc" ? "↑" : "↓") : ""}
          </th>
          <th class="sortable num" onclick={() => toggleSort("active")}>
            Active {sortColumn === "active" ? (sortDirection === "asc" ? "↑" : "↓") : ""}
          </th>
          <th class="sortable num" onclick={() => toggleSort("recovery")}>
            Recovery {sortColumn === "recovery" ? (sortDirection === "asc" ? "↑" : "↓") : ""}
          </th>
          <th class="sortable num" onclick={() => toggleSort("total")}>
            Total {sortColumn === "total" ? (sortDirection === "asc" ? "↑" : "↓") : ""}
          </th>
          <th class="sortable num" onclick={() => toggleSort("damage")}>
            Damage {sortColumn === "damage" ? (sortDirection === "asc" ? "↑" : "↓") : ""}
          </th>
          <th class="sortable num" onclick={() => toggleSort("advantage_hit")}>
            On Hit {sortColumn === "advantage_hit" ? (sortDirection === "asc" ? "↑" : "↓") : ""}
          </th>
          <th class="sortable num" onclick={() => toggleSort("advantage_block")}>
            On Block {sortColumn === "advantage_block" ? (sortDirection === "asc" ? "↑" : "↓") : ""}
          </th>
          <th>Guard</th>
        </tr>
      </thead>
      <tbody>
        {#each sortedMoves as move}
          <tr onclick={() => handleRowClick(move)}>
            <td class="input-cell">{move.input}</td>
            <td>{move.name}</td>
            <td class="num">{move.startup}</td>
            <td class="num">{move.active}</td>
            <td class="num">{move.recovery}</td>
            <td class="num">{getTotal(move)}</td>
            <td class="num">{move.damage}</td>
            <td class="num advantage" class:positive={getAdvantageHit(move) >= 0}>
              {formatAdvantage(getAdvantageHit(move))}
            </td>
            <td class="num advantage" class:positive={getAdvantageBlock(move) >= 0}>
              {formatAdvantage(getAdvantageBlock(move))}
            </td>
            <td class="guard">{move.guard}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
</div>

<style>
  .frame-data-container {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .toolbar {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 12px;
  }

  .count {
    color: var(--text-secondary);
    font-size: 13px;
  }

  .table-wrapper {
    flex: 1;
    overflow: auto;
    border: 1px solid var(--border);
    border-radius: 4px;
  }

  .frame-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 13px;
  }

  th, td {
    padding: 8px 12px;
    text-align: left;
    border-bottom: 1px solid var(--border);
  }

  th {
    background: var(--bg-secondary);
    font-weight: 600;
    position: sticky;
    top: 0;
  }

  th.sortable {
    cursor: pointer;
    user-select: none;
  }

  th.sortable:hover {
    background: var(--bg-tertiary);
  }

  th.num, td.num {
    text-align: right;
    font-variant-numeric: tabular-nums;
  }

  tbody tr {
    cursor: pointer;
  }

  tbody tr:hover {
    background: var(--bg-tertiary);
  }

  .input-cell {
    font-family: monospace;
    font-weight: 600;
    color: var(--accent);
  }

  .advantage {
    color: var(--accent);
  }

  .advantage.positive {
    color: var(--success);
  }

  .guard {
    text-transform: capitalize;
  }
</style>
```

**Step 2: Update +page.svelte to use FrameDataTable**

```svelte
<script lang="ts">
  import "$lib/../app.css";
  import Header from "$lib/components/Header.svelte";
  import Sidebar from "$lib/components/Sidebar.svelte";
  import FrameDataTable from "$lib/views/FrameDataTable.svelte";
  import { getCurrentCharacter } from "$lib/stores/character.svelte";

  let currentView = $state("overview");

  function handleViewChange(view: string) {
    currentView = view;
  }

  function handleEditMove(input: string) {
    currentView = "move-editor";
  }

  const currentCharacter = $derived(getCurrentCharacter());
</script>

<div class="app-container">
  <Header {currentView} onViewChange={handleViewChange} />
  <Sidebar />
  <main class="main-content">
    {#if !currentCharacter}
      <div class="placeholder">
        <p>Select a character from the sidebar to begin editing.</p>
      </div>
    {:else if currentView === "overview"}
      <h2>{currentCharacter.character.name}</h2>
      <p>Overview view coming soon...</p>
    {:else if currentView === "frame-data"}
      <FrameDataTable onEditMove={handleEditMove} />
    {:else if currentView === "move-editor"}
      <p>Move Editor view coming soon...</p>
    {:else if currentView === "cancel-graph"}
      <p>Cancel Graph view coming soon...</p>
    {/if}
  </main>
</div>

<style>
  .placeholder {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-secondary);
  }
</style>
```

**Step 3: Run `npm run tauri dev` to test**

Run: `npm run tauri dev`
Expected: Select GLITCH, click "Frame Data" tab, see table with 3 moves

**Step 4: Commit**

```bash
git add src/lib/views/ src/routes/
git commit -m "$(cat <<'EOF'
feat: add Frame Data Table view

Implements sortable, filterable spreadsheet view of move data
with calculated advantage values and click-to-edit navigation.

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

## Phase 3: Character Overview View

### Task 6: Implement Character Overview View

**Files:**
- Create: `src/lib/views/CharacterOverview.svelte`
- Modify: `src/routes/+page.svelte`

**Step 1: Create CharacterOverview.svelte**

```svelte
<script lang="ts">
  import { getCurrentCharacter } from "$lib/stores/character.svelte";

  const characterData = $derived(getCurrentCharacter());
  const character = $derived(characterData?.character);
  const moves = $derived(characterData?.moves ?? []);
  const cancelTable = $derived(characterData?.cancel_table);

  // Derived stats
  const normalCount = $derived(moves.filter((m) => !/\d{3,}/.test(m.input)).length);
  const specialCount = $derived(moves.filter((m) => /\d{3,}/.test(m.input)).length);
  const avgStartup = $derived(
    moves.length > 0
      ? Math.round(moves.reduce((sum, m) => sum + m.startup, 0) / moves.length)
      : 0
  );
  const avgDamage = $derived(
    moves.length > 0
      ? Math.round(moves.reduce((sum, m) => sum + m.damage, 0) / moves.length)
      : 0
  );
</script>

{#if character}
  <div class="overview-container">
    <div class="header-section">
      <h1 class="character-name">{character.name}</h1>
      <span class="archetype-badge">{character.archetype}</span>
    </div>

    <div class="stats-grid">
      <div class="stat-card">
        <h3>Properties</h3>
        <dl class="properties">
          <dt>Health</dt>
          <dd>{character.health}</dd>
          <dt>Walk Speed</dt>
          <dd>{character.walk_speed}</dd>
          <dt>Back Walk Speed</dt>
          <dd>{character.back_walk_speed}</dd>
          <dt>Jump Height</dt>
          <dd>{character.jump_height}</dd>
          <dt>Jump Duration</dt>
          <dd>{character.jump_duration}f</dd>
          <dt>Dash Distance</dt>
          <dd>{character.dash_distance}</dd>
          <dt>Dash Duration</dt>
          <dd>{character.dash_duration}f</dd>
        </dl>
      </div>

      <div class="stat-card">
        <h3>Move Summary</h3>
        <dl class="properties">
          <dt>Total Moves</dt>
          <dd>{moves.length}</dd>
          <dt>Normals</dt>
          <dd>{normalCount}</dd>
          <dt>Specials</dt>
          <dd>{specialCount}</dd>
          <dt>Avg Startup</dt>
          <dd>{avgStartup}f</dd>
          <dt>Avg Damage</dt>
          <dd>{avgDamage}</dd>
        </dl>
      </div>

      {#if cancelTable}
        <div class="stat-card">
          <h3>Cancel Routes</h3>
          <dl class="properties">
            <dt>Chain Starters</dt>
            <dd>{Object.keys(cancelTable.chains).length}</dd>
            <dt>Special Cancels</dt>
            <dd>{cancelTable.special_cancels.length}</dd>
            <dt>Super Cancels</dt>
            <dd>{cancelTable.super_cancels.length}</dd>
            <dt>Jump Cancels</dt>
            <dd>{cancelTable.jump_cancels.length}</dd>
          </dl>
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .overview-container {
    max-width: 900px;
  }

  .header-section {
    display: flex;
    align-items: center;
    gap: 16px;
    margin-bottom: 24px;
  }

  .character-name {
    font-size: 32px;
    font-weight: 700;
    margin: 0;
  }

  .archetype-badge {
    background: var(--accent);
    color: white;
    padding: 4px 12px;
    border-radius: 12px;
    font-size: 12px;
    font-weight: 600;
    text-transform: uppercase;
  }

  .stats-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
    gap: 16px;
  }

  .stat-card {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 16px;
  }

  .stat-card h3 {
    font-size: 14px;
    font-weight: 600;
    margin-bottom: 12px;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .properties {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 8px;
  }

  dt {
    color: var(--text-secondary);
  }

  dd {
    font-weight: 600;
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
</style>
```

**Step 2: Update +page.svelte**

```svelte
<script lang="ts">
  import "$lib/../app.css";
  import Header from "$lib/components/Header.svelte";
  import Sidebar from "$lib/components/Sidebar.svelte";
  import CharacterOverview from "$lib/views/CharacterOverview.svelte";
  import FrameDataTable from "$lib/views/FrameDataTable.svelte";
  import { getCurrentCharacter } from "$lib/stores/character.svelte";

  let currentView = $state("overview");

  function handleViewChange(view: string) {
    currentView = view;
  }

  function handleEditMove(input: string) {
    currentView = "move-editor";
  }

  const currentCharacter = $derived(getCurrentCharacter());
</script>

<div class="app-container">
  <Header {currentView} onViewChange={handleViewChange} />
  <Sidebar />
  <main class="main-content">
    {#if !currentCharacter}
      <div class="placeholder">
        <p>Select a character from the sidebar to begin editing.</p>
      </div>
    {:else if currentView === "overview"}
      <CharacterOverview />
    {:else if currentView === "frame-data"}
      <FrameDataTable onEditMove={handleEditMove} />
    {:else if currentView === "move-editor"}
      <p>Move Editor view coming soon...</p>
    {:else if currentView === "cancel-graph"}
      <p>Cancel Graph view coming soon...</p>
    {/if}
  </main>
</div>

<style>
  .placeholder {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-secondary);
  }
</style>
```

**Step 3: Run `npm run tauri dev` to test**

Run: `npm run tauri dev`
Expected: Select GLITCH, Overview tab shows stats cards

**Step 4: Commit**

```bash
git add src/lib/views/ src/routes/
git commit -m "$(cat <<'EOF'
feat: add Character Overview view

Displays character properties, move summary stats, and
cancel route counts in a card-based layout.

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

## Phase 4: Move Editor View

### Task 7: Implement Move Editor View (Basic Form)

**Files:**
- Create: `src/lib/views/MoveEditor.svelte`
- Modify: `src/routes/+page.svelte`

**Step 1: Create MoveEditor.svelte**

```svelte
<script lang="ts">
  import { getSelectedMove, getSelectedMoveInput, selectMove, getCurrentCharacter } from "$lib/stores/character.svelte";
  import type { Move } from "$lib/types";

  const characterData = $derived(getCurrentCharacter());
  const moves = $derived(characterData?.moves ?? []);
  const selectedInput = $derived(getSelectedMoveInput());
  const selectedMove = $derived(getSelectedMove());

  // Local editing state (copy of move data)
  let editingMove = $state<Move | null>(null);

  $effect(() => {
    if (selectedMove) {
      editingMove = structuredClone(selectedMove);
    } else {
      editingMove = null;
    }
  });

  function getTotal(): number {
    if (!editingMove) return 0;
    return editingMove.startup + editingMove.active + editingMove.recovery;
  }

  function getAdvantageHit(): number {
    if (!editingMove) return 0;
    return editingMove.hitstun - editingMove.recovery;
  }

  function getAdvantageBlock(): number {
    if (!editingMove) return 0;
    return editingMove.blockstun - editingMove.recovery;
  }

  function formatAdvantage(value: number): string {
    return value >= 0 ? `+${value}` : String(value);
  }
</script>

<div class="move-editor-container">
  <div class="move-selector">
    <label for="move-select">Move:</label>
    <select id="move-select" value={selectedInput ?? ""} onchange={(e) => selectMove(e.currentTarget.value)}>
      <option value="" disabled>Select a move</option>
      {#each moves as move}
        <option value={move.input}>{move.input} - {move.name}</option>
      {/each}
    </select>
  </div>

  {#if editingMove}
    <div class="editor-layout">
      <div class="form-panel">
        <h3>Frame Data</h3>

        <div class="form-section">
          <div class="form-row">
            <label for="input">Input</label>
            <input id="input" type="text" bind:value={editingMove.input} />
          </div>
          <div class="form-row">
            <label for="name">Name</label>
            <input id="name" type="text" bind:value={editingMove.name} />
          </div>
        </div>

        <div class="form-section">
          <h4>Timing</h4>
          <div class="form-grid">
            <div class="form-row">
              <label for="startup">Startup</label>
              <input id="startup" type="number" min="0" bind:value={editingMove.startup} />
            </div>
            <div class="form-row">
              <label for="active">Active</label>
              <input id="active" type="number" min="0" bind:value={editingMove.active} />
            </div>
            <div class="form-row">
              <label for="recovery">Recovery</label>
              <input id="recovery" type="number" min="0" bind:value={editingMove.recovery} />
            </div>
            <div class="form-row readonly">
              <label>Total</label>
              <span class="computed">{getTotal()}f</span>
            </div>
          </div>
        </div>

        <div class="form-section">
          <h4>Damage & Stun</h4>
          <div class="form-grid">
            <div class="form-row">
              <label for="damage">Damage</label>
              <input id="damage" type="number" min="0" bind:value={editingMove.damage} />
            </div>
            <div class="form-row">
              <label for="hitstun">Hitstun</label>
              <input id="hitstun" type="number" min="0" bind:value={editingMove.hitstun} />
            </div>
            <div class="form-row">
              <label for="blockstun">Blockstun</label>
              <input id="blockstun" type="number" min="0" bind:value={editingMove.blockstun} />
            </div>
            <div class="form-row">
              <label for="hitstop">Hitstop</label>
              <input id="hitstop" type="number" min="0" bind:value={editingMove.hitstop} />
            </div>
          </div>
        </div>

        <div class="form-section">
          <h4>Advantage</h4>
          <div class="advantage-display">
            <div class="advantage-item">
              <span class="label">On Hit</span>
              <span class="value" class:positive={getAdvantageHit() >= 0}>
                {formatAdvantage(getAdvantageHit())}
              </span>
            </div>
            <div class="advantage-item">
              <span class="label">On Block</span>
              <span class="value" class:positive={getAdvantageBlock() >= 0}>
                {formatAdvantage(getAdvantageBlock())}
              </span>
            </div>
          </div>
        </div>

        <div class="form-section">
          <h4>Properties</h4>
          <div class="form-grid">
            <div class="form-row">
              <label for="guard">Guard</label>
              <select id="guard" bind:value={editingMove.guard}>
                <option value="high">High</option>
                <option value="mid">Mid</option>
                <option value="low">Low</option>
                <option value="unblockable">Unblockable</option>
              </select>
            </div>
            <div class="form-row">
              <label for="animation">Animation</label>
              <input id="animation" type="text" bind:value={editingMove.animation} />
            </div>
          </div>
        </div>

        <div class="form-section">
          <h4>Pushback</h4>
          <div class="form-grid">
            <div class="form-row">
              <label for="pushback-hit">On Hit</label>
              <input id="pushback-hit" type="number" bind:value={editingMove.pushback.hit} />
            </div>
            <div class="form-row">
              <label for="pushback-block">On Block</label>
              <input id="pushback-block" type="number" bind:value={editingMove.pushback.block} />
            </div>
          </div>
        </div>

        <div class="form-section">
          <h4>Meter Gain</h4>
          <div class="form-grid">
            <div class="form-row">
              <label for="meter-hit">On Hit</label>
              <input id="meter-hit" type="number" min="0" bind:value={editingMove.meter_gain.hit} />
            </div>
            <div class="form-row">
              <label for="meter-whiff">On Whiff</label>
              <input id="meter-whiff" type="number" min="0" bind:value={editingMove.meter_gain.whiff} />
            </div>
          </div>
        </div>
      </div>

      <div class="preview-panel">
        <h3>Preview</h3>
        <div class="preview-placeholder">
          <p>Animation preview coming soon</p>
          <p class="hint">Hitbox overlay editing will appear here</p>
        </div>
      </div>
    </div>
  {:else}
    <div class="no-selection">
      <p>Select a move from the dropdown or click a row in the Frame Data table.</p>
    </div>
  {/if}
</div>

<style>
  .move-editor-container {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .move-selector {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 16px;
  }

  .move-selector select {
    min-width: 250px;
  }

  .editor-layout {
    display: grid;
    grid-template-columns: 350px 1fr;
    gap: 24px;
    flex: 1;
    min-height: 0;
  }

  .form-panel {
    overflow-y: auto;
    padding-right: 8px;
  }

  .form-panel h3 {
    font-size: 16px;
    margin-bottom: 16px;
  }

  .form-section {
    margin-bottom: 20px;
  }

  .form-section h4 {
    font-size: 12px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-secondary);
    margin-bottom: 8px;
  }

  .form-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
  }

  .form-row {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .form-row label {
    font-size: 12px;
    color: var(--text-secondary);
  }

  .form-row input, .form-row select {
    width: 100%;
  }

  .form-row.readonly .computed {
    padding: 6px 10px;
    background: var(--bg-secondary);
    border-radius: 4px;
    font-variant-numeric: tabular-nums;
  }

  .advantage-display {
    display: flex;
    gap: 24px;
  }

  .advantage-item {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .advantage-item .label {
    font-size: 12px;
    color: var(--text-secondary);
  }

  .advantage-item .value {
    font-size: 24px;
    font-weight: 700;
    color: var(--accent);
    font-variant-numeric: tabular-nums;
  }

  .advantage-item .value.positive {
    color: var(--success);
  }

  .preview-panel {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 16px;
    display: flex;
    flex-direction: column;
  }

  .preview-panel h3 {
    font-size: 16px;
    margin-bottom: 16px;
  }

  .preview-placeholder {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    color: var(--text-secondary);
    border: 2px dashed var(--border);
    border-radius: 8px;
  }

  .preview-placeholder .hint {
    font-size: 12px;
    margin-top: 8px;
  }

  .no-selection {
    display: flex;
    align-items: center;
    justify-content: center;
    flex: 1;
    color: var(--text-secondary);
  }
</style>
```

**Step 2: Update +page.svelte**

```svelte
<script lang="ts">
  import "$lib/../app.css";
  import Header from "$lib/components/Header.svelte";
  import Sidebar from "$lib/components/Sidebar.svelte";
  import CharacterOverview from "$lib/views/CharacterOverview.svelte";
  import FrameDataTable from "$lib/views/FrameDataTable.svelte";
  import MoveEditor from "$lib/views/MoveEditor.svelte";
  import { getCurrentCharacter } from "$lib/stores/character.svelte";

  let currentView = $state("overview");

  function handleViewChange(view: string) {
    currentView = view;
  }

  function handleEditMove(input: string) {
    currentView = "move-editor";
  }

  const currentCharacter = $derived(getCurrentCharacter());
</script>

<div class="app-container">
  <Header {currentView} onViewChange={handleViewChange} />
  <Sidebar />
  <main class="main-content">
    {#if !currentCharacter}
      <div class="placeholder">
        <p>Select a character from the sidebar to begin editing.</p>
      </div>
    {:else if currentView === "overview"}
      <CharacterOverview />
    {:else if currentView === "frame-data"}
      <FrameDataTable onEditMove={handleEditMove} />
    {:else if currentView === "move-editor"}
      <MoveEditor />
    {:else if currentView === "cancel-graph"}
      <p>Cancel Graph view coming soon...</p>
    {/if}
  </main>
</div>

<style>
  .placeholder {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-secondary);
  }
</style>
```

**Step 3: Run `npm run tauri dev` to test**

Run: `npm run tauri dev`
Expected: Select GLITCH, go to Frame Data, click a row, switches to Move Editor with form

**Step 4: Commit**

```bash
git add src/lib/views/ src/routes/
git commit -m "$(cat <<'EOF'
feat: add Move Editor view with frame data form

Implements form-based editing for all move properties including
timing, damage, stun, guard type, pushback, and meter gain.

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

## Phase 5: Save Functionality

### Task 8: Implement Save Move Command

**Files:**
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src/lib/stores/character.svelte.ts`
- Modify: `src/lib/views/MoveEditor.svelte`

**Step 1: Add save_move command to commands.rs**

Add to the end of `commands.rs`:

```rust
#[tauri::command]
pub fn save_move(characters_dir: String, character_id: String, mv: Move) -> Result<(), String> {
    let move_path = Path::new(&characters_dir)
        .join(&character_id)
        .join("moves")
        .join(format!("{}.json", mv.input));

    let content = serde_json::to_string_pretty(&mv).map_err(|e| e.to_string())?;
    fs::write(&move_path, content).map_err(|e| e.to_string())?;

    Ok(())
}
```

**Step 2: Register save_move in lib.rs**

```rust
pub mod codegen;
pub mod commands;
pub mod schema;

use commands::{list_characters, load_character, save_move};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![list_characters, load_character, save_move])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

**Step 3: Add saveMove to character store**

Add to the end of `character.svelte.ts`:

```typescript
export async function saveMove(mv: Move): Promise<void> {
  if (!currentCharacter) {
    throw new Error("No character selected");
  }

  loading = true;
  error = null;
  try {
    await invoke("save_move", {
      charactersDir: CHARACTERS_DIR,
      characterId: currentCharacter.character.id,
      mv,
    });

    // Update local state
    const index = currentCharacter.moves.findIndex((m) => m.input === mv.input);
    if (index >= 0) {
      currentCharacter.moves[index] = mv;
    }
  } catch (e) {
    error = String(e);
    throw e;
  } finally {
    loading = false;
  }
}
```

**Step 4: Add save button to MoveEditor.svelte**

Add after the form-panel div closes but before preview-panel:

```svelte
<div class="form-actions">
  <button class="save-btn" onclick={handleSave}>Save Move</button>
  {#if saveStatus}
    <span class="save-status" class:error={saveStatus.includes("Error")}>
      {saveStatus}
    </span>
  {/if}
</div>
```

Add the script:

```typescript
import { saveMove } from "$lib/stores/character.svelte";

let saveStatus = $state<string | null>(null);

async function handleSave() {
  if (!editingMove) return;

  saveStatus = null;
  try {
    await saveMove(editingMove);
    saveStatus = "Saved!";
    setTimeout(() => { saveStatus = null; }, 2000);
  } catch (e) {
    saveStatus = `Error: ${e}`;
  }
}
```

Add the styles:

```css
.form-actions {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-top: 16px;
  padding-top: 16px;
  border-top: 1px solid var(--border);
}

.save-btn {
  background: var(--accent);
  border-color: var(--accent);
}

.save-btn:hover {
  background: var(--accent-hover);
  border-color: var(--accent-hover);
}

.save-status {
  font-size: 13px;
  color: var(--success);
}

.save-status.error {
  color: var(--accent);
}
```

**Step 5: Run `cargo check` and `npm run check`**

Run: `cd src-tauri && cargo check && cd .. && npm run check`
Expected: Both pass

**Step 6: Run `npm run tauri dev` to test saving**

Run: `npm run tauri dev`
Expected: Edit a move value, click Save, verify file updated on disk

**Step 7: Commit**

```bash
git add src-tauri/src/ src/lib/
git commit -m "$(cat <<'EOF'
feat: add move save functionality

Implements save_move Tauri command and UI button to persist
move changes to disk.

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

## Phase 6: Export System

### Task 9: Implement JSON Blob Export

**Files:**
- Modify: `src-tauri/src/codegen/json_blob.rs`
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`

**Step 1: Implement export_json_blob**

```rust
use crate::commands::CharacterData;
use serde_json;

/// Export character data as a single minified JSON blob
pub fn export_json_blob(character_data: &CharacterData) -> Result<String, String> {
    serde_json::to_string(character_data).map_err(|e| e.to_string())
}

/// Export character data as a pretty-printed JSON blob
pub fn export_json_blob_pretty(character_data: &CharacterData) -> Result<String, String> {
    serde_json::to_string_pretty(character_data).map_err(|e| e.to_string())
}
```

**Step 2: Add export command to commands.rs**

```rust
use crate::codegen::json_blob::{export_json_blob, export_json_blob_pretty};

#[tauri::command]
pub fn export_character(
    characters_dir: String,
    character_id: String,
    adapter: String,
    output_path: String,
    pretty: bool,
) -> Result<(), String> {
    let char_data = load_character(characters_dir, character_id)?;

    let output = match adapter.as_str() {
        "json-blob" => {
            if pretty {
                export_json_blob_pretty(&char_data)?
            } else {
                export_json_blob(&char_data)?
            }
        }
        "breakpoint-rust" => {
            return Err("Breakpoint adapter not yet implemented".to_string());
        }
        _ => return Err(format!("Unknown adapter: {}", adapter)),
    };

    fs::write(&output_path, output).map_err(|e| e.to_string())?;
    Ok(())
}
```

**Step 3: Register export_character in lib.rs**

```rust
use commands::{export_character, list_characters, load_character, save_move};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            list_characters,
            load_character,
            save_move,
            export_character
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

**Step 4: Run `cargo check`**

Run: `cd src-tauri && cargo check`
Expected: Compilation succeeds

**Step 5: Commit**

```bash
git add src-tauri/src/
git commit -m "$(cat <<'EOF'
feat: implement JSON blob export adapter

Adds export_character command with json-blob adapter support
for generating single-file character exports.

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 10: Add Export UI to Character Overview

**Files:**
- Modify: `src/lib/stores/character.svelte.ts`
- Modify: `src/lib/views/CharacterOverview.svelte`

**Step 1: Add exportCharacter to store**

Add to `character.svelte.ts`:

```typescript
export async function exportCharacter(
  adapter: string,
  outputPath: string,
  pretty: boolean = false
): Promise<void> {
  if (!currentCharacter) {
    throw new Error("No character selected");
  }

  await invoke("export_character", {
    charactersDir: CHARACTERS_DIR,
    characterId: currentCharacter.character.id,
    adapter,
    outputPath,
    pretty,
  });
}
```

**Step 2: Add export section to CharacterOverview.svelte**

Add after stats-grid div:

```svelte
<div class="export-section">
  <h3>Export</h3>
  <div class="export-controls">
    <select bind:value={exportAdapter}>
      <option value="json-blob">JSON Blob</option>
      <option value="breakpoint-rust" disabled>Breakpoint Rust (coming soon)</option>
    </select>
    <label class="checkbox-label">
      <input type="checkbox" bind:checked={exportPretty} />
      Pretty print
    </label>
    <button onclick={handleExport}>Export Character</button>
    {#if exportStatus}
      <span class="export-status" class:error={exportStatus.includes("Error")}>
        {exportStatus}
      </span>
    {/if}
  </div>
</div>
```

Add the script:

```typescript
import { exportCharacter } from "$lib/stores/character.svelte";

let exportAdapter = $state("json-blob");
let exportPretty = $state(true);
let exportStatus = $state<string | null>(null);

async function handleExport() {
  if (!character) return;

  exportStatus = null;
  const filename = `${character.id}.${exportAdapter === "json-blob" ? "json" : "rs"}`;
  const outputPath = `exports/${filename}`;

  try {
    await exportCharacter(exportAdapter, outputPath, exportPretty);
    exportStatus = `Exported to ${outputPath}`;
    setTimeout(() => { exportStatus = null; }, 3000);
  } catch (e) {
    exportStatus = `Error: ${e}`;
  }
}
```

Add the styles:

```css
.export-section {
  margin-top: 24px;
  padding-top: 24px;
  border-top: 1px solid var(--border);
}

.export-section h3 {
  font-size: 14px;
  font-weight: 600;
  margin-bottom: 12px;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.export-controls {
  display: flex;
  align-items: center;
  gap: 12px;
  flex-wrap: wrap;
}

.checkbox-label {
  display: flex;
  align-items: center;
  gap: 6px;
  cursor: pointer;
}

.export-status {
  font-size: 13px;
  color: var(--success);
}

.export-status.error {
  color: var(--accent);
}
```

**Step 3: Create exports directory**

```bash
mkdir -p exports
echo "exports/" >> .gitignore
```

**Step 4: Run `npm run tauri dev` to test export**

Run: `npm run tauri dev`
Expected: Select GLITCH, click Export, verify `exports/glitch.json` created

**Step 5: Commit**

```bash
git add src/lib/ .gitignore
git commit -m "$(cat <<'EOF'
feat: add export UI to Character Overview

Adds export controls with adapter selection and pretty-print
option. Exports are saved to exports/ directory.

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

## Phase 7: Cancel Graph View (Stretch Goal)

### Task 11: Implement Basic Cancel Graph Visualization

**Files:**
- Create: `src/lib/views/CancelGraph.svelte`
- Modify: `src/routes/+page.svelte`

**Step 1: Create CancelGraph.svelte**

```svelte
<script lang="ts">
  import { getCurrentCharacter } from "$lib/stores/character.svelte";
  import { onMount } from "svelte";

  const characterData = $derived(getCurrentCharacter());
  const moves = $derived(characterData?.moves ?? []);
  const cancelTable = $derived(characterData?.cancel_table);

  interface Node {
    id: string;
    x: number;
    y: number;
  }

  interface Edge {
    from: string;
    to: string;
    type: "chain" | "special" | "super" | "jump";
  }

  let nodes = $state<Node[]>([]);
  let edges = $state<Edge[]>([]);
  let svgWidth = $state(800);
  let svgHeight = $state(600);

  $effect(() => {
    if (!cancelTable || moves.length === 0) {
      nodes = [];
      edges = [];
      return;
    }

    // Create nodes for each move
    const moveInputs = moves.map((m) => m.input);
    const nodeCount = moveInputs.length;
    const radius = Math.min(svgWidth, svgHeight) / 2 - 60;
    const centerX = svgWidth / 2;
    const centerY = svgHeight / 2;

    nodes = moveInputs.map((input, i) => {
      const angle = (2 * Math.PI * i) / nodeCount - Math.PI / 2;
      return {
        id: input,
        x: centerX + radius * Math.cos(angle),
        y: centerY + radius * Math.sin(angle),
      };
    });

    // Create edges from cancel table
    const newEdges: Edge[] = [];

    // Chain cancels
    for (const [from, targets] of Object.entries(cancelTable.chains)) {
      for (const to of targets) {
        if (from !== to && moveInputs.includes(from) && moveInputs.includes(to)) {
          newEdges.push({ from, to, type: "chain" });
        }
      }
    }

    edges = newEdges;
  });

  function getNodeById(id: string): Node | undefined {
    return nodes.find((n) => n.id === id);
  }

  const edgeColors = {
    chain: "#4ade80",
    special: "#fbbf24",
    super: "#e94560",
    jump: "#60a5fa",
  };
</script>

<div class="cancel-graph-container">
  <div class="legend">
    <span class="legend-item">
      <span class="legend-color" style="background: {edgeColors.chain}"></span>
      Chain
    </span>
    <span class="legend-item">
      <span class="legend-color" style="background: {edgeColors.special}"></span>
      Special
    </span>
    <span class="legend-item">
      <span class="legend-color" style="background: {edgeColors.super}"></span>
      Super
    </span>
    <span class="legend-item">
      <span class="legend-color" style="background: {edgeColors.jump}"></span>
      Jump
    </span>
  </div>

  {#if nodes.length === 0}
    <div class="empty-state">
      <p>No cancel relationships defined</p>
    </div>
  {:else}
    <svg width={svgWidth} height={svgHeight}>
      <defs>
        <marker
          id="arrowhead"
          markerWidth="10"
          markerHeight="7"
          refX="9"
          refY="3.5"
          orient="auto"
        >
          <polygon points="0 0, 10 3.5, 0 7" fill="#4ade80" />
        </marker>
      </defs>

      <!-- Edges -->
      {#each edges as edge}
        {@const fromNode = getNodeById(edge.from)}
        {@const toNode = getNodeById(edge.to)}
        {#if fromNode && toNode}
          <line
            x1={fromNode.x}
            y1={fromNode.y}
            x2={toNode.x}
            y2={toNode.y}
            stroke={edgeColors[edge.type]}
            stroke-width="2"
            marker-end="url(#arrowhead)"
            opacity="0.7"
          />
        {/if}
      {/each}

      <!-- Nodes -->
      {#each nodes as node}
        <g class="node" transform="translate({node.x}, {node.y})">
          <circle r="30" fill="var(--bg-tertiary)" stroke="var(--border)" stroke-width="2" />
          <text
            text-anchor="middle"
            dominant-baseline="middle"
            fill="var(--text-primary)"
            font-size="12"
            font-weight="600"
          >
            {node.id}
          </text>
        </g>
      {/each}
    </svg>
  {/if}
</div>

<style>
  .cancel-graph-container {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .legend {
    display: flex;
    gap: 16px;
    margin-bottom: 12px;
  }

  .legend-item {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 13px;
    color: var(--text-secondary);
  }

  .legend-color {
    width: 12px;
    height: 12px;
    border-radius: 2px;
  }

  .empty-state {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-secondary);
  }

  svg {
    flex: 1;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 8px;
  }

  .node {
    cursor: pointer;
  }

  .node:hover circle {
    fill: var(--accent);
  }
</style>
```

**Step 2: Update +page.svelte**

```svelte
<script lang="ts">
  import "$lib/../app.css";
  import Header from "$lib/components/Header.svelte";
  import Sidebar from "$lib/components/Sidebar.svelte";
  import CharacterOverview from "$lib/views/CharacterOverview.svelte";
  import FrameDataTable from "$lib/views/FrameDataTable.svelte";
  import MoveEditor from "$lib/views/MoveEditor.svelte";
  import CancelGraph from "$lib/views/CancelGraph.svelte";
  import { getCurrentCharacter } from "$lib/stores/character.svelte";

  let currentView = $state("overview");

  function handleViewChange(view: string) {
    currentView = view;
  }

  function handleEditMove(input: string) {
    currentView = "move-editor";
  }

  const currentCharacter = $derived(getCurrentCharacter());
</script>

<div class="app-container">
  <Header {currentView} onViewChange={handleViewChange} />
  <Sidebar />
  <main class="main-content">
    {#if !currentCharacter}
      <div class="placeholder">
        <p>Select a character from the sidebar to begin editing.</p>
      </div>
    {:else if currentView === "overview"}
      <CharacterOverview />
    {:else if currentView === "frame-data"}
      <FrameDataTable onEditMove={handleEditMove} />
    {:else if currentView === "move-editor"}
      <MoveEditor />
    {:else if currentView === "cancel-graph"}
      <CancelGraph />
    {/if}
  </main>
</div>

<style>
  .placeholder {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-secondary);
  }
</style>
```

**Step 3: Run `npm run tauri dev` to test**

Run: `npm run tauri dev`
Expected: Cancel Graph tab shows node graph with 3 moves and chain edges

**Step 4: Commit**

```bash
git add src/lib/views/ src/routes/
git commit -m "$(cat <<'EOF'
feat: add Cancel Graph visualization

Implements circular node layout showing moves with directional
edges for chain cancel relationships.

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

## Summary

This plan implements the core Framesmith editor in 11 tasks:

1. **Sample Data** - Create test character for development
2. **Rust Commands** - Character loading from filesystem
3. **TypeScript Types & Store** - Frontend state management
4. **App Shell** - Navigation and sidebar
5. **Frame Data Table** - Spreadsheet view of moves
6. **Character Overview** - Properties and stats display
7. **Move Editor** - Form-based move editing
8. **Save Functionality** - Persist changes to disk
9. **JSON Export** - Export adapter implementation
10. **Export UI** - User-facing export controls
11. **Cancel Graph** - Visual node graph of relationships

**Not Implemented (Future Work):**
- Animation preview with Threlte/Three.js
- Hitbox overlay editing
- Breakpoint Rust export adapter
- Asset baking
- Multi-character comparison mode
