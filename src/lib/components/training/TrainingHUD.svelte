<script lang="ts">
  /**
   * TrainingHUD - Displays health bars, resource meters, and frame data.
   *
   * This component provides the heads-up display for training mode,
   * showing character status and move information.
   */

  interface CharacterStatus {
    /** Current health (0-max). */
    health: number;
    /** Maximum health. */
    maxHealth: number;
    /** Resource values (meter, heat, etc.). */
    resources: Array<{
      name: string;
      value: number;
      max: number;
    }>;
  }

  interface MoveInfo {
    /** Move name/input notation. */
    name: string;
    /** Startup frames. */
    startup: number;
    /** Active frames. */
    active: number;
    /** Recovery frames. */
    recovery: number;
    /** Current frame within the move. */
    currentFrame: number;
    /** Total frames in the move. */
    totalFrames: number;
    /** Frame advantage on hit. */
    advantageOnHit?: number;
    /** Frame advantage on block. */
    advantageOnBlock?: number;
  }

  interface DummyStatus {
    /** Current dummy state label. */
    stateLabel: string;
  }

  interface ComboInfo {
    /** Current combo hit count. */
    hitCount: number;
    /** Total combo damage. */
    totalDamage: number;
  }

  interface Props {
    /** Player character status. */
    player: CharacterStatus;
    /** Dummy character status. */
    dummy: CharacterStatus;
    /** Current move info (null if idle). */
    currentMove: MoveInfo | null;
    /** Dummy status display. */
    dummyStatus: DummyStatus;
    /** List of available cancel move names. */
    availableCancels?: string[];
    /** Current combo information. */
    comboInfo?: ComboInfo;
    /** Callback to reset health bars. */
    onResetHealth?: () => void;
  }

  let {
    player,
    dummy,
    currentMove,
    dummyStatus,
    availableCancels = [],
    comboInfo,
    onResetHealth,
  }: Props = $props();

  // Format frame advantage with sign
  function formatAdvantage(value: number | undefined): string {
    if (value === undefined) return '?';
    if (value > 0) return `+${value}`;
    return String(value);
  }

  // Format cancels list, truncating if too long
  function formatCancels(cancels: string[]): string {
    if (cancels.length === 0) return 'None';
    if (cancels.length <= 4) return cancels.join(', ');
    return cancels.slice(0, 4).join(', ') + `, +${cancels.length - 4}`;
  }

  // Calculate health bar percentages
  const playerHealthPercent = $derived(
    Math.max(0, Math.min(100, (player.health / player.maxHealth) * 100))
  );
  const dummyHealthPercent = $derived(
    Math.max(0, Math.min(100, (dummy.health / dummy.maxHealth) * 100))
  );

  // Format frame data display
  function formatFrameData(move: MoveInfo): string {
    return `${move.startup}/${move.active}/${move.recovery}`;
  }
</script>

<div class="hud">
  <!-- Health bars row -->
  <div class="health-row">
    <!-- Player health -->
    <div class="health-section player">
      <div class="health-label">P1</div>
      <div class="health-bar-container">
        <div class="health-bar" style:width="{playerHealthPercent}%"></div>
      </div>
      <div class="health-value">{player.health}/{player.maxHealth}</div>
    </div>

    <!-- Reset button -->
    {#if onResetHealth}
      <button class="reset-btn" onclick={onResetHealth} title="Reset health (R)">
        Reset
      </button>
    {/if}

    <!-- Dummy health -->
    <div class="health-section dummy">
      <div class="health-label">CPU</div>
      <div class="health-bar-container">
        <div class="health-bar" style:width="{dummyHealthPercent}%"></div>
      </div>
      <div class="health-value">{dummy.health}/{dummy.maxHealth}</div>
    </div>
  </div>

  <!-- Resource meters row -->
  <div class="resources-row">
    <!-- Player resources -->
    <div class="resources-section">
      {#each player.resources as resource}
        <div class="resource">
          <span class="resource-name">{resource.name}</span>
          <div class="resource-bar-container">
            <div
              class="resource-bar"
              style:width="{(resource.value / resource.max) * 100}%"
            ></div>
          </div>
          <span class="resource-value">{resource.value}</span>
        </div>
      {/each}
    </div>

    <!-- Dummy resources -->
    <div class="resources-section">
      {#each dummy.resources as resource}
        <div class="resource">
          <span class="resource-name">{resource.name}</span>
          <div class="resource-bar-container">
            <div
              class="resource-bar"
              style:width="{(resource.value / resource.max) * 100}%"
            ></div>
          </div>
          <span class="resource-value">{resource.value}</span>
        </div>
      {/each}
    </div>
  </div>

  <!-- Frame data row -->
  <div class="frame-data-row">
    <!-- Current move info -->
    <div class="move-section">
      <div class="move-info">
        {#if currentMove}
          <span class="move-name">{currentMove.name}</span>
          <span class="frame-data">({formatFrameData(currentMove)})</span>
          <span class="frame-counter">frame {currentMove.currentFrame}</span>
        {:else}
          <span class="move-name idle">Idle</span>
        {/if}
      </div>
      {#if availableCancels.length > 0}
        <div class="cancels-info">
          <span class="cancels-label">Cancels:</span>
          <span class="cancels-list">{formatCancels(availableCancels)}</span>
        </div>
      {/if}
    </div>

    <!-- Frame advantage & dummy state -->
    <div class="advantage-section">
      <div class="dummy-state">
        <span class="state-label">{dummyStatus.stateLabel}</span>
      </div>
      {#if currentMove}
        <div class="frame-advantage">
          <span class="advantage-item on-hit">
            {formatAdvantage(currentMove.advantageOnHit)} on hit
          </span>
          <span class="advantage-item on-block">
            {formatAdvantage(currentMove.advantageOnBlock)} on block
          </span>
        </div>
      {/if}
    </div>

    <!-- Combo display -->
    <div class="combo-section">
      {#if comboInfo && comboInfo.hitCount > 0}
        <div class="combo-display">
          <span class="combo-hits">{comboInfo.hitCount} hits</span>
          <span class="combo-damage">{comboInfo.totalDamage}</span>
        </div>
      {:else}
        <div class="combo-display empty">
          <span class="combo-label">Combo</span>
        </div>
      {/if}
    </div>
  </div>
</div>

<style>
  .hud {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 8px;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 4px;
  }

  /* Health bars */
  .health-row {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .health-section {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .health-section.player {
    flex-direction: row;
  }

  .health-section.dummy {
    flex-direction: row-reverse;
  }

  .health-label {
    font-size: 11px;
    font-weight: 600;
    color: var(--text-secondary);
    min-width: 28px;
  }

  .health-bar-container {
    flex: 1;
    height: 16px;
    background: var(--bg-primary);
    border-radius: 2px;
    overflow: hidden;
  }

  .health-section.player .health-bar {
    height: 100%;
    background: linear-gradient(to right, var(--success), #22c55e);
    transition: width 0.1s ease-out;
  }

  .health-section.dummy .health-bar {
    height: 100%;
    background: linear-gradient(to left, var(--accent), #f43f5e);
    margin-left: auto;
    transition: width 0.1s ease-out;
  }

  .health-value {
    font-size: 11px;
    font-variant-numeric: tabular-nums;
    color: var(--text-secondary);
    min-width: 60px;
    text-align: center;
  }

  .reset-btn {
    padding: 4px 8px;
    font-size: 10px;
    background: var(--bg-tertiary);
  }

  /* Resource meters */
  .resources-row {
    display: flex;
    justify-content: space-between;
    gap: 24px;
  }

  .resources-section {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .resource {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .resource-name {
    font-size: 10px;
    color: var(--text-secondary);
    min-width: 40px;
  }

  .resource-bar-container {
    flex: 1;
    height: 8px;
    background: var(--bg-primary);
    border-radius: 2px;
    overflow: hidden;
    max-width: 80px;
  }

  .resource-bar {
    height: 100%;
    background: var(--warning);
    transition: width 0.1s ease-out;
  }

  .resource-value {
    font-size: 10px;
    font-variant-numeric: tabular-nums;
    color: var(--text-secondary);
    min-width: 24px;
    text-align: right;
  }

  /* Frame data */
  .frame-data-row {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    padding-top: 4px;
    border-top: 1px solid var(--border);
    gap: 16px;
  }

  .move-section {
    display: flex;
    flex-direction: column;
    gap: 4px;
    flex: 1;
  }

  .move-info {
    display: flex;
    align-items: baseline;
    gap: 8px;
  }

  .move-name {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .move-name.idle {
    color: var(--text-secondary);
    font-weight: 400;
  }

  .frame-data {
    font-size: 12px;
    color: var(--text-secondary);
    font-variant-numeric: tabular-nums;
  }

  .frame-counter {
    font-size: 11px;
    color: var(--accent);
    font-variant-numeric: tabular-nums;
  }

  .cancels-info {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 11px;
  }

  .cancels-label {
    color: var(--text-secondary);
  }

  .cancels-list {
    color: var(--text-primary);
    font-family: monospace;
  }

  .advantage-section {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
  }

  .dummy-state {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .state-label {
    font-size: 12px;
    color: var(--text-secondary);
    padding: 2px 8px;
    background: var(--bg-tertiary);
    border-radius: 4px;
  }

  .frame-advantage {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
  }

  .advantage-item {
    font-size: 10px;
    font-variant-numeric: tabular-nums;
    padding: 1px 6px;
    border-radius: 3px;
  }

  .advantage-item.on-hit {
    color: var(--success);
    background: rgba(74, 222, 128, 0.15);
  }

  .advantage-item.on-block {
    color: var(--warning);
    background: rgba(251, 191, 36, 0.15);
  }

  .combo-section {
    display: flex;
    align-items: center;
  }

  .combo-display {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 4px 12px;
    background: var(--bg-tertiary);
    border-radius: 4px;
    min-width: 80px;
  }

  .combo-display.empty {
    opacity: 0.5;
  }

  .combo-hits {
    font-size: 14px;
    font-weight: 600;
    color: var(--accent);
  }

  .combo-damage {
    font-size: 11px;
    font-variant-numeric: tabular-nums;
    color: var(--text-secondary);
  }

  .combo-label {
    font-size: 11px;
    color: var(--text-secondary);
  }
</style>
