<script lang="ts">
  import type { Precondition } from "$lib/types";

  interface Props {
    preconditions?: Precondition[];
    onAdd: (type: string) => void;
    onRemove: (index: number) => void;
  }

  let { preconditions, onAdd, onRemove }: Props = $props();

  let showPreconditions = $state(false);

  function handleAdd(event: Event) {
    const target = event.currentTarget as HTMLSelectElement;
    onAdd(target.value);
    target.value = ""; // Reset select
  }
</script>

<section class="form-section">
  <button type="button" class="section-title collapsible" onclick={() => showPreconditions = !showPreconditions}>
    Preconditions {preconditions?.length ? `(${preconditions.length})` : ''}
    <span class="collapse-icon">{showPreconditions ? '▼' : '▶'}</span>
  </button>
  {#if showPreconditions}
    <div class="array-editor">
      {#if preconditions}
        {#each preconditions as precondition, i}
          <div class="array-item">
            <span class="item-label">{precondition.type}</span>
            <button class="remove-btn" onclick={() => onRemove(i)}>×</button>
          </div>
        {/each}
      {/if}
      <select class="add-select" onchange={handleAdd}>
        <option value="">+ Add precondition...</option>
        <option value="meter">Meter requirement</option>
        <option value="charge">Charge requirement</option>
        <option value="state">State requirement</option>
        <option value="grounded">Grounded only</option>
        <option value="airborne">Airborne only</option>
        <option value="health">Health requirement</option>
      </select>
    </div>
  {/if}
</section>

<style>
  .form-section {
    margin-bottom: 20px;
    padding-bottom: 16px;
    border-bottom: 1px solid var(--border);
  }

  .section-title {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    margin-bottom: 12px;
  }

  .section-title.collapsible {
    cursor: pointer;
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .section-title.collapsible:hover {
    color: var(--text-primary);
  }

  .collapse-icon {
    font-size: 10px;
  }

  .array-editor {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .array-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px 12px;
    background: var(--bg-secondary);
    border-radius: 4px;
    font-size: 13px;
  }

  .item-label {
    font-family: monospace;
  }

  .remove-btn {
    background: transparent;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
    font-size: 16px;
    padding: 0 4px;
  }

  .remove-btn:hover {
    color: var(--accent);
  }

  .add-select {
    padding: 8px;
    font-size: 13px;
    color: var(--text-secondary);
  }
</style>
