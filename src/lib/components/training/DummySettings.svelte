<script lang="ts">
  /**
   * DummySettings - UI panel for configuring dummy behavior in training mode.
   *
   * This component provides controls for:
   * - Dummy state (stand, crouch, jump, block modes)
   * - Recovery behavior (neutral, reversal)
   * - Reversal move selection
   * - Counter-on-hit toggle
   */

  import type { DummyConfig, DummyState, DummyRecovery } from '$lib/training';

  interface Props {
    /** Current dummy configuration. */
    config: DummyConfig;
    /** Available moves for reversal selection. */
    availableMoves?: string[];
    /** Callback when state changes. */
    onStateChange?: (state: DummyState) => void;
    /** Callback when recovery changes. */
    onRecoveryChange?: (recovery: DummyRecovery) => void;
    /** Callback when reversal move changes. */
    onReversalMoveChange?: (move: string | undefined) => void;
    /** Callback when counter-on-hit changes. */
    onCounterOnHitChange?: (enabled: boolean) => void;
    /** Whether the panel is collapsed. */
    collapsed?: boolean;
    /** Callback to toggle collapsed state. */
    onToggleCollapse?: () => void;
  }

  let {
    config,
    availableMoves = [],
    onStateChange,
    onRecoveryChange,
    onReversalMoveChange,
    onCounterOnHitChange,
    collapsed = false,
    onToggleCollapse,
  }: Props = $props();

  // State options with labels
  const stateOptions: { value: DummyState; label: string }[] = [
    { value: 'stand', label: 'Stand' },
    { value: 'crouch', label: 'Crouch' },
    { value: 'jump', label: 'Jump' },
    { value: 'block_stand', label: 'Block (Stand)' },
    { value: 'block_crouch', label: 'Block (Crouch)' },
    { value: 'block_auto', label: 'Block (Auto)' },
  ];

  // Recovery options with labels
  const recoveryOptions: { value: DummyRecovery; label: string }[] = [
    { value: 'neutral', label: 'Neutral' },
    { value: 'reversal', label: 'Reversal' },
  ];

  function handleStateChange(event: Event) {
    const target = event.target as HTMLSelectElement;
    onStateChange?.(target.value as DummyState);
  }

  function handleRecoveryChange(event: Event) {
    const target = event.target as HTMLSelectElement;
    onRecoveryChange?.(target.value as DummyRecovery);
  }

  function handleReversalMoveChange(event: Event) {
    const target = event.target as HTMLSelectElement;
    const value = target.value === '' ? undefined : target.value;
    onReversalMoveChange?.(value);
  }

  function handleCounterOnHitChange(event: Event) {
    const target = event.target as HTMLInputElement;
    onCounterOnHitChange?.(target.checked);
  }
</script>

<div class="dummy-settings" class:collapsed>
  <button class="header" onclick={onToggleCollapse} type="button">
    <h4>Dummy Settings</h4>
    <span class="toggle-icon">{collapsed ? '+' : '-'}</span>
  </button>

  {#if !collapsed}
    <div class="settings-content">
      <!-- State selection -->
      <div class="setting">
        <label for="dummy-state">State</label>
        <select id="dummy-state" value={config.state} onchange={handleStateChange}>
          {#each stateOptions as option}
            <option value={option.value}>{option.label}</option>
          {/each}
        </select>
      </div>

      <!-- Recovery selection -->
      <div class="setting">
        <label for="dummy-recovery">Recovery</label>
        <select id="dummy-recovery" value={config.recovery} onchange={handleRecoveryChange}>
          {#each recoveryOptions as option}
            <option value={option.value}>{option.label}</option>
          {/each}
        </select>
      </div>

      <!-- Reversal move selection (only shown when recovery is 'reversal') -->
      {#if config.recovery === 'reversal'}
        <div class="setting">
          <label for="reversal-move">Reversal Move</label>
          <select
            id="reversal-move"
            value={config.reversal_move ?? ''}
            onchange={handleReversalMoveChange}
          >
            <option value="">-- Select move --</option>
            {#each availableMoves as move}
              <option value={move}>{move}</option>
            {/each}
          </select>
        </div>
      {/if}

      <!-- Counter on hit toggle -->
      <div class="setting checkbox">
        <label for="counter-on-hit">
          <input
            id="counter-on-hit"
            type="checkbox"
            checked={config.counter_on_hit}
            onchange={handleCounterOnHitChange}
          />
          Counter on hit
        </label>
      </div>
    </div>
  {/if}
</div>

<style>
  .dummy-settings {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 4px;
    overflow: hidden;
  }

  .header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    width: 100%;
    padding: 8px 12px;
    background: transparent;
    border: none;
    cursor: pointer;
    text-align: left;
  }

  .header:hover {
    background: var(--bg-tertiary);
  }

  .header h4 {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    margin: 0;
  }

  .toggle-icon {
    font-size: 14px;
    color: var(--text-secondary);
    font-weight: 600;
  }

  .settings-content {
    padding: 8px 12px 12px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    border-top: 1px solid var(--border);
  }

  .setting {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .setting label {
    font-size: 11px;
    color: var(--text-secondary);
  }

  .setting select {
    font-size: 12px;
    padding: 4px 8px;
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: 3px;
    color: var(--text-primary);
  }

  .setting select:focus {
    outline: none;
    border-color: var(--accent);
  }

  .setting.checkbox {
    flex-direction: row;
    align-items: center;
    gap: 0;
  }

  .setting.checkbox label {
    display: flex;
    align-items: center;
    gap: 6px;
    cursor: pointer;
    user-select: none;
  }

  .setting.checkbox input[type='checkbox'] {
    width: 14px;
    height: 14px;
    cursor: pointer;
  }

  .collapsed .settings-content {
    display: none;
  }
</style>
