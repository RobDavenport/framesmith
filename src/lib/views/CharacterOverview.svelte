<script lang="ts">
  import { getCurrentCharacter, getRulesRegistry, exportCharacter } from "$lib/stores/character.svelte";
  import DeleteCharacterModal from "$lib/components/DeleteCharacterModal.svelte";
  import type { State, CancelTable } from "$lib/types";

  let exportAdapter = $state("json-blob");
  let exportPretty = $state(true);
  let exportStatus = $state<string | null>(null);
  let showDeleteModal = $state(false);

  const characterData = $derived(getCurrentCharacter());
  const character = $derived(characterData?.character);
  const moves = $derived(characterData?.moves ?? []);
  const cancelTable = $derived(characterData?.cancel_table);
  const registry = $derived(getRulesRegistry());

  // Default filter groups if none defined in registry
  const defaultNormalTypes = ["normal", "command_normal"];
  const defaultSpecialTypes = ["special", "super", "ex", "rekka"];

  // Get filter group from registry or use defaults
  const normalTypes = $derived(
    registry?.move_types?.filter_groups?.["normals"] ?? defaultNormalTypes
  );
  const specialTypes = $derived(
    registry?.move_types?.filter_groups?.["specials"] ?? defaultSpecialTypes
  );

  // State categorization helpers using move.type field
  function isNormalMove(move: State): boolean {
    if (move.type) {
      return normalTypes.includes(move.type);
    }
    // Fallback: use input pattern if type not set
    return !/\d{3,}/.test(move.input);
  }

  function isSpecialMove(move: State): boolean {
    if (move.type) {
      return specialTypes.includes(move.type);
    }
    // Fallback: use input pattern if type not set
    return /\d{3,}/.test(move.input);
  }

  // Move statistics
  const normalMoves = $derived(moves.filter((m) => isNormalMove(m)));
  const specialMoves = $derived(moves.filter((m) => isSpecialMove(m)));

  const avgStartup = $derived.by(() => {
    if (moves.length === 0) return 0;
    const sum = moves.reduce((acc, m) => acc + m.startup, 0);
    return Math.round((sum / moves.length) * 10) / 10;
  });

  const avgDamage = $derived.by(() => {
    if (moves.length === 0) return 0;
    const sum = moves.reduce((acc, m) => acc + m.damage, 0);
    return Math.round((sum / moves.length) * 10) / 10;
  });

  // Cancel statistics
  const chainStarters = $derived.by(() => {
    if (!cancelTable) return 0;
    return Object.keys(cancelTable.chains).length;
  });

  const specialCancelCount = $derived(cancelTable?.special_cancels?.length ?? 0);
  const superCancelCount = $derived(cancelTable?.super_cancels?.length ?? 0);
  const jumpCancelCount = $derived(cancelTable?.jump_cancels?.length ?? 0);

  // Format property names: convert snake_case to Title Case
  function formatPropertyName(key: string): string {
    return key
      .split("_")
      .map((word) => word.charAt(0).toUpperCase() + word.slice(1))
      .join(" ");
  }

  // Format property values: numbers get formatted, booleans become Yes/No, strings as-is
  function formatPropertyValue(value: unknown): string {
    if (typeof value === "number") {
      // If it's a whole number, show without decimals; otherwise show one decimal
      return Number.isInteger(value) ? value.toString() : value.toFixed(1);
    }
    if (typeof value === "boolean") {
      return value ? "Yes" : "No";
    }
    return String(value);
  }

  async function handleExport() {
    if (!character) return;

    exportStatus = null;
    let extension: string;
    if (exportAdapter === "zx-fspack") {
      extension = "fspk";
    } else if (exportAdapter === "json-blob") {
      extension = "json";
    } else {
      extension = "rs";
    }
    const filename = `${character.id}.${extension}`;
    const outputPath = `exports/${filename}`;

    try {
      await exportCharacter(exportAdapter, outputPath, exportPretty);
      exportStatus = `Exported to ${outputPath}`;
      setTimeout(() => { exportStatus = null; }, 3000);
    } catch (e) {
      exportStatus = `Error: ${e}`;
    }
  }
</script>

{#if character}
  <div class="overview-container">
    <!-- Header -->
    <div class="character-header">
      <h1 class="character-name">{character.name}</h1>
      <span class="archetype-badge">{character.archetype}</span>
    </div>

    <div class="stats-grid">
      <!-- Properties Card -->
      <div class="stats-card">
        <h3 class="card-title">Properties</h3>
        <div class="stat-list">
          {#each Object.entries(character.properties ?? {}) as [key, value]}
            <div class="stat-row">
              <span class="stat-label">{formatPropertyName(key)}</span>
              <span class="stat-value">{formatPropertyValue(value)}</span>
            </div>
          {/each}
        </div>
      </div>

      <!-- Move Summary Card -->
      <div class="stats-card">
        <h3 class="card-title">Move Summary</h3>
        <div class="stat-list">
          <div class="stat-row">
            <span class="stat-label">Total Moves</span>
            <span class="stat-value highlight">{moves.length}</span>
          </div>
          <div class="stat-row">
            <span class="stat-label">Normals</span>
            <span class="stat-value">{normalMoves.length}</span>
          </div>
          <div class="stat-row">
            <span class="stat-label">Specials</span>
            <span class="stat-value">{specialMoves.length}</span>
          </div>
          <div class="stat-row">
            <span class="stat-label">Avg Startup</span>
            <span class="stat-value">{avgStartup}f</span>
          </div>
          <div class="stat-row">
            <span class="stat-label">Avg Damage</span>
            <span class="stat-value">{avgDamage}</span>
          </div>
        </div>
      </div>

      <!-- Cancel Routes Card -->
      <div class="stats-card">
        <h3 class="card-title">Cancel Routes</h3>
        <div class="stat-list">
          <div class="stat-row">
            <span class="stat-label">Chain Starters</span>
            <span class="stat-value">{chainStarters}</span>
          </div>
          <div class="stat-row">
            <span class="stat-label">Special Cancels</span>
            <span class="stat-value">{specialCancelCount}</span>
          </div>
          <div class="stat-row">
            <span class="stat-label">Super Cancels</span>
            <span class="stat-value">{superCancelCount}</span>
          </div>
          <div class="stat-row">
            <span class="stat-label">Jump Cancels</span>
            <span class="stat-value">{jumpCancelCount}</span>
          </div>
        </div>
      </div>
    </div>

    <div class="export-section">
      <h3>Export</h3>
      <div class="export-controls">
        <select bind:value={exportAdapter}>
          <option value="json-blob">JSON Blob</option>
          <option value="zx-fspack">ZX FSPK (Binary)</option>
        </select>
        <label class="checkbox-label">
          <input
            type="checkbox"
            bind:checked={exportPretty}
            disabled={exportAdapter === "zx-fspack"}
          />
          Pretty print
        </label>
        <button onclick={handleExport}>Export Character</button>
        {#if exportStatus}
          <span class="export-status" class:error={exportStatus.includes("Error")}>
            {exportStatus}
          </span>
        {/if}
      </div>
    </div>

    <div class="danger-zone">
      <h3>Danger Zone</h3>
      <div class="danger-content">
        <div class="danger-info">
          <span class="danger-title">Delete this character</span>
          <span class="danger-desc">Once deleted, this character cannot be recovered.</span>
        </div>
        <button class="delete-btn" onclick={() => showDeleteModal = true}>
          Delete Character
        </button>
      </div>
    </div>

    <DeleteCharacterModal
      open={showDeleteModal}
      characterId={character.id}
      characterName={character.name}
      onClose={() => showDeleteModal = false}
    />
  </div>
{/if}

<style>
  .overview-container {
    max-width: 1000px;
    margin: 0 auto;
  }

  .character-header {
    display: flex;
    align-items: center;
    gap: 16px;
    margin-bottom: 24px;
  }

  .character-name {
    font-size: 28px;
    font-weight: 700;
    margin: 0;
  }

  .archetype-badge {
    background: var(--accent);
    color: var(--text-primary);
    padding: 4px 12px;
    border-radius: 16px;
    font-size: 13px;
    font-weight: 600;
    text-transform: capitalize;
  }

  .stats-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
    gap: 16px;
  }

  .stats-card {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 16px;
  }

  .card-title {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-secondary);
    margin: 0 0 12px 0;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .stat-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .stat-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 4px 0;
    border-bottom: 1px solid var(--border);
  }

  .stat-row:last-child {
    border-bottom: none;
  }

  .stat-label {
    color: var(--text-secondary);
    font-size: 13px;
  }

  .stat-value {
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }

  .stat-value.highlight {
    color: var(--accent);
    font-size: 16px;
  }

  .export-section {
    margin-top: 24px;
    padding-top: 24px;
    border-top: 1px solid var(--border);
  }

  .export-section h3 {
    font-size: 14px;
    font-weight: 600;
    margin-bottom: 12px;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .export-controls {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
  }

  .checkbox-label {
    display: flex;
    align-items: center;
    gap: 6px;
    cursor: pointer;
  }

  .export-status {
    font-size: 13px;
    color: var(--success);
  }

  .export-status.error {
    color: var(--accent);
  }

  .danger-zone {
    margin-top: 48px;
    padding-top: 24px;
    border-top: 1px solid var(--error, #ef4444);
  }

  .danger-zone h3 {
    font-size: 14px;
    font-weight: 600;
    margin-bottom: 12px;
    color: var(--error, #ef4444);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .danger-content {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 16px;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 8px;
  }

  .danger-info {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .danger-title {
    font-weight: 600;
    font-size: 14px;
  }

  .danger-desc {
    font-size: 13px;
    color: var(--text-secondary);
  }

  .delete-btn {
    background: transparent;
    border: 1px solid var(--error, #ef4444);
    color: var(--error, #ef4444);
    flex-shrink: 0;
  }

  .delete-btn:hover {
    background: var(--error, #ef4444);
    color: white;
  }
</style>
