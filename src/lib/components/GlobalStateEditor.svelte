<script lang="ts">
  import {
    getCurrentGlobalState,
    getSelectedGlobalId,
    saveGlobalState,
    getError,
  } from '$lib/stores/globals.svelte';
  import type { State } from '$lib/types';

  // Local editing state
  let editingState = $state<State | null>(null);
  let hasChanges = $state(false);
  let saving = $state(false);
  let saveError = $state<string | null>(null);

  const currentState = $derived(getCurrentGlobalState());
  const selectedId = $derived(getSelectedGlobalId());
  const storeError = $derived(getError());

  // Sync local state when selection changes
  $effect(() => {
    if (currentState) {
      editingState = structuredClone(currentState);
      hasChanges = false;
      saveError = null;
    } else {
      editingState = null;
    }
  });

  function handleFieldChange(field: keyof State, value: unknown) {
    if (!editingState) return;
    (editingState as unknown as Record<string, unknown>)[field] = value;
    hasChanges = true;
  }

  async function handleSave() {
    if (!editingState || !selectedId) return;

    saving = true;
    saveError = null;

    const success = await saveGlobalState(selectedId, editingState);

    if (success) {
      hasChanges = false;
    } else {
      saveError = storeError;
    }

    saving = false;
  }

  function handleRevert() {
    if (currentState) {
      editingState = structuredClone(currentState);
      hasChanges = false;
      saveError = null;
    }
  }
</script>

<div class="global-state-editor">
  {#if !editingState}
    <div class="no-selection">
      <p>Select a global state to edit</p>
    </div>
  {:else}
    <div class="editor-header">
      <h3>
        <span class="indicator">🌐</span>
        {selectedId}
      </h3>
      {#if hasChanges}
        <span class="unsaved">Unsaved changes</span>
      {/if}
    </div>

    {#if saveError}
      <div class="error">{saveError}</div>
    {/if}

    <form class="editor-form" onsubmit={(e) => { e.preventDefault(); handleSave(); }}>
      <div class="field">
        <label for="name">Name</label>
        <input
          id="name"
          type="text"
          value={editingState.name ?? ''}
          oninput={(e) => handleFieldChange('name', e.currentTarget.value || null)}
        />
      </div>

      <div class="field">
        <label for="type">Type</label>
        <input
          id="type"
          type="text"
          value={editingState.type ?? ''}
          oninput={(e) => handleFieldChange('type', e.currentTarget.value || null)}
          placeholder="e.g., system, normal, special"
        />
      </div>

      <div class="field-row">
        <div class="field">
          <label for="startup">Startup</label>
          <input
            id="startup"
            type="number"
            value={editingState.startup ?? ''}
            oninput={(e) => handleFieldChange('startup', e.currentTarget.valueAsNumber || null)}
          />
        </div>

        <div class="field">
          <label for="active">Active</label>
          <input
            id="active"
            type="number"
            value={editingState.active ?? ''}
            oninput={(e) => handleFieldChange('active', e.currentTarget.valueAsNumber || null)}
          />
        </div>

        <div class="field">
          <label for="recovery">Recovery</label>
          <input
            id="recovery"
            type="number"
            value={editingState.recovery ?? ''}
            oninput={(e) => handleFieldChange('recovery', e.currentTarget.valueAsNumber || null)}
          />
        </div>

        <div class="field">
          <label for="total">Total</label>
          <input
            id="total"
            type="number"
            value={editingState.total ?? ''}
            oninput={(e) => handleFieldChange('total', e.currentTarget.valueAsNumber || null)}
          />
        </div>
      </div>

      <div class="field">
        <label for="tags">Tags (comma-separated)</label>
        <input
          id="tags"
          type="text"
          value={editingState.tags?.join(', ') ?? ''}
          oninput={(e) => {
            const tags = e.currentTarget.value
              .split(',')
              .map(t => t.trim())
              .filter(t => t.length > 0);
            handleFieldChange('tags', tags.length > 0 ? tags : null);
          }}
          placeholder="e.g., invulnerable, reversal"
        />
      </div>

      <div class="field">
        <label for="animation">Animation</label>
        <input
          id="animation"
          type="text"
          value={editingState.animation ?? ''}
          oninput={(e) => handleFieldChange('animation', e.currentTarget.value || null)}
        />
      </div>

      <div class="actions">
        <button type="button" onclick={handleRevert} disabled={!hasChanges || saving}>
          Revert
        </button>
        <button type="submit" disabled={!hasChanges || saving}>
          {saving ? 'Saving...' : 'Save'}
        </button>
      </div>
    </form>
  {/if}
</div>

<style>
  .global-state-editor {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .no-selection {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-muted, #888);
  }

  .editor-header {
    display: flex;
    align-items: center;
    gap: 1rem;
    padding: 1rem;
    border-bottom: 1px solid var(--border-color, #333);
  }

  .editor-header h3 {
    margin: 0;
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .indicator {
    font-size: 0.9rem;
  }

  .unsaved {
    font-size: 0.8rem;
    color: var(--warning-color, #ff9944);
  }

  .error {
    padding: 0.75rem 1rem;
    background: rgba(255, 68, 68, 0.1);
    color: var(--danger-color, #ff4444);
    border-left: 3px solid var(--danger-color, #ff4444);
  }

  .editor-form {
    padding: 1rem;
    display: flex;
    flex-direction: column;
    gap: 1rem;
    overflow-y: auto;
    flex: 1;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .field-row {
    display: flex;
    gap: 1rem;
  }

  .field-row .field {
    flex: 1;
  }

  label {
    font-size: 0.85rem;
    color: var(--text-muted, #888);
  }

  input {
    padding: 0.5rem;
    border: 1px solid var(--border-color, #333);
    border-radius: 4px;
    background: var(--bg-secondary, #222);
    color: inherit;
  }

  input:focus {
    outline: none;
    border-color: var(--accent-color, #4a9eff);
  }

  input[type="number"] {
    width: 100%;
  }

  .actions {
    display: flex;
    gap: 0.5rem;
    justify-content: flex-end;
    padding-top: 1rem;
    border-top: 1px solid var(--border-color, #333);
  }

  .actions button {
    padding: 0.5rem 1rem;
    border: none;
    border-radius: 4px;
    cursor: pointer;
  }

  .actions button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .actions button[type="submit"] {
    background: var(--accent-color, #4a9eff);
    color: white;
  }
</style>
