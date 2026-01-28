<script lang="ts">
  import {
    getCurrentCharacter,
    getSelectedMove,
    getSelectedMoveInput,
    selectMove,
  } from "$lib/stores/character.svelte";
  import type { Move } from "$lib/types";

  const characterData = $derived(getCurrentCharacter());
  const moves = $derived(characterData?.moves ?? []);
  const selectedMoveInput = $derived(getSelectedMoveInput());
  const selectedMove = $derived(getSelectedMove());

  // Local editing state - copy of the move data
  let editingMove = $state<Move | null>(null);

  // Watch for selected move changes and create a local copy
  $effect(() => {
    if (selectedMove) {
      editingMove = structuredClone(selectedMove);
    } else if (moves.length > 0 && !selectedMoveInput) {
      // Auto-select first move if none selected
      selectMove(moves[0].input);
    }
  });

  // Computed values
  const totalFrames = $derived(
    editingMove ? editingMove.startup + editingMove.active + editingMove.recovery : 0
  );

  const advantageOnHit = $derived(
    editingMove ? editingMove.hitstun - editingMove.recovery : 0
  );

  const advantageOnBlock = $derived(
    editingMove ? editingMove.blockstun - editingMove.recovery : 0
  );

  // Check if there are unsaved changes
  const hasChanges = $derived.by(() => {
    if (!editingMove || !selectedMove) return false;
    return JSON.stringify(editingMove) !== JSON.stringify(selectedMove);
  });

  function handleMoveSelect(event: Event) {
    const target = event.target as HTMLSelectElement;
    selectMove(target.value);
  }

  function formatAdvantage(value: number): string {
    return value >= 0 ? `+${value}` : String(value);
  }

  const guardOptions: Move["guard"][] = ["high", "mid", "low", "unblockable"];
</script>

{#if editingMove}
  <div class="move-editor-container">
    <!-- Move Selector -->
    <div class="move-selector">
      <label for="move-select">Move:</label>
      <select id="move-select" value={selectedMoveInput} onchange={handleMoveSelect}>
        {#each moves as move}
          <option value={move.input}>{move.input} - {move.name}</option>
        {/each}
      </select>
      {#if hasChanges}
        <span class="unsaved-indicator">Unsaved changes</span>
      {/if}
    </div>

    <div class="editor-layout">
      <!-- Left Panel: Form Fields -->
      <div class="form-panel">
        <!-- Basic Section -->
        <section class="form-section">
          <h3 class="section-title">Basic</h3>
          <div class="form-grid">
            <div class="form-field">
              <label for="input">Input</label>
              <input
                type="text"
                id="input"
                bind:value={editingMove.input}
              />
            </div>
            <div class="form-field">
              <label for="name">Name</label>
              <input
                type="text"
                id="name"
                bind:value={editingMove.name}
              />
            </div>
          </div>
        </section>

        <!-- Timing Section -->
        <section class="form-section">
          <h3 class="section-title">Timing</h3>
          <div class="form-grid four-col">
            <div class="form-field">
              <label for="startup">Startup</label>
              <input
                type="number"
                id="startup"
                min="0"
                bind:value={editingMove.startup}
              />
            </div>
            <div class="form-field">
              <label for="active">Active</label>
              <input
                type="number"
                id="active"
                min="0"
                bind:value={editingMove.active}
              />
            </div>
            <div class="form-field">
              <label for="recovery">Recovery</label>
              <input
                type="number"
                id="recovery"
                min="0"
                bind:value={editingMove.recovery}
              />
            </div>
            <div class="form-field computed">
              <label>Total</label>
              <span class="computed-value">{totalFrames}f</span>
            </div>
          </div>
        </section>

        <!-- Damage & Stun Section -->
        <section class="form-section">
          <h3 class="section-title">Damage & Stun</h3>
          <div class="form-grid four-col">
            <div class="form-field">
              <label for="damage">Damage</label>
              <input
                type="number"
                id="damage"
                min="0"
                bind:value={editingMove.damage}
              />
            </div>
            <div class="form-field">
              <label for="hitstun">Hitstun</label>
              <input
                type="number"
                id="hitstun"
                min="0"
                bind:value={editingMove.hitstun}
              />
            </div>
            <div class="form-field">
              <label for="blockstun">Blockstun</label>
              <input
                type="number"
                id="blockstun"
                min="0"
                bind:value={editingMove.blockstun}
              />
            </div>
            <div class="form-field">
              <label for="hitstop">Hitstop</label>
              <input
                type="number"
                id="hitstop"
                min="0"
                bind:value={editingMove.hitstop}
              />
            </div>
          </div>
        </section>

        <!-- Advantage Display Section -->
        <section class="form-section">
          <h3 class="section-title">Advantage</h3>
          <div class="form-grid">
            <div class="form-field computed">
              <label>On Hit</label>
              <span class="computed-value" class:positive={advantageOnHit >= 0}>
                {formatAdvantage(advantageOnHit)}
              </span>
            </div>
            <div class="form-field computed">
              <label>On Block</label>
              <span class="computed-value" class:positive={advantageOnBlock >= 0}>
                {formatAdvantage(advantageOnBlock)}
              </span>
            </div>
          </div>
        </section>

        <!-- Properties Section -->
        <section class="form-section">
          <h3 class="section-title">Properties</h3>
          <div class="form-grid">
            <div class="form-field">
              <label for="guard">Guard</label>
              <select id="guard" bind:value={editingMove.guard}>
                {#each guardOptions as option}
                  <option value={option}>{option}</option>
                {/each}
              </select>
            </div>
            <div class="form-field">
              <label for="animation">Animation</label>
              <input
                type="text"
                id="animation"
                bind:value={editingMove.animation}
              />
            </div>
          </div>
        </section>

        <!-- Pushback Section -->
        <section class="form-section">
          <h3 class="section-title">Pushback</h3>
          <div class="form-grid">
            <div class="form-field">
              <label for="pushback-hit">On Hit</label>
              <input
                type="number"
                id="pushback-hit"
                step="0.1"
                bind:value={editingMove.pushback.hit}
              />
            </div>
            <div class="form-field">
              <label for="pushback-block">On Block</label>
              <input
                type="number"
                id="pushback-block"
                step="0.1"
                bind:value={editingMove.pushback.block}
              />
            </div>
          </div>
        </section>

        <!-- Meter Gain Section -->
        <section class="form-section">
          <h3 class="section-title">Meter Gain</h3>
          <div class="form-grid">
            <div class="form-field">
              <label for="meter-hit">On Hit</label>
              <input
                type="number"
                id="meter-hit"
                min="0"
                bind:value={editingMove.meter_gain.hit}
              />
            </div>
            <div class="form-field">
              <label for="meter-whiff">On Whiff</label>
              <input
                type="number"
                id="meter-whiff"
                min="0"
                bind:value={editingMove.meter_gain.whiff}
              />
            </div>
          </div>
        </section>
      </div>

      <!-- Right Panel: Preview Placeholder -->
      <div class="preview-panel">
        <div class="preview-placeholder">
          <span class="placeholder-text">Animation Preview</span>
          <span class="placeholder-subtext">Coming in future update</span>
        </div>
      </div>
    </div>
  </div>
{:else}
  <div class="no-move">
    <p>No move selected. Select a move from the Frame Data table or use the dropdown above.</p>
  </div>
{/if}

<style>
  .move-editor-container {
    display: flex;
    flex-direction: column;
    height: 100%;
    gap: 16px;
  }

  .move-selector {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .move-selector label {
    font-weight: 600;
  }

  .move-selector select {
    min-width: 250px;
  }

  .unsaved-indicator {
    color: var(--warning);
    font-size: 12px;
    font-weight: 500;
  }

  .editor-layout {
    display: grid;
    grid-template-columns: 1fr 300px;
    gap: 24px;
    flex: 1;
    overflow: hidden;
  }

  .form-panel {
    overflow-y: auto;
    padding-right: 8px;
  }

  .form-section {
    margin-bottom: 20px;
    padding-bottom: 16px;
    border-bottom: 1px solid var(--border);
  }

  .form-section:last-child {
    border-bottom: none;
    margin-bottom: 0;
  }

  .section-title {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    margin-bottom: 12px;
  }

  .form-grid {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 12px;
  }

  .form-grid.four-col {
    grid-template-columns: repeat(4, 1fr);
  }

  .form-field {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .form-field label {
    font-size: 12px;
    color: var(--text-secondary);
  }

  .form-field input,
  .form-field select {
    width: 100%;
  }

  .form-field input[type="number"] {
    font-variant-numeric: tabular-nums;
  }

  .form-field.computed {
    background: var(--bg-secondary);
    padding: 8px;
    border-radius: 4px;
  }

  .computed-value {
    font-size: 18px;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
    color: var(--accent);
  }

  .computed-value.positive {
    color: var(--success);
  }

  .preview-panel {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 8px;
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 400px;
  }

  .preview-placeholder {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    color: var(--text-secondary);
  }

  .placeholder-text {
    font-size: 16px;
    font-weight: 600;
  }

  .placeholder-subtext {
    font-size: 12px;
  }

  .no-move {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-secondary);
  }
</style>
