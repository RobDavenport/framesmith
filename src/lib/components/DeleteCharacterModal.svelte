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
