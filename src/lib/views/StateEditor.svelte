<script lang="ts">
  import {
    characterStore,
    selectMove,
    saveMove,
  } from "$lib/stores/character.svelte";
  import { getAssets, getAssetsError, isAssetsLoading } from "$lib/stores/assets.svelte";
  import MoveAnimationPreview from "$lib/components/MoveAnimationPreview.svelte";
  import type { State, TriggerType, Precondition, Cost, HitboxShape, StatusEffect } from "$lib/types";

  // Common move types - custom types can be entered directly
  const commonMoveTypes = ["normal", "command_normal", "special", "super", "movement", "throw", "ex", "rekka"];
  const triggerOptions: TriggerType[] = ["press", "release", "hold"];

  // Use reactive getters from characterStore for proper dependency tracking
  const characterData = $derived(characterStore.currentCharacter);
  const selectedMoveInputValue = $derived(characterStore.selectedMoveInput);
  const selectedMoveValue = $derived(characterStore.selectedMove);

  const moves = $derived(characterData?.moves ?? []);
  const characterId = $derived(characterData?.character.id ?? null);
  const assets = $derived(getAssets());
  const assetsLoading = $derived(isAssetsLoading());
  const assetsError = $derived(getAssetsError());

  // Local editing state - copy of the state data
  let editingMove = $state<State | null>(null);

  // Collapsible section states
  let showPreconditions = $state(false);
  let showCosts = $state(false);
  let showPushboxes = $state(false);

  // Watch for selected move changes and create a local copy
  $effect(() => {
    if (selectedMoveValue) {
      editingMove = structuredClone(selectedMoveValue);
    } else if (moves.length > 0 && !selectedMoveInputValue) {
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
    if (!editingMove || !selectedMoveValue) return false;
    return JSON.stringify(editingMove) !== JSON.stringify(selectedMoveValue);
  });

  function handleMoveSelect(event: Event) {
    const target = event.target as HTMLSelectElement;
    selectMove(target.value);
  }

  function formatAdvantage(value: number): string {
    return value >= 0 ? `+${value}` : String(value);
  }

  const guardOptions: State["guard"][] = ["high", "mid", "low", "unblockable"];

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

  function addPushbox() {
    if (!editingMove) return;
    const newPushbox = {
      frames: [1, editingMove.startup + editingMove.active + editingMove.recovery] as [number, number],
      box: { x: 0, y: 0, w: 50, h: 100 }
    };
    editingMove.pushboxes = [...(editingMove.pushboxes ?? []), newPushbox];
  }

  function removePushbox(index: number) {
    if (!editingMove?.pushboxes) return;
    editingMove.pushboxes = editingMove.pushboxes.filter((_, i) => i !== index);
    if (editingMove.pushboxes.length === 0) {
      editingMove.pushboxes = undefined;
    }
  }
</script>

<div class="move-editor-container">
  <!-- Move Selector - always visible so users can select a move -->
  <div class="move-selector">
    <label for="move-select">Move:</label>
    <select id="move-select" value={selectedMoveInputValue ?? ""} onchange={handleMoveSelect}>
      {#if moves.length === 0}
        <option value="">No moves available</option>
      {:else}
        <option value="" disabled>Select a move...</option>
        {#each moves as move}
          <option value={move.input}>{move.input} - {move.name}</option>
        {/each}
      {/if}
    </select>
    {#if hasChanges}
      <span class="unsaved-indicator">Unsaved changes</span>
    {/if}
  </div>

  {#if editingMove}
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
              <span class="computed-label">Total</span>
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
              <span class="computed-label">On Hit</span>
              <span class="computed-value" class:positive={advantageOnHit >= 0}>
                {formatAdvantage(advantageOnHit)}
              </span>
            </div>
            <div class="form-field computed">
              <span class="computed-label">On Block</span>
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
              <input
                type="text"
                id="move-type"
                list="move-type-options"
                placeholder="normal, special, super..."
                bind:value={editingMove.type}
              />
              <datalist id="move-type-options">
                {#each commonMoveTypes as option}
                  <option value={option}>{option}</option>
                {/each}
              </datalist>
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

        <!-- Pushboxes Section -->
        <section class="form-section">
          <button type="button" class="section-title collapsible" onclick={() => showPushboxes = !showPushboxes}>
            Pushboxes {editingMove.pushboxes?.length ? `(${editingMove.pushboxes.length})` : ''}
            <span class="collapse-icon">{showPushboxes ? '▼' : '▶'}</span>
          </button>
          {#if showPushboxes}
            <div class="array-editor">
              {#if editingMove.pushboxes?.length}
                {#each editingMove.pushboxes as pb, i}
                  <div class="pushbox-item">
                    <div class="pushbox-header">
                      <span class="item-label">Pushbox {i + 1}</span>
                      <button class="remove-btn" onclick={() => removePushbox(i)}>×</button>
                    </div>
                    <div class="pushbox-fields">
                      <div class="pushbox-row">
                        <label for="pb-{i}-frame-start">Frames:</label>
                        <input
                          id="pb-{i}-frame-start"
                          type="number"
                          min="1"
                          value={pb.frames[0]}
                          oninput={(e) => pb.frames = [parseInt(e.currentTarget.value) || 1, pb.frames[1]]}
                        />
                        <span>-</span>
                        <input
                          id="pb-{i}-frame-end"
                          aria-label="Frame end"
                          type="number"
                          min="1"
                          value={pb.frames[1]}
                          oninput={(e) => pb.frames = [pb.frames[0], parseInt(e.currentTarget.value) || 1]}
                        />
                      </div>
                      <div class="pushbox-row">
                        <label for="pb-{i}-x">X:</label>
                        <input
                          id="pb-{i}-x"
                          type="number"
                          value={pb.box.x}
                          oninput={(e) => pb.box.x = parseInt(e.currentTarget.value) || 0}
                        />
                        <label for="pb-{i}-y">Y:</label>
                        <input
                          id="pb-{i}-y"
                          type="number"
                          value={pb.box.y}
                          oninput={(e) => pb.box.y = parseInt(e.currentTarget.value) || 0}
                        />
                      </div>
                      <div class="pushbox-row">
                        <label for="pb-{i}-w">W:</label>
                        <input
                          id="pb-{i}-w"
                          type="number"
                          min="1"
                          value={pb.box.w}
                          oninput={(e) => pb.box.w = parseInt(e.currentTarget.value) || 1}
                        />
                        <label for="pb-{i}-h">H:</label>
                        <input
                          id="pb-{i}-h"
                          type="number"
                          min="1"
                          value={pb.box.h}
                          oninput={(e) => pb.box.h = parseInt(e.currentTarget.value) || 1}
                        />
                      </div>
                    </div>
                  </div>
                {/each}
              {:else}
                <p class="empty-hint">No pushboxes defined</p>
              {/if}
              <button type="button" class="add-btn" onclick={addPushbox}>+ Add Pushbox</button>
            </div>
          {/if}
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
          selectionKey={selectedMoveInputValue}
          move={editingMove}
          onMoveChange={(m) => (editingMove = m)}
          assets={assets}
          assetsLoading={assetsLoading}
          assetsError={assetsError}
        />
      </div>
    </div>
  {:else}
    <div class="no-move">
      <p>No move selected. Select a move from the dropdown above or the Frame Data table.</p>
    </div>
  {/if}
</div>

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

  .form-field label,
  .form-field .computed-label {
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

  .pushbox-item {
    background: var(--bg-secondary);
    border-radius: 4px;
    padding: 12px;
  }

  .pushbox-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 8px;
  }

  .pushbox-fields {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .pushbox-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .pushbox-row label {
    font-size: 12px;
    color: var(--text-secondary);
    min-width: 50px;
  }

  .pushbox-row input {
    width: 60px;
    font-variant-numeric: tabular-nums;
  }

  .pushbox-row span {
    color: var(--text-secondary);
  }

  .empty-hint {
    color: var(--text-secondary);
    font-size: 13px;
    font-style: italic;
    padding: 8px 0;
  }

  .add-btn {
    background: transparent;
    border: 1px dashed var(--border);
    color: var(--text-secondary);
    padding: 8px 16px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 13px;
    transition: all 0.2s;
  }

  .add-btn:hover {
    border-color: var(--accent);
    color: var(--accent);
  }
</style>
