<script lang="ts">
  /**
   * TrainingMode - Main training mode view for testing character moves.
   *
   * This view initializes the WASM runtime with character FSPK data and
   * provides a game loop for simulating character interactions.
   */

  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import TrainingViewport, {
    type HitboxData,
    type CollisionBox,
  } from '$lib/components/training/TrainingViewport.svelte';
  import TrainingHUD from '$lib/components/training/TrainingHUD.svelte';
  import InputHistory from '$lib/components/training/InputHistory.svelte';
  import PlaybackControls, { type PlaybackSpeed } from '$lib/components/training/PlaybackControls.svelte';
  import {
    TrainingSession,
    initWasm,
    isWasmReady,
    NO_INPUT,
    type CharacterState,
    type FrameResult,
    type HitResult,
  } from '$lib/training/TrainingSession';
  import {
    InputManager,
    InputBuffer,
    MoveResolver,
    DummyController,
    calculateSimpleFrameAdvantage,
    type TrainingInputConfig,
    type MoveList,
    type MoveDefinition,
    type InputSnapshot,
  } from '$lib/training';
  import { getCurrentCharacter, getRulesRegistry } from '$lib/stores/character.svelte';
  import { getProjectPath } from '$lib/stores/project.svelte';

  // Props
  interface Props {
    onExit?: () => void;
  }

  let { onExit }: Props = $props();

  // State
  let session: TrainingSession | null = $state(null);
  let inputManager: InputManager | null = $state(null);
  let inputBuffer: InputBuffer | null = $state(null);
  let moveResolver: MoveResolver | null = $state(null);
  let dummyController: DummyController | null = $state(null);
  let animationFrameId: number | null = $state(null);
  let isInitializing = $state(true);
  let initError = $state<string | null>(null);

  // Game state
  let frameCount = $state(0);
  let playerState = $state<CharacterState | null>(null);
  let dummyState = $state<CharacterState | null>(null);

  // Character display state
  // TODO: Position updates will come from WASM in a future phase. Currently these
  // are static placeholder values. The WASM runtime will eventually provide position
  // data as part of CharacterState, and these values will be updated each frame.
  let playerX = $state(200);
  let playerY = $state(0);
  let dummyX = $state(600);
  let dummyY = $state(0);

  // Health tracking (separate from WASM state for reset functionality)
  // Note: Player damage is not yet implemented because the dummy cannot attack.
  // Once dummy AI or playback is added, player health will decrease from hits.
  let playerHealth = $state(10000);
  let dummyHealth = $state(10000);
  let maxHealth = $state(10000);

  // Combo tracking
  let comboHits = $state(0);
  let comboDamage = $state(0);
  let comboResetTimer = $state(0);
  const COMBO_RESET_FRAMES = 60; // Reset combo after 1 second of no hits

  // Input history (stores recent snapshots for display)
  let inputHistory = $state<InputSnapshot[]>([]);
  const INPUT_HISTORY_MAX = 30;

  // Playback controls
  let isPlaying = $state(true);
  let playbackSpeed = $state<PlaybackSpeed>(1);
  let frameAccumulator = $state(0);

  // Developer overlay toggles
  let showHitboxes = $state(false);

  // Current character data
  const currentCharacter = $derived(getCurrentCharacter());
  const registry = $derived(getRulesRegistry());

  // Default input configuration
  const defaultInputConfig: TrainingInputConfig = {
    directions: {
      up: 'KeyW',
      down: 'KeyS',
      left: 'KeyA',
      right: 'KeyD',
    },
    buttons: {
      L: 'KeyU',
      M: 'KeyI',
      H: 'KeyO',
      P: 'KeyJ',
      K: 'KeyK',
      S: 'KeyL',
    },
  };

  // Build move list from character moves
  function buildMoveList(): MoveList {
    const moves: MoveDefinition[] = [];
    const moveNameToIndex = new Map<string, number>();

    if (!currentCharacter?.moves) {
      return { moves, moveNameToIndex };
    }

    // Sort moves by type priority (specials/supers first, then normals)
    const sortedMoves = [...currentCharacter.moves].sort((a, b) => {
      const priorityA = getMoveTypePriority(a.type);
      const priorityB = getMoveTypePriority(b.type);
      return priorityB - priorityA;
    });

    for (const move of sortedMoves) {
      const index = moves.length;
      const parsed = parseInputNotation(move.input);
      if (parsed) {
        moves.push({
          name: move.input,
          input: parsed,
          priority: getMoveTypePriority(move.type),
        });
        moveNameToIndex.set(move.input, index);
      }
    }

    return { moves, moveNameToIndex };
  }

  // Get priority for a move type
  function getMoveTypePriority(type: string | undefined): number {
    switch (type) {
      case 'super':
        return 100;
      case 'ex':
        return 90;
      case 'special':
        return 80;
      case 'rekka':
        return 70;
      case 'command_normal':
        return 60;
      case 'normal':
      default:
        return 50;
    }
  }

  // Parse input notation to MoveInput
  function parseInputNotation(input: string): MoveDefinition['input'] | null {
    // Simple patterns: 5L, 2M, 6H, etc.
    const simpleMatch = input.match(/^([1-9])([LMHPKS])$/);
    if (simpleMatch) {
      return {
        type: 'simple',
        direction: parseInt(simpleMatch[1]),
        button: simpleMatch[2] as any,
      };
    }

    // Motion patterns: 236P, 214K, 623H, etc.
    const motionMatch = input.match(/^(\d{3,})([LMHPKS])$/);
    if (motionMatch) {
      const sequence = motionMatch[1].split('').map(d => parseInt(d));
      return {
        type: 'motion',
        sequence,
        button: motionMatch[2] as any,
      };
    }

    return null;
  }

  // Initialize training mode
  async function initialize() {
    isInitializing = true;
    initError = null;

    try {
      // Initialize WASM module
      await initWasm();

      // Get FSPK data for current character
      const projectPath = getProjectPath();
      if (!projectPath) {
        throw new Error('No project open');
      }
      if (!currentCharacter) {
        throw new Error('No character selected');
      }

      const charactersDir = `${projectPath}/characters`;
      const characterId = currentCharacter.character.id;

      // Get FSPK bytes from Tauri
      const fspkBase64 = await invoke<string>('get_character_fspk', {
        charactersDir,
        characterId,
      });

      // Decode base64 to Uint8Array
      const binaryString = atob(fspkBase64);
      const fspkBytes = new Uint8Array(binaryString.length);
      for (let i = 0; i < binaryString.length; i++) {
        fspkBytes[i] = binaryString.charCodeAt(i);
      }

      // Create training session (using same character for player and dummy)
      session = await TrainingSession.create(fspkBytes, fspkBytes);

      // Initialize input system
      inputManager = new InputManager(defaultInputConfig);
      inputBuffer = new InputBuffer();
      moveResolver = new MoveResolver(buildMoveList());
      dummyController = new DummyController();

      // Set initial health from character data
      maxHealth = currentCharacter.character.health;
      playerHealth = maxHealth;
      dummyHealth = maxHealth;

      // Get initial state
      playerState = session.playerState();
      dummyState = session.dummyState();

      // Start game loop
      startGameLoop();

      isInitializing = false;
    } catch (e) {
      console.error('Failed to initialize training mode:', e);
      initError = e instanceof Error ? e.message : String(e);
      isInitializing = false;
    }
  }

  // Game loop
  function startGameLoop() {
    let lastTime = performance.now();

    function gameLoop(currentTime: number) {
      if (!session || !inputManager || !inputBuffer || !moveResolver || !dummyController) {
        return;
      }

      // Handle playback speed
      if (!isPlaying) {
        // Still schedule next frame but don't tick
        animationFrameId = requestAnimationFrame(gameLoop);
        lastTime = currentTime;
        return;
      }

      // Speed control (0 = frame-by-frame, handled separately)
      if (playbackSpeed === 0) {
        animationFrameId = requestAnimationFrame(gameLoop);
        lastTime = currentTime;
        return;
      }

      // Accumulate time for sub-speed playback
      const deltaTime = currentTime - lastTime;
      lastTime = currentTime;

      // Calculate how many frames to run based on speed
      // At 60fps, one frame is ~16.67ms
      const frameTime = 16.67;
      frameAccumulator += deltaTime * playbackSpeed;

      if (frameAccumulator < frameTime) {
        animationFrameId = requestAnimationFrame(gameLoop);
        return;
      }

      // Run one frame (don't accumulate multiple to keep it smooth)
      frameAccumulator = Math.min(frameAccumulator - frameTime, frameTime);

      tickOneFrame();

      // Schedule next frame
      animationFrameId = requestAnimationFrame(gameLoop);
    }

    animationFrameId = requestAnimationFrame(gameLoop);
  }

  // Tick one frame of simulation
  function tickOneFrame() {
    if (!session || !inputManager || !inputBuffer || !moveResolver || !dummyController) {
      return;
    }

    // Get input snapshot and add to buffer
    const snapshot = inputManager.getSnapshot();
    inputBuffer.push(snapshot);

    // Store input in history for display
    inputHistory = [...inputHistory.slice(-(INPUT_HISTORY_MAX - 1)), snapshot];

    // Resolve move from input
    const newlyPressed = inputManager.newlyPressedButtons;
    const resolved = moveResolver.resolve(inputBuffer, newlyPressed, session.availableCancels());
    inputManager.consumeNewlyPressed();

    // Get move index or NO_INPUT
    const playerInput = resolved ? resolved.index : NO_INPUT;

    // Get dummy state
    const wasmDummyState = dummyController.getWasmState();

    // Tick simulation with error handling for WASM errors
    let result: FrameResult;
    try {
      result = session.tick(playerInput, wasmDummyState);
    } catch (e) {
      console.error('WASM tick error:', e);
      // Stop the game loop on WASM error to prevent spam
      stopGameLoop();
      initError = `WASM error: ${e instanceof Error ? e.message : String(e)}`;
      return;
    }

    // Update state
    playerState = result.player;
    dummyState = result.dummy;

    // Process hits and track combos
    const hits = result.hits;
    if (hits.length > 0) {
      for (const hit of hits) {
        // Apply damage to dummy (player attacking)
        dummyHealth = Math.max(0, dummyHealth - hit.damage);
        // Track combo
        comboHits++;
        comboDamage += hit.damage;
      }
      // Reset combo timer on hit
      comboResetTimer = COMBO_RESET_FRAMES;
    } else {
      // Decrease combo timer
      if (comboResetTimer > 0) {
        comboResetTimer--;
        if (comboResetTimer === 0) {
          // Reset combo
          comboHits = 0;
          comboDamage = 0;
        }
      }
    }

    frameCount++;
  }

  // Stop game loop
  function stopGameLoop() {
    if (animationFrameId !== null) {
      cancelAnimationFrame(animationFrameId);
      animationFrameId = null;
    }
  }

  // Handle keyboard events
  function handleKeyDown(event: KeyboardEvent) {
    if (!inputManager) return;

    // Handle special keys
    if (event.code === 'Escape') {
      onExit?.();
      return;
    }
    if (event.code === 'KeyR') {
      resetHealth();
      return;
    }
    // Playback controls
    if (event.code === 'Space') {
      event.preventDefault();
      togglePlayPause();
      return;
    }
    if (event.code === 'Period') {
      // Step forward
      stepForward();
      return;
    }
    if (event.code === 'Comma') {
      // Step back (not implemented - would need state history)
      return;
    }
    // Hitbox toggle
    if (event.code === 'KeyH') {
      showHitboxes = !showHitboxes;
      return;
    }

    inputManager.handleKeyDown(event.code);
  }

  // Playback control functions
  function togglePlayPause() {
    isPlaying = !isPlaying;
  }

  function stepForward() {
    if (isPlaying) {
      isPlaying = false;
    }
    tickOneFrame();
  }

  function stepBack() {
    // Step back requires state history/rollback - not implemented yet
    // This would be a feature for Phase 6 (sequence recorder)
  }

  function setPlaybackSpeed(speed: PlaybackSpeed) {
    playbackSpeed = speed;
  }

  function handleKeyUp(event: KeyboardEvent) {
    if (!inputManager) return;
    inputManager.handleKeyUp(event.code);
  }

  // Reset health
  function resetHealth() {
    playerHealth = maxHealth;
    dummyHealth = maxHealth;
    session?.reset();
  }

  // Cleanup
  function cleanup() {
    stopGameLoop();
    inputManager?.reset();
    session?.free();
    session = null;
  }

  // Lifecycle
  onMount(() => {
    initialize();
    window.addEventListener('keydown', handleKeyDown);
    window.addEventListener('keyup', handleKeyUp);
  });

  onDestroy(() => {
    cleanup();
    window.removeEventListener('keydown', handleKeyDown);
    window.removeEventListener('keyup', handleKeyUp);
  });

  // Derived display data
  const playerDisplay = $derived.by(() => ({
    x: playerX,
    y: playerY,
    width: 60,
    height: 120,
    facingRight: true,
    currentMove: playerState && moveResolver
      ? moveResolver.getMove(playerState.current_move)?.name
      : undefined,
  }));

  const dummyDisplay = $derived.by(() => ({
    x: dummyX,
    y: dummyY,
    width: 60,
    height: 120,
    facingRight: false,
    currentMove: dummyState && moveResolver
      ? moveResolver.getMove(dummyState.current_move)?.name
      : undefined,
  }));

  const playerStatus = $derived.by(() => ({
    health: playerHealth,
    maxHealth: maxHealth,
    resources: playerState?.resources
      ? playerState.resources.slice(0, 2).map((v, i) => ({
          name: i === 0 ? 'Meter' : 'Heat',
          value: v,
          max: 100,
        }))
      : [],
  }));

  const dummyStatusState = $derived.by(() => ({
    health: dummyHealth,
    maxHealth: maxHealth,
    resources: dummyState?.resources
      ? dummyState.resources.slice(0, 2).map((v, i) => ({
          name: i === 0 ? 'Meter' : 'Heat',
          value: v,
          max: 100,
        }))
      : [],
  }));

  const currentMoveInfo = $derived.by(() => {
    if (!playerState || !currentCharacter || !moveResolver) return null;
    const moveDef = moveResolver.getMove(playerState.current_move);
    if (!moveDef) return null;

    const move = currentCharacter.moves.find(m => m.input === moveDef.name);
    if (!move) return null;

    // Calculate frame advantage
    const totalFrames = move.startup + move.active + move.recovery;

    // Get hitstun/blockstun from move data
    // First try v2 hits array, then fall back to legacy fields
    const hitData = move.hits?.[0];
    const hitstun = hitData?.hitstun ?? move.hitstun ?? 15;
    const blockstun = hitData?.blockstun ?? move.blockstun ?? 10;

    const advantage = calculateSimpleFrameAdvantage(move.recovery, hitstun, blockstun);

    return {
      name: move.input,
      startup: move.startup,
      active: move.active,
      recovery: move.recovery,
      currentFrame: playerState.frame,
      totalFrames,
      advantageOnHit: advantage.onHit,
      advantageOnBlock: advantage.onBlock,
    };
  });

  // Available cancels as move names
  const availableCancelNames = $derived.by(() => {
    if (!session || !moveResolver) return [];
    const cancelIndices = session.availableCancels();
    const resolver = moveResolver; // Capture in local variable for TypeScript
    return cancelIndices
      .map(idx => resolver.getMove(idx)?.name)
      .filter((name): name is string => name !== undefined);
  });

  // Combo info
  const comboInfo = $derived.by(() => ({
    hitCount: comboHits,
    totalDamage: comboDamage,
  }));

  const dummyStatusDisplay = $derived.by(() => ({
    stateLabel: dummyController?.config.state
      ? formatDummyState(dummyController.config.state)
      : 'Standing',
  }));

  function formatDummyState(state: string): string {
    switch (state) {
      case 'stand':
        return 'Standing';
      case 'crouch':
        return 'Crouching';
      case 'jump':
        return 'Jumping';
      case 'block_stand':
        return 'Block (Stand)';
      case 'block_crouch':
        return 'Block (Crouch)';
      case 'block_auto':
        return 'Block (Auto)';
      default:
        return state;
    }
  }
</script>

<div class="training-mode">
  {#if isInitializing}
    <div class="loading">
      <p>Initializing training mode...</p>
    </div>
  {:else if initError}
    <div class="error">
      <h3>Failed to initialize training mode</h3>
      <p>{initError}</p>
      <button onclick={() => onExit?.()}>Back</button>
    </div>
  {:else}
    <!-- HUD at top -->
    <TrainingHUD
      player={playerStatus}
      dummy={dummyStatusState}
      currentMove={currentMoveInfo}
      dummyStatus={dummyStatusDisplay}
      availableCancels={availableCancelNames}
      comboInfo={comboInfo}
      onResetHealth={resetHealth}
    />

    <!-- Main content area with viewport and sidebar -->
    <div class="main-content">
      <!-- Viewport -->
      <div class="viewport-container">
        <TrainingViewport
          player={playerDisplay}
          dummy={dummyDisplay}
          viewportWidth={800}
          viewportHeight={400}
          showHitboxes={showHitboxes}
        />
      </div>

      <!-- Right sidebar with input history -->
      <div class="sidebar">
        <InputHistory inputs={inputHistory} maxDisplay={15} />
      </div>
    </div>

    <!-- Bottom bar with playback controls and keyboard help -->
    <div class="bottom-bar">
      <PlaybackControls
        isPlaying={isPlaying}
        speed={playbackSpeed}
        onPlayPause={togglePlayPause}
        onStepBack={stepBack}
        onStepForward={stepForward}
        onSpeedChange={setPlaybackSpeed}
      />

      <!-- Controls info -->
      <div class="controls-info">
        <div class="control-group">
          <span class="control-label">Movement:</span>
          <span class="control-keys">WASD</span>
        </div>
        <div class="control-group">
          <span class="control-label">Attacks:</span>
          <span class="control-keys">U I O / J K L</span>
        </div>
        <div class="control-group">
          <span class="control-label">Hitboxes:</span>
          <span class="control-keys">H</span>
        </div>
        <div class="control-group">
          <span class="control-label">Reset:</span>
          <span class="control-keys">R</span>
        </div>
        <div class="control-group">
          <span class="control-label">Exit:</span>
          <span class="control-keys">Esc</span>
        </div>
      </div>
    </div>

    <!-- Dummy settings panel -->
    <div class="dummy-panel">
      <h4>Dummy Settings</h4>
      <div class="dummy-setting">
        <label for="dummy-state">State:</label>
        <select
          id="dummy-state"
          onchange={(e) => dummyController?.setState(e.currentTarget.value as any)}
        >
          <option value="stand">Stand</option>
          <option value="crouch">Crouch</option>
          <option value="jump">Jump</option>
          <option value="block_stand">Block (Stand)</option>
          <option value="block_crouch">Block (Crouch)</option>
          <option value="block_auto">Block (Auto)</option>
        </select>
      </div>
      <div class="dummy-setting">
        <label for="dummy-recovery">Recovery:</label>
        <select
          id="dummy-recovery"
          onchange={(e) => dummyController?.setRecovery(e.currentTarget.value as any)}
        >
          <option value="neutral">Neutral</option>
          <option value="reversal">Reversal</option>
        </select>
      </div>
    </div>

    <!-- Frame counter (debug) -->
    <div class="frame-counter">
      Frame: {frameCount}
    </div>
  {/if}
</div>

<style>
  .training-mode {
    display: flex;
    flex-direction: column;
    height: 100%;
    gap: 8px;
    padding: 8px;
  }

  .loading,
  .error {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    gap: 16px;
  }

  .error h3 {
    color: var(--accent);
  }

  .error p {
    color: var(--text-secondary);
    max-width: 400px;
    text-align: center;
  }

  .main-content {
    display: flex;
    gap: 12px;
    flex: 1;
    min-height: 0;
  }

  .viewport-container {
    display: flex;
    justify-content: center;
    flex: 1;
    min-height: 0;
  }

  .sidebar {
    display: flex;
    flex-direction: column;
    gap: 8px;
    width: 140px;
  }

  .bottom-bar {
    display: flex;
    align-items: center;
    gap: 16px;
  }

  .controls-info {
    display: flex;
    flex-wrap: wrap;
    gap: 16px;
    padding: 6px 12px;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 4px;
    flex: 1;
  }

  .control-group {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .control-label {
    font-size: 11px;
    color: var(--text-secondary);
  }

  .control-keys {
    font-size: 10px;
    font-family: monospace;
    padding: 2px 4px;
    background: var(--bg-tertiary);
    border-radius: 3px;
    color: var(--text-primary);
  }

  .dummy-panel {
    position: absolute;
    top: 60px;
    right: 160px;
    padding: 12px;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 4px;
    width: 160px;
    z-index: 10;
  }

  .dummy-panel h4 {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-secondary);
    margin-bottom: 8px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .dummy-setting {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-bottom: 8px;
  }

  .dummy-setting:last-child {
    margin-bottom: 0;
  }

  .dummy-setting label {
    font-size: 11px;
    color: var(--text-secondary);
  }

  .dummy-setting select {
    font-size: 12px;
  }

  .frame-counter {
    position: absolute;
    bottom: 50px;
    right: 160px;
    font-size: 10px;
    font-variant-numeric: tabular-nums;
    color: var(--text-secondary);
    padding: 2px 6px;
    background: var(--bg-tertiary);
    border-radius: 3px;
  }
</style>
