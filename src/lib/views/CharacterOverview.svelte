<script lang="ts">
  import { getCurrentCharacter } from "$lib/stores/character.svelte";
  import type { Move, CancelTable } from "$lib/types";

  const characterData = $derived(getCurrentCharacter());
  const character = $derived(characterData?.character);
  const moves = $derived(characterData?.moves ?? []);
  const cancelTable = $derived(characterData?.cancel_table);

  // Move categorization helpers
  function isSpecialMove(input: string): boolean {
    return /\d{3,}/.test(input); // Contains 3+ consecutive digits (motion input)
  }

  function isNormalMove(input: string): boolean {
    return !isSpecialMove(input);
  }

  // Move statistics
  const normalMoves = $derived(moves.filter((m) => isNormalMove(m.input)));
  const specialMoves = $derived(moves.filter((m) => isSpecialMove(m.input)));

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

  // Format speed values
  function formatSpeed(value: number): string {
    return value.toFixed(1);
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
          <div class="stat-row">
            <span class="stat-label">Health</span>
            <span class="stat-value">{character.health}</span>
          </div>
          <div class="stat-row">
            <span class="stat-label">Walk Speed</span>
            <span class="stat-value">{formatSpeed(character.walk_speed)}</span>
          </div>
          <div class="stat-row">
            <span class="stat-label">Back Walk Speed</span>
            <span class="stat-value">{formatSpeed(character.back_walk_speed)}</span>
          </div>
          <div class="stat-row">
            <span class="stat-label">Jump Height</span>
            <span class="stat-value">{formatSpeed(character.jump_height)}</span>
          </div>
          <div class="stat-row">
            <span class="stat-label">Jump Duration</span>
            <span class="stat-value">{character.jump_duration}f</span>
          </div>
          <div class="stat-row">
            <span class="stat-label">Dash Distance</span>
            <span class="stat-value">{formatSpeed(character.dash_distance)}</span>
          </div>
          <div class="stat-row">
            <span class="stat-label">Dash Duration</span>
            <span class="stat-value">{character.dash_duration}f</span>
          </div>
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
</style>
