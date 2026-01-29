<script lang="ts">
  import { getToasts, dismissToast } from "$lib/stores/toast.svelte";

  const toasts = $derived(getToasts());
</script>

{#if toasts.length > 0}
  <div class="toast-container">
    {#each toasts as toast (toast.id)}
      <div class="toast toast-{toast.type}">
        <span class="toast-message">{toast.message}</span>
        <button class="toast-dismiss" onclick={() => dismissToast(toast.id)}>
          &times;
        </button>
      </div>
    {/each}
  </div>
{/if}

<style>
  .toast-container {
    position: fixed;
    bottom: 16px;
    right: 16px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    z-index: 1000;
    max-width: 400px;
  }

  .toast {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 16px;
    border-radius: 6px;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
    animation: slideIn 0.2s ease;
  }

  @keyframes slideIn {
    from {
      transform: translateX(100%);
      opacity: 0;
    }
    to {
      transform: translateX(0);
      opacity: 1;
    }
  }

  .toast-error {
    border-left: 4px solid var(--accent);
  }

  .toast-success {
    border-left: 4px solid var(--success);
  }

  .toast-info {
    border-left: 4px solid var(--accent);
  }

  .toast-message {
    flex: 1;
    font-size: 13px;
  }

  .toast-dismiss {
    background: transparent;
    border: none;
    color: var(--text-secondary);
    font-size: 18px;
    padding: 0;
    width: 24px;
    height: 24px;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
  }

  .toast-dismiss:hover {
    color: var(--text-primary);
    background: transparent;
  }
</style>
