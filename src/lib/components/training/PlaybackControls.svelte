<script lang="ts">
  /**
   * PlaybackControls - Play/pause, step, and speed controls for training mode.
   *
   * Provides controls for:
   * - Play/pause the simulation
   * - Step forward/backward one frame
   * - Adjust playback speed (1x, 0.5x, 0.25x, frame-by-frame)
   */

  /** Available playback speeds. */
  export type PlaybackSpeed = 1 | 0.5 | 0.25 | 0;

  interface Props {
    /** Whether the simulation is currently playing. */
    isPlaying: boolean;
    /** Current playback speed multiplier. */
    speed: PlaybackSpeed;
    /** Callback when play/pause is toggled. */
    onPlayPause?: () => void;
    /** Callback to step back one frame. */
    onStepBack?: () => void;
    /** Callback to step forward one frame. */
    onStepForward?: () => void;
    /** Callback when speed changes. */
    onSpeedChange?: (speed: PlaybackSpeed) => void;
  }

  let {
    isPlaying,
    speed,
    onPlayPause,
    onStepBack,
    onStepForward,
    onSpeedChange,
  }: Props = $props();

  const speedOptions: { value: PlaybackSpeed; label: string }[] = [
    { value: 1, label: '1x' },
    { value: 0.5, label: '0.5x' },
    { value: 0.25, label: '0.25x' },
    { value: 0, label: 'Frame' },
  ];

  function handleSpeedChange(event: Event) {
    const target = event.target as HTMLSelectElement;
    const newSpeed = parseFloat(target.value) as PlaybackSpeed;
    onSpeedChange?.(newSpeed);
  }
</script>

<div class="playback-controls">
  <!-- Step back button -->
  <button
    class="control-btn"
    onclick={onStepBack}
    disabled={isPlaying}
    title="Step back (,)"
  >
    <span class="icon">{'\u23EE'}</span>
  </button>

  <!-- Play/Pause button -->
  <button
    class="control-btn play-pause"
    onclick={onPlayPause}
    title={isPlaying ? 'Pause (Space)' : 'Play (Space)'}
  >
    <span class="icon">{isPlaying ? '\u23F8' : '\u25B6'}</span>
  </button>

  <!-- Step forward button -->
  <button
    class="control-btn"
    onclick={onStepForward}
    disabled={isPlaying}
    title="Step forward (.)"
  >
    <span class="icon">{'\u23ED'}</span>
  </button>

  <!-- Speed selector -->
  <div class="speed-control">
    <label for="speed-select">Speed:</label>
    <select
      id="speed-select"
      value={speed}
      onchange={handleSpeedChange}
    >
      {#each speedOptions as option}
        <option value={option.value}>{option.label}</option>
      {/each}
    </select>
  </div>
</div>

<style>
  .playback-controls {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 8px;
    background: var(--bg-tertiary);
    border-radius: 4px;
  }

  .control-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    padding: 0;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--bg-secondary);
    color: var(--text-primary);
    cursor: pointer;
    transition: background 0.1s, border-color 0.1s;
  }

  .control-btn:hover:not(:disabled) {
    background: var(--bg-primary);
    border-color: var(--accent);
  }

  .control-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .control-btn.play-pause {
    width: 32px;
    height: 32px;
  }

  .icon {
    font-size: 14px;
  }

  .play-pause .icon {
    font-size: 16px;
  }

  .speed-control {
    display: flex;
    align-items: center;
    gap: 4px;
    margin-left: 8px;
  }

  .speed-control label {
    font-size: 11px;
    color: var(--text-secondary);
  }

  .speed-control select {
    font-size: 11px;
    padding: 2px 4px;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 3px;
    color: var(--text-primary);
  }
</style>
