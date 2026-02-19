<script lang="ts">
  import {
    getGlobalStateList,
    getSelectedGlobalId,
    selectGlobalState,
    deleteGlobalState,
    isLoading,
  } from '$lib/stores/globals.svelte';

  let showDeleteConfirm = $state(false);
  let deleteTargetId = $state<string | null>(null);

  const globalStates = $derived(getGlobalStateList());
  const selectedId = $derived(getSelectedGlobalId());
  const loading = $derived(isLoading());

  function handleSelect(id: string) {
    selectGlobalState(id);
  }

  function handleDeleteClick(id: string, event: MouseEvent) {
    event.stopPropagation();
    deleteTargetId = id;
    showDeleteConfirm = true;
  }

  async function confirmDelete() {
    if (deleteTargetId) {
      await deleteGlobalState(deleteTargetId);
    }
    showDeleteConfirm = false;
    deleteTargetId = null;
  }

  function cancelDelete() {
    showDeleteConfirm = false;
    deleteTargetId = null;
  }
</script>

<div class="global-state-list">
  <div class="header">
    <h3>Global States</h3>
    <span class="indicator" title="Project-wide shared states">🌐</span>
  </div>

  {#if loading}
    <div class="loading">Loading...</div>
  {:else if globalStates.length === 0}
    <div class="empty">
      <p>No global states defined.</p>
      <p class="hint">Create globals in <code>globals/states/</code></p>
    </div>
  {:else}
    <ul class="state-list" role="listbox" aria-label="Global states">
      {#each globalStates as state (state.id)}
        <li
          class="state-item"
          class:selected={selectedId === state.id}
          onclick={() => handleSelect(state.id)}
          onkeydown={(e) => {
            if (e.key === 'Enter' || e.key === ' ') {
              e.preventDefault();
              handleSelect(state.id);
            } else if (e.key === 'ArrowDown') {
              e.preventDefault();
              const next = e.currentTarget.nextElementSibling as HTMLElement;
              next?.focus();
            } else if (e.key === 'ArrowUp') {
              e.preventDefault();
              const prev = e.currentTarget.previousElementSibling as HTMLElement;
              prev?.focus();
            }
          }}
          role="option"
          aria-selected={selectedId === state.id}
          tabindex="0"
        >
          <div class="state-info">
            <span class="state-id">{state.id}</span>
            {#if state.type}
              <span class="state-type">{state.type}</span>
            {/if}
          </div>
          {#if state.name && state.name !== state.id}
            <span class="state-name">{state.name}</span>
          {/if}
          <button
            class="delete-btn"
            onclick={(e) => handleDeleteClick(state.id, e)}
            title="Delete global state"
            aria-label={`Delete ${state.id}`}
          >
            ×
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</div>

{#if showDeleteConfirm}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="modal-overlay" onclick={cancelDelete} onkeydown={(e) => e.key === 'Escape' && cancelDelete()} role="presentation">
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <div class="modal" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.stopPropagation()} role="dialog" aria-modal="true" aria-labelledby="delete-modal-title" tabindex="-1">
      <h4 id="delete-modal-title">Delete Global State</h4>
      <p>Are you sure you want to delete <strong>{deleteTargetId}</strong>?</p>
      <p class="warning">This may break characters that reference it.</p>
      <div class="modal-actions">
        <button onclick={cancelDelete}>Cancel</button>
        <button class="danger" onclick={confirmDelete}>Delete</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .global-state-list {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem;
    border-bottom: 1px solid var(--border-color, #333);
  }

  .header h3 {
    margin: 0;
    font-size: 0.9rem;
  }

  .indicator {
    font-size: 0.8rem;
  }

  .loading,
  .empty {
    padding: 1rem;
    text-align: center;
    color: var(--text-muted, #888);
  }

  .empty .hint {
    font-size: 0.8rem;
    margin-top: 0.5rem;
  }

  .empty code {
    background: var(--bg-secondary, #222);
    padding: 0.1rem 0.3rem;
    border-radius: 3px;
  }

  .state-list {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .state-item {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
    padding: 0.5rem;
    cursor: pointer;
    border-left: 3px solid transparent;
    position: relative;
  }

  .state-item:hover {
    background: var(--bg-hover, #2a2a2a);
  }

  .state-item.selected {
    background: var(--bg-selected, #333);
    border-left-color: var(--accent-color, #4a9eff);
  }

  .state-info {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .state-id {
    font-family: monospace;
    font-weight: 500;
  }

  .state-type {
    font-size: 0.75rem;
    padding: 0.1rem 0.3rem;
    background: var(--bg-secondary, #222);
    border-radius: 3px;
    color: var(--text-muted, #888);
  }

  .state-name {
    font-size: 0.8rem;
    color: var(--text-muted, #888);
  }

  .delete-btn {
    position: absolute;
    right: 0.5rem;
    top: 50%;
    transform: translateY(-50%);
    background: none;
    border: none;
    color: var(--text-muted, #888);
    cursor: pointer;
    font-size: 1.2rem;
    padding: 0.2rem 0.4rem;
    opacity: 0;
    transition: opacity 0.15s;
  }

  .state-item:hover .delete-btn {
    opacity: 1;
  }

  .delete-btn:hover {
    color: var(--danger-color, #ff4444);
  }

  .modal-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .modal {
    background: var(--bg-primary, #1a1a1a);
    padding: 1.5rem;
    border-radius: 8px;
    max-width: 400px;
    width: 90%;
  }

  .modal h4 {
    margin: 0 0 1rem;
  }

  .modal .warning {
    color: var(--warning-color, #ff9944);
    font-size: 0.9rem;
  }

  .modal-actions {
    display: flex;
    gap: 0.5rem;
    justify-content: flex-end;
    margin-top: 1rem;
  }

  .modal-actions button {
    padding: 0.5rem 1rem;
    border: none;
    border-radius: 4px;
    cursor: pointer;
  }

  .modal-actions button.danger {
    background: var(--danger-color, #ff4444);
    color: white;
  }
</style>
