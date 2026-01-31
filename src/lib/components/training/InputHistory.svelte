<script lang="ts">
  /**
   * InputHistory - Displays a scrolling list of recent inputs.
   *
   * Shows direction arrows and button names for the last N inputs,
   * useful for visualizing input timing and execution.
   */

  import type { InputSnapshot, ButtonName } from '$lib/training';

  interface Props {
    /** Recent input snapshots to display (newest first). */
    inputs: InputSnapshot[];
    /** Maximum number of inputs to show. */
    maxDisplay?: number;
  }

  let { inputs, maxDisplay = 10 }: Props = $props();

  // Get display inputs (newest first, limited to maxDisplay)
  const displayInputs = $derived(inputs.slice(-maxDisplay).reverse());

  /**
   * Convert numpad direction to arrow character.
   */
  function directionToArrow(direction: number): string {
    switch (direction) {
      case 1:
        return '\u2199'; // down-left
      case 2:
        return '\u2193'; // down
      case 3:
        return '\u2198'; // down-right
      case 4:
        return '\u2190'; // left
      case 5:
        return '\u25CF'; // neutral (dot)
      case 6:
        return '\u2192'; // right
      case 7:
        return '\u2196'; // up-left
      case 8:
        return '\u2191'; // up
      case 9:
        return '\u2197'; // up-right
      default:
        return '?';
    }
  }

  /**
   * Format buttons for display.
   */
  function formatButtons(buttons: ButtonName[]): string {
    return buttons.length > 0 ? buttons.join('+') : '';
  }

  /**
   * Check if an input has any active buttons or non-neutral direction.
   */
  function isActiveInput(snapshot: InputSnapshot): boolean {
    return snapshot.buttons.length > 0 || snapshot.direction !== 5;
  }
</script>

<div class="input-history">
  <div class="history-header">Input History</div>
  <div class="history-list">
    {#each displayInputs as input, i}
      <div class="input-entry" class:active={isActiveInput(input)} class:newest={i === 0}>
        <span class="direction">{directionToArrow(input.direction)}</span>
        {#if input.buttons.length > 0}
          <span class="buttons">{formatButtons(input.buttons)}</span>
        {/if}
      </div>
    {/each}
    {#if displayInputs.length === 0}
      <div class="empty">No inputs</div>
    {/if}
  </div>
</div>

<style>
  .input-history {
    display: flex;
    flex-direction: column;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 4px;
    overflow: hidden;
    min-width: 100px;
  }

  .history-header {
    font-size: 10px;
    font-weight: 600;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    padding: 4px 8px;
    background: var(--bg-tertiary);
    border-bottom: 1px solid var(--border);
  }

  .history-list {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 4px;
    max-height: 200px;
    overflow-y: auto;
  }

  .input-entry {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 2px 4px;
    font-size: 12px;
    font-family: monospace;
    color: var(--text-secondary);
    border-radius: 2px;
  }

  .input-entry.active {
    color: var(--text-primary);
    background: var(--bg-tertiary);
  }

  .input-entry.newest {
    color: var(--accent);
    font-weight: 600;
  }

  .direction {
    font-size: 14px;
    min-width: 20px;
    text-align: center;
  }

  .buttons {
    font-size: 11px;
    color: var(--warning);
  }

  .empty {
    font-size: 11px;
    color: var(--text-secondary);
    font-style: italic;
    padding: 8px;
    text-align: center;
  }
</style>
