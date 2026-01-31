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
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div class="modal-backdrop" role="button" tabindex="-1" onclick={handleBackdropClick}>
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
