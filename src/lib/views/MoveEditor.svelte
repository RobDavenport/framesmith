<script lang="ts">
  import {
    getCurrentCharacter,
    getSelectedMove,
    getSelectedMoveInput,
    selectMove,
    saveMove,
  } from "$lib/stores/character.svelte";
  import { getAssets, getAssetsError, isAssetsLoading } from "$lib/stores/assets.svelte";
  import MoveAnimationPreview from "$lib/components/MoveAnimationPreview.svelte";
  import type { Move, MoveType, TriggerType, Precondition, Cost, HitboxShape, StatusEffect } from "$lib/types";

  const moveTypeOptions: MoveType[] = ["normal", "command_normal", "special", "super", "movement", "throw"];
  const triggerOptions: TriggerType[] = ["press", "release", "hold"];

  const characterData = $derived(getCurrentCharacter());
  const moves = $derived(characterData?.moves ?? []);
  const selectedMoveInput = $derived(getSelectedMoveInput());
  const selectedMove = $derived(getSelectedMove());
  const characterId = $derived(characterData?.character.id ?? null);
  const assets = $derived(getAssets());
  const assetsLoading = $derived(isAssetsLoading());
  const assetsError = $derived(getAssetsError());

  // Local editing state - copy of the move data
  let editingMove = $state<Move | null>(null);

  // Collapsible section states
  let showPreconditions = $state(false);
  let showCosts = $state(false);

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

  let saveStatus = $state<string | null>(null);

  async function handleSave() {
    if (!editingMove) return;

    saveStatus = null;
    try {
      await saveMove(editingMove);
      saveStatus = "Saved!";
      setTimeout(() => { saveStatus = null; }, 2000);
    } catch (e) {
      saveStatus = `Error: ${e}`;
    }
  }

  // Helper functions for v2 fields
  function addPrecondition(type: string) {
    if (!type) return;
    if (!editingMove) return;

    const newPrecondition: Precondition = type === 'grounded'
      ? { type: 'grounded' }
      : type === 'airborne'
      ? { type: 'airborne' }
      : type === 'meter'
      ? { type: 'meter', min: 25 }
      : type === 'charge'
      ? { type: 'charge', direction: '4', min_frames: 45 }
      : type === 'state'
      ? { type: 'state', in: '' }
      : { type: 'health', max_percent: 30 };

    editingMove.preconditions = [...(editingMove.preconditions ?? []), newPrecondition];
  }

  function removePrecondition(index: number) {
    if (!editingMove?.preconditions) return;
    editingMove.preconditions = editingMove.preconditions.filter((_, i) => i !== index);
    if (editingMove.preconditions.length === 0) {
      editingMove.preconditions = undefined;
    }
  }

  function addCost(type: string) {
    if (!type) return;
    if (!editingMove) return;

    const newCost: Cost = type === 'meter'
      ? { type: 'meter', amount: 25 }
      : type === 'health'
      ? { type: 'health', amount: 10 }
      : { type: 'resource', name: 'custom', amount: 1 };

    editingMove.costs = [...(editingMove.costs ?? []), newCost];
  }

  function removeCost(index: number) {
    if (!editingMove?.costs) return;
    editingMove.costs = editingMove.costs.filter((_, i) => i !== index);
    if (editingMove.costs.length === 0) {
      editingMove.costs = undefined;
    }
  }

  function updateMovement(field: string, value: any) {
    if (!editingMove) return;
    if (!editingMove.movement) {
      editingMove.movement = { distance: 0, direction: 'forward' };
    }
    (editingMove.movement as any)[field] = value;
  }

  function updateSuperFreeze(field: string, value: any) {
    if (!editingMove) return;
    if (!editingMove.super_freeze) {
      editingMove.super_freeze = { frames: 45 };
    }
    (editingMove.super_freeze as any)[field] = value;
  }
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

        <!-- Move Type & Trigger Section -->
        <section class="form-section">
          <h3 class="section-title">Move Type</h3>
          <div class="form-grid">
            <div class="form-field">
              <label for="move-type">Type</label>
              <select id="move-type" bind:value={editingMove.type}>
                <option value={undefined}>-- Default --</option>
                {#each moveTypeOptions as option}
                  <option value={option}>{option}</option>
                {/each}
              </select>
            </div>
            <div class="form-field">
              <label for="trigger">Trigger</label>
              <select id="trigger" bind:value={editingMove.trigger}>
                <option value={undefined}>press (default)</option>
                {#each triggerOptions as option}
                  <option value={option}>{option}</option>
                {/each}
              </select>
            </div>
          </div>
          <div class="form-grid" style="margin-top: 12px;">
            <div class="form-field">
              <label for="parent">Parent Move</label>
              <input
                type="text"
                id="parent"
                placeholder="e.g., 236K for follow-up"
                bind:value={editingMove.parent}
              />
            </div>
            <div class="form-field">
              <label for="total">Total Frames (override)</label>
              <input
                type="number"
                id="total"
                min="0"
                bind:value={editingMove.total}
              />
            </div>
          </div>
        </section>

        <!-- Preconditions Section -->
        <section class="form-section">
          <button type="button" class="section-title collapsible" onclick={() => showPreconditions = !showPreconditions}>
            Preconditions {editingMove.preconditions?.length ? `(${editingMove.preconditions.length})` : ''}
            <span class="collapse-icon">{showPreconditions ? '▼' : '▶'}</span>
          </button>
          {#if showPreconditions}
            <div class="array-editor">
              {#if editingMove.preconditions}
                {#each editingMove.preconditions as precondition, i}
                  <div class="array-item">
                    <span class="item-label">{precondition.type}</span>
                    <button class="remove-btn" onclick={() => removePrecondition(i)}>×</button>
                  </div>
                {/each}
              {/if}
              <select class="add-select" onchange={(e) => addPrecondition(e.currentTarget.value)}>
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

        <!-- Costs Section -->
        <section class="form-section">
          <button type="button" class="section-title collapsible" onclick={() => showCosts = !showCosts}>
            Costs {editingMove.costs?.length ? `(${editingMove.costs.length})` : ''}
            <span class="collapse-icon">{showCosts ? '▼' : '▶'}</span>
          </button>
          {#if showCosts}
            <div class="array-editor">
              {#if editingMove.costs}
                {#each editingMove.costs as cost, i}
                  <div class="array-item">
                    <span class="item-label">{cost.type}: {cost.amount}</span>
                    <button class="remove-btn" onclick={() => removeCost(i)}>×</button>
                  </div>
                {/each}
              {/if}
              <select class="add-select" onchange={(e) => addCost(e.currentTarget.value)}>
                <option value="">+ Add cost...</option>
                <option value="meter">Meter cost</option>
                <option value="health">Health cost</option>
                <option value="resource">Resource cost</option>
              </select>
            </div>
          {/if}
        </section>

        <!-- Movement Section (for movement type moves) -->
        {#if editingMove.type === 'movement' || editingMove.movement}
        <section class="form-section">
          <h3 class="section-title">Movement</h3>
          <div class="form-grid four-col">
            <div class="form-field">
              <label for="move-distance">Distance</label>
              <input
                type="number"
                id="move-distance"
                min="0"
                value={editingMove.movement?.distance ?? 0}
                oninput={(e) => updateMovement('distance', parseInt(e.currentTarget.value))}
              />
            </div>
            <div class="form-field">
              <label for="move-direction">Direction</label>
              <select
                id="move-direction"
                value={editingMove.movement?.direction ?? 'forward'}
                onchange={(e) => updateMovement('direction', e.currentTarget.value)}
              >
                <option value="forward">Forward</option>
                <option value="backward">Backward</option>
              </select>
            </div>
            <div class="form-field">
              <label for="move-curve">Curve</label>
              <select
                id="move-curve"
                value={editingMove.movement?.curve ?? ''}
                onchange={(e) => updateMovement('curve', e.currentTarget.value || undefined)}
              >
                <option value="">Linear</option>
                <option value="ease-in">Ease In</option>
                <option value="ease-out">Ease Out</option>
                <option value="ease-in-out">Ease In-Out</option>
              </select>
            </div>
            <div class="form-field">
              <label for="move-airborne">Airborne</label>
              <input
                type="checkbox"
                id="move-airborne"
                checked={editingMove.movement?.airborne ?? false}
                onchange={(e) => updateMovement('airborne', e.currentTarget.checked)}
              />
            </div>
          </div>
        </section>
        {/if}

        <!-- Super Freeze Section (for super type moves) -->
        {#if editingMove.type === 'super' || editingMove.super_freeze}
        <section class="form-section">
          <h3 class="section-title">Super Freeze</h3>
          <div class="form-grid four-col">
            <div class="form-field">
              <label for="freeze-frames">Frames</label>
              <input
                type="number"
                id="freeze-frames"
                min="0"
                value={editingMove.super_freeze?.frames ?? 0}
                oninput={(e) => updateSuperFreeze('frames', parseInt(e.currentTarget.value))}
              />
            </div>
            <div class="form-field">
              <label for="freeze-zoom">Zoom</label>
              <input
                type="number"
                id="freeze-zoom"
                min="0"
                step="0.1"
                value={editingMove.super_freeze?.zoom ?? 1.0}
                oninput={(e) => updateSuperFreeze('zoom', parseFloat(e.currentTarget.value))}
              />
            </div>
            <div class="form-field">
              <label for="freeze-darken">Darken</label>
              <input
                type="number"
                id="freeze-darken"
                min="0"
                max="1"
                step="0.1"
                value={editingMove.super_freeze?.darken ?? 0}
                oninput={(e) => updateSuperFreeze('darken', parseFloat(e.currentTarget.value))}
              />
            </div>
            <div class="form-field">
              <label for="freeze-flash">Flash</label>
              <input
                type="checkbox"
                id="freeze-flash"
                checked={editingMove.super_freeze?.flash ?? false}
                onchange={(e) => updateSuperFreeze('flash', e.currentTarget.checked)}
              />
            </div>
          </div>
        </section>
        {/if}

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

        <div class="form-actions">
          <button class="save-btn" onclick={handleSave}>Save Move</button>
          {#if saveStatus}
            <span class="save-status" class:error={saveStatus.includes("Error")}>
              {saveStatus}
            </span>
          {/if}
        </div>
      </div>

      <!-- Right Panel: Preview Placeholder -->
      <div class="preview-panel">
        <MoveAnimationPreview
          characterId={characterId}
          selectionKey={selectedMoveInput}
          move={editingMove}
          onMoveChange={(m) => (editingMove = m)}
          assets={assets}
          assetsLoading={assetsLoading}
          assetsError={assetsError}
        />
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

  .preview-panel {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 8px;
    display: flex;
    min-height: 400px;
    overflow: hidden;
  }

  .no-move {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-secondary);
  }

  .form-actions {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-top: 16px;
    padding-top: 16px;
    border-top: 1px solid var(--border);
  }

  .save-btn {
    background: var(--accent);
    border-color: var(--accent);
  }

  .save-btn:hover {
    background: var(--accent-hover);
    border-color: var(--accent-hover);
  }

  .save-status {
    font-size: 13px;
    color: var(--success);
  }

  .save-status.error {
    color: var(--accent);
  }
</style>
