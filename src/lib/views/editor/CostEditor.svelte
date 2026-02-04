<script lang="ts">
  import type { Cost } from "$lib/types";

  interface Props {
    costs?: Cost[];
    onAdd: (type: string) => void;
    onRemove: (index: number) => void;
  }

  let { costs, onAdd, onRemove }: Props = $props();

  let showCosts = $state(false);

  function handleAdd(event: Event) {
    const target = event.currentTarget as HTMLSelectElement;
    onAdd(target.value);
    target.value = ""; // Reset select
  }
</script>

<section class="form-section">
  <button type="button" class="section-title collapsible" onclick={() => showCosts = !showCosts}>
    Costs {costs?.length ? `(${costs.length})` : ''}
    <span class="collapse-icon">{showCosts ? '▼' : '▶'}</span>
  </button>
  {#if showCosts}
    <div class="array-editor">
      {#if costs}
        {#each costs as cost, i}
          <div class="array-item">
            <span class="item-label">{cost.type}: {cost.amount}</span>
            <button class="remove-btn" onclick={() => onRemove(i)}>×</button>
          </div>
        {/each}
      {/if}
      <select class="add-select" onchange={handleAdd}>
        <option value="">+ Add cost...</option>
        <option value="meter">Meter cost</option>
        <option value="health">Health cost</option>
        <option value="resource">Resource cost</option>
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
