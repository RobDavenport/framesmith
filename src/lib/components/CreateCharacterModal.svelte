<script lang="ts">
  import {
    createCharacter,
    cloneCharacter,
    getCharacterList,
    selectCharacter,
  } from "$lib/stores/character.svelte";
  import { showError, showSuccess } from "$lib/stores/toast.svelte";

  interface Props {
    open: boolean;
    mode: "new" | "clone";
    onClose: () => void;
  }

  let { open, mode, onClose }: Props = $props();

  let characterId = $state("");
  let characterName = $state("");
  let archetype = $state("");
  let sourceCharacterId = $state("");
  let submitting = $state(false);

  const characterList = $derived(getCharacterList());

  // Auto-generate ID from name
  function slugify(text: string): string {
    return text
      .toLowerCase()
      .trim()
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/^-+|-+$/g, "");
  }

  function handleNameChange(e: Event) {
    const target = e.target as HTMLInputElement;
    characterName = target.value;
    // Auto-generate ID if user hasn't manually edited it
    characterId = slugify(target.value);
  }

  function handleIdChange(e: Event) {
    const target = e.target as HTMLInputElement;
    characterId = target.value.toLowerCase().replace(/[^a-z0-9-_]/g, "");
  }

  async function handleSubmit(e: Event) {
    e.preventDefault();

    if (!characterId.trim()) {
      showError("Character ID is required");
      return;
    }

    if (!characterName.trim()) {
      showError("Character name is required");
      return;
    }

    if (mode === "clone" && !sourceCharacterId) {
      showError("Please select a character to clone");
      return;
    }

    submitting = true;

    try {
      if (mode === "new") {
        await createCharacter(characterId, characterName, archetype);
        showSuccess(`Character "${characterName}" created`);
      } else {
        await cloneCharacter(sourceCharacterId, characterId, characterName);
        showSuccess(`Character "${characterName}" cloned`);
      }

      // Select the new character
      await selectCharacter(characterId);

      // Reset form and close
      resetForm();
      onClose();
    } catch (e) {
      showError(String(e));
    } finally {
      submitting = false;
    }
  }

  function resetForm() {
    characterId = "";
    characterName = "";
    archetype = "";
    sourceCharacterId = "";
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
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div class="modal-backdrop" role="button" tabindex="-1" onclick={handleBackdropClick}>
    <div class="modal">
      <div class="modal-header">
        <h2>{mode === "new" ? "New Character" : "Clone Character"}</h2>
        <button class="close-btn" onclick={handleClose}>&times;</button>
      </div>

      <form onsubmit={handleSubmit}>
        {#if mode === "clone"}
          <div class="field">
            <label for="source">Clone from</label>
            <select
              id="source"
              bind:value={sourceCharacterId}
              disabled={submitting}
            >
              <option value="">Select a character...</option>
              {#each characterList as char}
                <option value={char.id}>{char.name}</option>
              {/each}
            </select>
          </div>
        {/if}

        <div class="field">
          <label for="name">Display Name</label>
          <input
            id="name"
            type="text"
            value={characterName}
            oninput={handleNameChange}
            placeholder="e.g. Glitch"
            disabled={submitting}
          />
        </div>

        <div class="field">
          <label for="id">Character ID</label>
          <input
            id="id"
            type="text"
            value={characterId}
            oninput={handleIdChange}
            placeholder="e.g. test_char"
            disabled={submitting}
          />
          <span class="hint">Lowercase, letters, numbers, dashes only</span>
        </div>

        {#if mode === "new"}
          <div class="field">
            <label for="archetype">Archetype</label>
            <input
              id="archetype"
              type="text"
              bind:value={archetype}
              placeholder="e.g. Rushdown, Zoner, Grappler"
              disabled={submitting}
            />
          </div>
        {/if}

        <div class="actions">
          <button type="button" class="cancel-btn" onclick={handleClose}>
            Cancel
          </button>
          <button type="submit" class="submit-btn" disabled={submitting}>
            {submitting ? "Creating..." : "Create"}
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

  .field input,
  .field select {
    width: 100%;
  }

  .field .hint {
    display: block;
    font-size: 11px;
    color: var(--text-secondary);
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
</style>
