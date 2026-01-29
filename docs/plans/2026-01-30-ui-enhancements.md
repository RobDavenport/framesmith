# Framesmith UI Enhancements Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add ZX FSPK export option, delete character functionality, and create move functionality to Framesmith.

**Architecture:** Three independent features that follow existing Framesmith patterns - Tauri commands for backend, Svelte stores for state management, and modal components for user interaction. Each feature builds on established validation and error handling patterns.

**Tech Stack:** Rust (Tauri backend), Svelte 5 runes, TypeScript

---

## Task 1: Add ZX FSPK Export Option to UI

**Files:**
- Modify: `src/lib/views/CharacterOverview.svelte:5-68`

**Step 1: Add zx-fspack option to export dropdown**

In `CharacterOverview.svelte`, update the export dropdown (around line 169-172):

```svelte
<select bind:value={exportAdapter}>
  <option value="json-blob">JSON Blob</option>
  <option value="zx-fspack">ZX FSPK (Binary)</option>
  <option value="breakpoint-rust" disabled>Breakpoint Rust (coming soon)</option>
</select>
```

**Step 2: Update handleExport to use correct extension**

Replace the `handleExport` function (lines 54-68) with:

```typescript
async function handleExport() {
  if (!character) return;

  exportStatus = null;
  let extension: string;
  if (exportAdapter === "zx-fspack") {
    extension = "fspk";
  } else if (exportAdapter === "json-blob") {
    extension = "json";
  } else {
    extension = "rs";
  }
  const filename = `${character.id}.${extension}`;
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

**Step 3: Disable pretty print for binary format**

Update the pretty print checkbox (line 173-176):

```svelte
<label class="checkbox-label">
  <input
    type="checkbox"
    bind:checked={exportPretty}
    disabled={exportAdapter === "zx-fspack"}
  />
  Pretty print
</label>
```

**Step 4: Verify feature works**

Run: `npm run tauri dev`
- Open a project with a character
- Select "ZX FSPK (Binary)" from export dropdown
- Confirm "Pretty print" checkbox is disabled
- Click Export - should create `.fspk` file in exports/

**Step 5: Commit**

```bash
git add src/lib/views/CharacterOverview.svelte
git commit -m "feat: add ZX FSPK export option to character overview"
```

---

## Task 2: Add delete_character Backend Command

**Files:**
- Modify: `src-tauri/src/commands.rs:516+`
- Modify: `src-tauri/src/lib.rs:7-28`

**Step 1: Add delete_character command to commands.rs**

Add this function at the end of `commands.rs` (after line 515):

```rust
#[tauri::command]
pub fn delete_character(characters_dir: String, character_id: String) -> Result<(), String> {
    validate_character_id(&character_id)?;

    let char_path = Path::new(&characters_dir).join(&character_id);

    // Check character exists
    if !char_path.exists() {
        return Err(format!("Character '{}' not found", character_id));
    }

    // Delete the character directory recursively
    fs::remove_dir_all(&char_path)
        .map_err(|e| format!("Failed to delete character: {}", e))?;

    Ok(())
}
```

**Step 2: Register command in lib.rs**

Update the imports in `lib.rs` (line 7-9):

```rust
use commands::{
    clone_character, create_character, create_project, delete_character, export_character,
    list_characters, load_character, open_folder_dialog, save_move, validate_project,
};
```

Update the invoke_handler (lines 17-28):

```rust
.invoke_handler(tauri::generate_handler![
    list_characters,
    load_character,
    save_move,
    export_character,
    open_folder_dialog,
    validate_project,
    create_project,
    create_character,
    clone_character,
    delete_character,
])
```

**Step 3: Verify backend compiles**

Run: `cd src-tauri && cargo check`
Expected: Compiles without errors

**Step 4: Commit**

```bash
git add src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat: add delete_character backend command"
```

---

## Task 3: Add deleteCharacter to Character Store

**Files:**
- Modify: `src/lib/stores/character.svelte.ts:196+`

**Step 1: Add deleteCharacter function**

Add this function at the end of `character.svelte.ts` (after line 195):

```typescript
export async function deleteCharacter(characterId: string): Promise<void> {
  const charactersDir = getCharactersDir();
  if (!charactersDir) {
    throw new Error("No project open");
  }

  await invoke("delete_character", {
    charactersDir,
    characterId,
  });

  // Clear selection if deleted character was selected
  if (currentCharacter?.character.id === characterId) {
    currentCharacter = null;
    selectedMoveInput = null;
  }

  // Reload character list
  await loadCharacterList();
}
```

**Step 2: Commit**

```bash
git add src/lib/stores/character.svelte.ts
git commit -m "feat: add deleteCharacter function to character store"
```

---

## Task 4: Create DeleteCharacterModal Component

**Files:**
- Create: `src/lib/components/DeleteCharacterModal.svelte`

**Step 1: Create the modal component**

Create `src/lib/components/DeleteCharacterModal.svelte`:

```svelte
<script lang="ts">
  import { deleteCharacter } from "$lib/stores/character.svelte";
  import { showError, showSuccess } from "$lib/stores/toast.svelte";

  interface Props {
    open: boolean;
    characterId: string;
    characterName: string;
    onClose: () => void;
  }

  let { open, characterId, characterName, onClose }: Props = $props();

  let confirmInput = $state("");
  let submitting = $state(false);

  const canDelete = $derived(confirmInput === characterId);

  async function handleDelete() {
    if (!canDelete) return;

    submitting = true;
    try {
      await deleteCharacter(characterId);
      showSuccess(`Character "${characterName}" deleted`);
      resetForm();
      onClose();
    } catch (e) {
      showError(String(e));
    } finally {
      submitting = false;
    }
  }

  function resetForm() {
    confirmInput = "";
  }

  function handleClose() {
    resetForm();
    onClose();
  }

  function handleBackdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget) {
      handleClose();
    }
  }
</script>

{#if open}
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div class="modal-backdrop" onclick={handleBackdropClick}>
    <div class="modal">
      <div class="modal-header">
        <h2>Delete Character</h2>
        <button class="close-btn" onclick={handleClose}>&times;</button>
      </div>

      <div class="modal-body">
        <p class="warning">
          This will permanently delete <strong>{characterName}</strong> and all its move data.
          This action cannot be undone.
        </p>

        <div class="field">
          <label for="confirm">Type <code>{characterId}</code> to confirm</label>
          <input
            id="confirm"
            type="text"
            bind:value={confirmInput}
            placeholder={characterId}
            disabled={submitting}
          />
        </div>
      </div>

      <div class="actions">
        <button type="button" class="cancel-btn" onclick={handleClose}>
          Cancel
        </button>
        <button
          type="button"
          class="delete-btn"
          onclick={handleDelete}
          disabled={!canDelete || submitting}
        >
          {submitting ? "Deleting..." : "Delete Character"}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
  }

  .modal {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 8px;
    width: 400px;
    max-width: 90vw;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px;
    border-bottom: 1px solid var(--border);
  }

  .modal-header h2 {
    font-size: 16px;
    font-weight: 600;
    color: var(--error, #ef4444);
  }

  .close-btn {
    background: transparent;
    border: none;
    color: var(--text-secondary);
    font-size: 20px;
    padding: 0;
    width: 28px;
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
  }

  .close-btn:hover {
    color: var(--text-primary);
    background: transparent;
  }

  .modal-body {
    padding: 16px;
  }

  .warning {
    font-size: 14px;
    color: var(--text-secondary);
    margin: 0 0 16px 0;
    line-height: 1.5;
  }

  .warning strong {
    color: var(--text-primary);
  }

  .field {
    margin-bottom: 0;
  }

  .field label {
    display: block;
    font-size: 12px;
    font-weight: 500;
    color: var(--text-secondary);
    margin-bottom: 6px;
  }

  .field label code {
    background: var(--bg-tertiary);
    padding: 2px 6px;
    border-radius: 4px;
    font-family: monospace;
    color: var(--text-primary);
  }

  .field input {
    width: 100%;
  }

  .actions {
    display: flex;
    gap: 8px;
    justify-content: flex-end;
    padding: 16px;
    border-top: 1px solid var(--border);
  }

  .cancel-btn {
    background: transparent;
    border-color: var(--border);
  }

  .cancel-btn:hover {
    background: var(--bg-tertiary);
    border-color: var(--border);
  }

  .delete-btn {
    background: var(--error, #ef4444);
    border-color: var(--error, #ef4444);
    color: white;
  }

  .delete-btn:hover:not(:disabled) {
    background: #dc2626;
    border-color: #dc2626;
  }

  .delete-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
```

**Step 2: Commit**

```bash
git add src/lib/components/DeleteCharacterModal.svelte
git commit -m "feat: add DeleteCharacterModal component"
```

---

## Task 5: Add Delete Button to CharacterOverview

**Files:**
- Modify: `src/lib/views/CharacterOverview.svelte:1-310`

**Step 1: Add import and state for modal**

Update the script section at the top of `CharacterOverview.svelte`:

```svelte
<script lang="ts">
  import { getCurrentCharacter, exportCharacter } from "$lib/stores/character.svelte";
  import DeleteCharacterModal from "$lib/components/DeleteCharacterModal.svelte";
  import type { Move, CancelTable } from "$lib/types";

  let exportAdapter = $state("json-blob");
  let exportPretty = $state(true);
  let exportStatus = $state<string | null>(null);
  let showDeleteModal = $state(false);
```

**Step 2: Add delete section after export section**

After the export-section div (after line 184), add:

```svelte
    <div class="danger-zone">
      <h3>Danger Zone</h3>
      <div class="danger-content">
        <div class="danger-info">
          <span class="danger-title">Delete this character</span>
          <span class="danger-desc">Once deleted, this character cannot be recovered.</span>
        </div>
        <button class="delete-btn" onclick={() => showDeleteModal = true}>
          Delete Character
        </button>
      </div>
    </div>

    <DeleteCharacterModal
      open={showDeleteModal}
      characterId={character.id}
      characterName={character.name}
      onClose={() => showDeleteModal = false}
    />
```

**Step 3: Add danger zone styles**

Add these styles at the end of the style block (before `</style>`):

```css
  .danger-zone {
    margin-top: 48px;
    padding-top: 24px;
    border-top: 1px solid var(--error, #ef4444);
  }

  .danger-zone h3 {
    font-size: 14px;
    font-weight: 600;
    margin-bottom: 12px;
    color: var(--error, #ef4444);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .danger-content {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 16px;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 8px;
  }

  .danger-info {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .danger-title {
    font-weight: 600;
    font-size: 14px;
  }

  .danger-desc {
    font-size: 13px;
    color: var(--text-secondary);
  }

  .delete-btn {
    background: transparent;
    border: 1px solid var(--error, #ef4444);
    color: var(--error, #ef4444);
    flex-shrink: 0;
  }

  .delete-btn:hover {
    background: var(--error, #ef4444);
    color: white;
  }
```

**Step 4: Verify feature works**

Run: `npm run tauri dev`
- Open a project
- Select a character
- Scroll down to see "Danger Zone" section
- Click "Delete Character"
- Modal appears with warning
- Type wrong ID - delete button stays disabled
- Type correct ID - delete button enables
- Click cancel - modal closes
- Click delete - character is deleted

**Step 5: Commit**

```bash
git add src/lib/views/CharacterOverview.svelte
git commit -m "feat: add delete character button to CharacterOverview"
```

---

## Task 6: Add create_move Backend Command

**Files:**
- Modify: `src-tauri/src/commands.rs:530+`
- Modify: `src-tauri/src/lib.rs:7-28`

**Step 1: Add validate_move_input function to commands.rs**

Add this function after `validate_character_id` (around line 393):

```rust
fn validate_move_input(input: &str) -> Result<(), String> {
    if input.is_empty() {
        return Err("Move input cannot be empty".to_string());
    }

    // Prevent path traversal
    if input.contains("..") || input.contains('/') || input.contains('\\') {
        return Err("Invalid move input".to_string());
    }

    // Only allow alphanumeric, plus common fighting game notation characters
    if !input.chars().all(|c| {
        c.is_ascii_alphanumeric()
            || c == '+'
            || c == '['
            || c == ']'
            || c == '.'
            || c == '_'
            || c == '-'
    }) {
        return Err(
            "Move input can only contain letters, numbers, +, [], ., -, and _".to_string(),
        );
    }

    Ok(())
}
```

**Step 2: Add create_move command**

Add this function at the end of `commands.rs` (after delete_character):

```rust
#[tauri::command]
pub fn create_move(
    characters_dir: String,
    character_id: String,
    input: String,
    name: String,
) -> Result<Move, String> {
    validate_character_id(&character_id)?;
    validate_move_input(&input)?;

    if name.trim().is_empty() {
        return Err("Move name cannot be empty".to_string());
    }

    let moves_dir = Path::new(&characters_dir)
        .join(&character_id)
        .join("moves");

    // Check moves directory exists
    if !moves_dir.exists() {
        return Err(format!(
            "Character '{}' moves directory not found",
            character_id
        ));
    }

    let move_path = moves_dir.join(format!("{}.json", input));

    // Check move doesn't already exist
    if move_path.exists() {
        return Err(format!("Move '{}' already exists", input));
    }

    // Create move with default values
    let mv = Move {
        input: input.clone(),
        name,
        tags: vec![],
        startup: 5,
        active: 2,
        recovery: 10,
        damage: 500,
        hitstun: 15,
        blockstun: 10,
        hitstop: 10,
        guard: crate::schema::GuardType::Mid,
        hitboxes: vec![],
        hurtboxes: vec![],
        pushback: crate::schema::Pushback { hit: 5, block: 8 },
        meter_gain: crate::schema::MeterGain { hit: 100, whiff: 20 },
        animation: input.clone(),
        move_type: None,
        trigger: None,
        parent: None,
        total: None,
        hits: None,
        preconditions: None,
        costs: None,
        movement: None,
        super_freeze: None,
        on_use: None,
        on_hit: None,
        on_block: None,
        advanced_hurtboxes: None,
    };

    // Write the move file
    let content = serde_json::to_string_pretty(&mv)
        .map_err(|e| format!("Failed to serialize move: {}", e))?;
    fs::write(&move_path, content).map_err(|e| format!("Failed to write move file: {}", e))?;

    Ok(mv)
}
```

**Step 3: Register command in lib.rs**

Update the imports in `lib.rs`:

```rust
use commands::{
    clone_character, create_character, create_move, create_project, delete_character,
    export_character, list_characters, load_character, open_folder_dialog, save_move,
    validate_project,
};
```

Update the invoke_handler:

```rust
.invoke_handler(tauri::generate_handler![
    list_characters,
    load_character,
    save_move,
    export_character,
    open_folder_dialog,
    validate_project,
    create_project,
    create_character,
    clone_character,
    delete_character,
    create_move,
])
```

**Step 4: Verify backend compiles**

Run: `cd src-tauri && cargo check`
Expected: Compiles without errors

**Step 5: Commit**

```bash
git add src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat: add create_move backend command"
```

---

## Task 7: Add createMove to Character Store

**Files:**
- Modify: `src/lib/stores/character.svelte.ts:215+`

**Step 1: Add createMove function**

Add this function at the end of `character.svelte.ts`:

```typescript
export async function createMove(input: string, name: string): Promise<Move> {
  const charactersDir = getCharactersDir();
  if (!charactersDir) {
    throw new Error("No project open");
  }
  if (!currentCharacter) {
    throw new Error("No character selected");
  }

  const mv = await invoke<Move>("create_move", {
    charactersDir,
    characterId: currentCharacter.character.id,
    input,
    name,
  });

  // Add move to local state
  currentCharacter.moves = [...currentCharacter.moves, mv];

  // Select the new move
  selectedMoveInput = mv.input;

  return mv;
}
```

**Step 2: Commit**

```bash
git add src/lib/stores/character.svelte.ts
git commit -m "feat: add createMove function to character store"
```

---

## Task 8: Create CreateMoveModal Component

**Files:**
- Create: `src/lib/components/CreateMoveModal.svelte`

**Step 1: Create the modal component**

Create `src/lib/components/CreateMoveModal.svelte`:

```svelte
<script lang="ts">
  import { createMove, getCurrentCharacter } from "$lib/stores/character.svelte";
  import { showError, showSuccess } from "$lib/stores/toast.svelte";

  interface Props {
    open: boolean;
    onClose: () => void;
    onCreated: (input: string) => void;
  }

  let { open, onClose, onCreated }: Props = $props();

  let moveInput = $state("");
  let moveName = $state("");
  let submitting = $state(false);

  const characterData = $derived(getCurrentCharacter());
  const existingMoves = $derived(characterData?.moves ?? []);

  const isDuplicate = $derived(
    existingMoves.some((m) => m.input.toLowerCase() === moveInput.toLowerCase())
  );

  const canCreate = $derived(
    moveInput.trim().length > 0 && moveName.trim().length > 0 && !isDuplicate
  );

  async function handleSubmit(e: Event) {
    e.preventDefault();
    if (!canCreate) return;

    submitting = true;
    try {
      const mv = await createMove(moveInput.trim(), moveName.trim());
      showSuccess(`Move "${moveName}" created`);
      resetForm();
      onClose();
      onCreated(mv.input);
    } catch (e) {
      showError(String(e));
    } finally {
      submitting = false;
    }
  }

  function resetForm() {
    moveInput = "";
    moveName = "";
  }

  function handleClose() {
    resetForm();
    onClose();
  }

  function handleBackdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget) {
      handleClose();
    }
  }
</script>

{#if open}
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div class="modal-backdrop" onclick={handleBackdropClick}>
    <div class="modal">
      <div class="modal-header">
        <h2>New Move</h2>
        <button class="close-btn" onclick={handleClose}>&times;</button>
      </div>

      <form onsubmit={handleSubmit}>
        <div class="field">
          <label for="input">Input</label>
          <input
            id="input"
            type="text"
            bind:value={moveInput}
            placeholder="e.g. 5L, 236P, j.H"
            disabled={submitting}
          />
          {#if isDuplicate}
            <span class="error-hint">A move with this input already exists</span>
          {:else}
            <span class="hint">The button/motion input for this move</span>
          {/if}
        </div>

        <div class="field">
          <label for="name">Name</label>
          <input
            id="name"
            type="text"
            bind:value={moveName}
            placeholder="e.g. Light Punch, Fireball"
            disabled={submitting}
          />
          <span class="hint">Display name for the move</span>
        </div>

        <div class="actions">
          <button type="button" class="cancel-btn" onclick={handleClose}>
            Cancel
          </button>
          <button type="submit" class="submit-btn" disabled={!canCreate || submitting}>
            {submitting ? "Creating..." : "Create Move"}
          </button>
        </div>
      </form>
    </div>
  </div>
{/if}

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
  }

  .modal {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 8px;
    width: 400px;
    max-width: 90vw;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px;
    border-bottom: 1px solid var(--border);
  }

  .modal-header h2 {
    font-size: 16px;
    font-weight: 600;
  }

  .close-btn {
    background: transparent;
    border: none;
    color: var(--text-secondary);
    font-size: 20px;
    padding: 0;
    width: 28px;
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
  }

  .close-btn:hover {
    color: var(--text-primary);
    background: transparent;
  }

  form {
    padding: 16px;
  }

  .field {
    margin-bottom: 16px;
  }

  .field label {
    display: block;
    font-size: 12px;
    font-weight: 500;
    color: var(--text-secondary);
    margin-bottom: 6px;
  }

  .field input {
    width: 100%;
  }

  .field .hint {
    display: block;
    font-size: 11px;
    color: var(--text-secondary);
    margin-top: 4px;
  }

  .field .error-hint {
    display: block;
    font-size: 11px;
    color: var(--error, #ef4444);
    margin-top: 4px;
  }

  .actions {
    display: flex;
    gap: 8px;
    justify-content: flex-end;
    margin-top: 24px;
  }

  .cancel-btn {
    background: transparent;
    border-color: var(--border);
  }

  .cancel-btn:hover {
    background: var(--bg-tertiary);
    border-color: var(--border);
  }

  .submit-btn {
    background: var(--accent);
    border-color: var(--accent);
  }

  .submit-btn:hover:not(:disabled) {
    background: var(--accent-hover);
    border-color: var(--accent-hover);
  }

  .submit-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
```

**Step 2: Commit**

```bash
git add src/lib/components/CreateMoveModal.svelte
git commit -m "feat: add CreateMoveModal component"
```

---

## Task 9: Add New Move Button to FrameDataTable

**Files:**
- Modify: `src/lib/views/FrameDataTable.svelte:1-252`

**Step 1: Add import and modal state**

Update the script section at the top of `FrameDataTable.svelte`:

```svelte
<script lang="ts">
  import { getCurrentCharacter, selectMove } from "$lib/stores/character.svelte";
  import CreateMoveModal from "$lib/components/CreateMoveModal.svelte";
  import type { Move } from "$lib/types";

  interface Props {
    onEditMove: (input: string) => void;
  }

  let { onEditMove }: Props = $props();
  let showCreateModal = $state(false);
```

**Step 2: Add handleMoveCreated function**

Add this function after `handleRowClick` (around line 93):

```typescript
function handleMoveCreated(input: string) {
  selectMove(input);
  onEditMove(input);
}
```

**Step 3: Add New Move button to toolbar**

Update the toolbar div (lines 99-107):

```svelte
<div class="toolbar">
  <select bind:value={filterType}>
    {#each filterOptions as option}
      <option value={option.value}>{option.label}</option>
    {/each}
  </select>
  <span class="count">{filteredMoves.length} moves</span>
  <div class="toolbar-spacer"></div>
  <button class="new-move-btn" onclick={() => showCreateModal = true}>
    + New Move
  </button>
</div>
```

**Step 4: Add modal at end of component**

Add this before the closing `</div>` of the container (before `</style>`):

```svelte
<CreateMoveModal
  open={showCreateModal}
  onClose={() => showCreateModal = false}
  onCreated={handleMoveCreated}
/>
```

**Step 5: Add toolbar styles**

Add these styles (inside the style block):

```css
  .toolbar-spacer {
    flex: 1;
  }

  .new-move-btn {
    background: var(--accent);
    border-color: var(--accent);
    padding: 6px 12px;
    font-size: 13px;
  }

  .new-move-btn:hover {
    background: var(--accent-hover);
    border-color: var(--accent-hover);
  }
```

**Step 6: Verify feature works**

Run: `npm run tauri dev`
- Open a project
- Select a character
- Go to Frame Data Table view
- Click "+ New Move" button
- Enter input "5L" and name "Light Punch"
- If move exists - error hint appears, Create disabled
- Enter unique input - Create enables
- Click Create
- Modal closes, move appears in table, move is selected

**Step 7: Commit**

```bash
git add src/lib/views/FrameDataTable.svelte
git commit -m "feat: add New Move button to FrameDataTable"
```

---

## Task 10: Final Integration Test

**Files:**
- None (manual testing)

**Step 1: Run the application**

Run: `npm run tauri dev`

**Step 2: Test ZX FSPK Export**

- Open a project with characters
- Select a character
- In CharacterOverview, select "ZX FSPK (Binary)" from dropdown
- Verify "Pretty print" is disabled
- Click Export
- Verify `exports/<character-id>.fspk` file is created

**Step 3: Test Create Move**

- Select a character
- Navigate to Frame Data Table
- Click "+ New Move"
- Enter input: "test-move"
- Enter name: "Test Move"
- Click Create
- Verify move appears in table
- Verify move file exists in `characters/<id>/moves/test-move.json`

**Step 4: Test Delete Character**

- Create a test character first via New Character
- Navigate to CharacterOverview for the test character
- Scroll down to "Danger Zone"
- Click "Delete Character"
- Type wrong ID - verify delete button disabled
- Type correct ID - verify delete button enables
- Click Delete
- Verify character removed from sidebar
- Verify character folder deleted from filesystem

**Step 5: Commit completion marker**

```bash
git add -A
git commit -m "feat: complete UI enhancements - FSPK export, delete character, create move"
```

---

## Verification Checklist

- [ ] ZX FSPK export option appears in dropdown
- [ ] Pretty print disabled for FSPK format
- [ ] Export creates .fspk file
- [ ] Delete button appears in Danger Zone
- [ ] Delete modal requires exact ID match
- [ ] Delete removes character from list and filesystem
- [ ] New Move button appears in toolbar
- [ ] New Move modal validates duplicate inputs
- [ ] Created move appears in table and filesystem
- [ ] All toast messages appear correctly
