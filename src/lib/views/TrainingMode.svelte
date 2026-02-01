<script lang="ts">
  /**
   * TrainingMode - Main training mode view for testing character moves.
   *
   * This view initializes the WASM runtime with character FSPK data and
   * provides a game loop for simulating character interactions.
   */

  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import TrainingViewport from '$lib/components/training/TrainingViewport.svelte';
  import TrainingHUD from '$lib/components/training/TrainingHUD.svelte';
  import InputHistory from '$lib/components/training/InputHistory.svelte';
  import PlaybackControls, { type PlaybackSpeed } from '$lib/components/training/PlaybackControls.svelte';
  import DummySettings from '$lib/components/training/DummySettings.svelte';
  import HitboxOverlay from '$lib/components/training/HitboxOverlay.svelte';
  import {
    TrainingSession,
    initWasm,
    NO_INPUT,
    type CharacterState,
    type FrameResult,
  } from '$lib/training/TrainingSession';
  import {
    InputManager,
    InputBuffer,
    MoveResolver,
    DummyController,
    calculateSimpleFrameAdvantage,
    type TrainingInputConfig,
    type InputSnapshot,
  } from '$lib/training';
  import { pickAnimationKey } from '$lib/training/pickAnimationKey';
  import { buildMoveList } from '$lib/training/buildMoveList';
  import { getCurrentCharacter, getTrainingSync } from '$lib/stores/character.svelte';
  import { getProjectPath } from '$lib/stores/project.svelte';
  import type { Character, CharacterAssets } from '$lib/types';
  import type { ActorSpec, Facing } from '$lib/rendercore/types';
  import { buildActorSpecForMoveAnimation, getMoveForStateIndex } from '$lib/training/renderMapping';
  import { getCharProp } from '$lib/utils';

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

  let initSeq = 0;
  let destroyed = false;

  // Rendering assets (loaded via Tauri)
  let renderAssets = $state<CharacterAssets | null>(null);
  let renderAssetsLoading = $state(false);
  let renderAssetsError = $state<string | null>(null);
  let renderAssetsSeq = 0;

  // Game state
  let frameCount = $state(0);
  let playerState = $state<CharacterState | null>(null);
  let dummyState = $state<CharacterState | null>(null);

  // Character display state
  // Positions are in screen pixels. Characters need to be close enough for hitboxes to collide.
  // Typical hitbox reach is ~30-50 pixels, so characters should be ~100 pixels apart to test hits.
  let playerX = $state(350);
  let playerY = $state(0);
  let dummyX = $state(450);
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
  let dummySettingsCollapsed = $state(false);

  // Current character data
  const currentCharacter = $derived(getCurrentCharacter());

  const charactersDir = $derived.by((): string | null => {
    const projectPath = getProjectPath();
    if (!projectPath) return null;
    return `${projectPath}/characters`;
  });

  const characterId = $derived.by((): string | null => {
    return currentCharacter?.character.id ?? null;
  });

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

  // NOTE: Move list building lives in $lib/training/buildMoveList to keep
  // canonical indices aligned with the backend/exporter.

  function formatError(e: unknown): string {
    if (e instanceof Error) return e.message;
    if (typeof e === 'string') return e;
    try {
      return JSON.stringify(e);
    } catch {
      return String(e);
    }
  }

  $effect(() => {
    const dir = charactersDir;
    const id = characterId;

    if (!dir) {
      renderAssets = null;
      renderAssetsLoading = false;
      renderAssetsError = 'No project open';
      return;
    }
    if (!id) {
      renderAssets = null;
      renderAssetsLoading = false;
      renderAssetsError = 'No character selected';
      return;
    }

    const seq = ++renderAssetsSeq;
    renderAssetsLoading = true;
    renderAssetsError = null;
    void invoke<CharacterAssets>('load_character_assets', {
      charactersDir: dir,
      characterId: id,
    })
      .then((next) => {
        if (seq !== renderAssetsSeq) return;
        renderAssets = next;
      })
      .catch((e) => {
        if (seq !== renderAssetsSeq) return;
        renderAssets = null;
        renderAssetsError = formatError(e);
      })
      .finally(() => {
        if (seq === renderAssetsSeq) renderAssetsLoading = false;
      });
  });


  // Initialize training mode
  async function initialize() {
    const seq = ++initSeq;
    isInitializing = true;
    initError = null;

    cleanup();

    try {
      // Initialize WASM module
      await initWasm();

      if (destroyed || seq !== initSeq) return;

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

      if (destroyed || seq !== initSeq) return;

      // Decode base64 to Uint8Array
      const binaryString = atob(fspkBase64);
      const fspkBytes = new Uint8Array(binaryString.length);
      for (let i = 0; i < binaryString.length; i++) {
        fspkBytes[i] = binaryString.charCodeAt(i);
      }

      // Create training session (using same character for player and dummy)
      const nextSession = await TrainingSession.create(fspkBytes, fspkBytes);

      if (destroyed || seq !== initSeq) {
        nextSession.free();
        return;
      }

      // Initialize input system
      const nextInputManager = new InputManager(defaultInputConfig);
      const nextInputBuffer = new InputBuffer();
      const nextMoveResolver = new MoveResolver(buildMoveList(currentCharacter?.moves));
      const nextDummyController = new DummyController();

      if (destroyed || seq !== initSeq) {
        nextInputManager.reset();
        nextSession.free();
        return;
      }

      session = nextSession;
      inputManager = nextInputManager;
      inputBuffer = nextInputBuffer;
      moveResolver = nextMoveResolver;
      dummyController = nextDummyController;

      // Set initial health from character data
      maxHealth = getCharProp(currentCharacter.character, 'health', 1000);
      playerHealth = maxHealth;
      dummyHealth = maxHealth;

      // Get initial state
      playerState = session.playerState();
      dummyState = session.dummyState();

      // Start game loop
      startGameLoop(seq);

      // Initialize training sync for detached windows
      getTrainingSync();

      isInitializing = false;
    } catch (e) {
      console.error('Failed to initialize training mode:', e);
      if (destroyed || seq !== initSeq) return;
      initError = e instanceof Error ? e.message : String(e);
      isInitializing = false;
    }
  }

  // Open detached training window
  async function openDetachedWindow() {
    if (!currentCharacter) return;

    try {
      await invoke('open_training_window', {
        characterId: currentCharacter.character.id,
      });
    } catch (e) {
      console.error('Failed to open detached window:', e);
      initError = e instanceof Error ? e.message : String(e);
    }
  }

  // Game loop
  function startGameLoop(loopSeq: number) {
    stopGameLoop();
    let lastTime = performance.now();

    function gameLoop(currentTime: number) {
      if (destroyed || loopSeq !== initSeq) return;
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

    // Sync positions to WASM for hit detection
    session.setPositions(playerX, playerY, dummyX, dummyY);

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
    const prevState = playerState?.current_state;
    playerState = result.player;
    dummyState = result.dummy;

    // Log state transitions for debugging
    if (prevState !== result.player.current_state) {
      const move = currentCharacter?.moves[result.player.current_state];
      console.log('[STATE]', {
        from: prevState,
        to: result.player.current_state,
        moveName: move?.input ?? 'unknown',
        hitboxes: move?.hitboxes?.length ?? 0,
      });
    }

    // Apply movement
    applyMovement(snapshot, result.player);

    // Process hits and track combos
    const hits = result.hits;
    if (hits.length > 0) {
      console.log('[HIT]', {
        playerPos: { x: playerX, y: playerY },
        dummyPos: { x: dummyX, y: dummyY },
        playerState: result.player.current_state,
        playerFrame: result.player.frame,
        hits: hits.map(h => ({ damage: h.damage, move: h.attacker_move })),
      });
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

    // Apply push separation when characters' pushboxes overlap
    if (result.push_separation) {
      playerX += result.push_separation.player_dx;
      dummyX += result.push_separation.dummy_dx;
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

  // Apply movement based on current state and input
  function applyMovement(snapshot: InputSnapshot, state: CharacterState) {
    if (!currentCharacter) return;

    const char = currentCharacter.character;
    const move = currentCharacter.moves[state.current_state];

    // Stage boundaries (prevent going off screen)
    const MIN_X = 50;
    const MAX_X = 750;

    // Check if in a movement state with movement data
    if (move?.movement) {
      const movement = move.movement;
      const totalFrames = move.total ?? (move.startup + move.active + move.recovery);

      if (movement.distance && movement.direction) {
        // Calculate per-frame movement
        const perFrame = movement.distance / totalFrames;
        const direction = movement.direction === 'forward' ? 1 : -1;

        // Apply movement (player faces right)
        playerX = Math.max(MIN_X, Math.min(MAX_X, playerX + perFrame * direction));
      }
    }
    // Walking: apply when in system state (idle/crouch) and holding direction
    else if (state.current_state <= 1) {
      // Direction 4 = back (left), 6 = forward (right)
      // Also handle diagonals: 1, 4, 7 = back; 3, 6, 9 = forward
      const isHoldingBack = [1, 4, 7].includes(snapshot.direction);
      const isHoldingForward = [3, 6, 9].includes(snapshot.direction);

      if (isHoldingBack) {
        const backWalkSpeed = getCharProp(char, 'back_walk_speed', 3.2);
        playerX = Math.max(MIN_X, playerX - backWalkSpeed);
      } else if (isHoldingForward) {
        const walkSpeed = getCharProp(char, 'walk_speed', 4.5);
        playerX = Math.min(MAX_X, playerX + walkSpeed);
      }
    }
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
    inputManager = null;
    inputBuffer = null;
    moveResolver = null;
    dummyController = null;
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
    destroyed = true;
    initSeq++;
    renderAssetsSeq++;
    renderAssetsLoading = false;
    cleanup();
    window.removeEventListener('keydown', handleKeyDown);
    window.removeEventListener('keyup', handleKeyUp);
  });

  const renderBuild = $derived.by((): { actors: ActorSpec[]; error: string | null } => {
    const errs: string[] = [];
    const specs: ActorSpec[] = [];

    const assets = renderAssets;
    const char = currentCharacter;

    if (!assets) {
      if (renderAssetsLoading) errs.push('Loading assets...');
      else if (renderAssetsError) errs.push(`Assets error: ${renderAssetsError}`);
    }

    if (!assets || !char || !playerState || !dummyState) {
      return { actors: specs, error: errs.length ? errs.join('\n') : null };
    }

    const resolveOne = (actorId: string, state: CharacterState, pos: { x: number; y: number }, facing: Facing) => {
      const move = getMoveForStateIndex(char.moves, state.current_state);
      if (!move) {
        errs.push(`${actorId}: State index out of bounds: ${state.current_state}`);
      }

      const picked = pickAnimationKey(assets, move?.animation ?? '');
      if (picked.note) errs.push(`${actorId}: ${picked.note}`);
      if (!picked.key) return;

      const built = buildActorSpecForMoveAnimation({
        assets,
        animationKey: picked.key,
        actorId,
        pos,
        facing,
        frameIndex: state.frame,
      });
      if (built.error) errs.push(`${actorId}: ${built.error}`);
      if (built.spec) specs.push(built.spec);
    };

    resolveOne('p1', playerState, { x: playerX, y: playerY }, 'right');
    resolveOne('cpu', dummyState, { x: dummyX, y: dummyY }, 'left');

    return { actors: specs, error: errs.length ? errs.join('\n') : null };
  });

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
    const moveDef = moveResolver.getMove(playerState.current_state);
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

  // Hitbox overlay data
  const playerHitboxData = $derived.by(() => {
    const move = playerState && currentCharacter
      ? getMoveForStateIndex(currentCharacter.moves, playerState.current_state)
      : null;
    return {
      move,
      frame: playerState?.frame ?? 0,
      x: playerX,
      y: playerY,
      facing: 'right' as const,
    };
  });

  const dummyHitboxData = $derived.by(() => {
    const move = dummyState && currentCharacter
      ? getMoveForStateIndex(currentCharacter.moves, dummyState.current_state)
      : null;
    return {
      move,
      frame: dummyState?.frame ?? 0,
      x: dummyX,
      y: dummyY,
      facing: 'left' as const,
    };
  });

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
        <div class="viewport-frame">
          <TrainingViewport
            charactersDir={charactersDir ?? ''}
            characterId={characterId ?? ''}
            actors={renderBuild.actors}
            error={renderBuild.error}
          />
          <HitboxOverlay
            player={playerHitboxData}
            dummy={dummyHitboxData}
            show={showHitboxes}
            width={800}
            height={400}
          />
        </div>
        </div>

      <!-- Right sidebar with input history and dummy settings -->
      <div class="sidebar">
        <InputHistory inputs={inputHistory} maxDisplay={15} />
        {#if dummyController}
          <DummySettings
            config={dummyController.config}
            availableMoves={currentCharacter?.moves.map(m => m.input) ?? []}
            onStateChange={(state) => dummyController?.setState(state)}
            onRecoveryChange={(recovery) => dummyController?.setRecovery(recovery)}
            onReversalMoveChange={(move) => dummyController?.setReversalMove(move)}
            onCounterOnHitChange={(enabled) => dummyController?.setCounterOnHit(enabled)}
            collapsed={dummySettingsCollapsed}
            onToggleCollapse={() => dummySettingsCollapsed = !dummySettingsCollapsed}
          />
        {/if}
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

      <!-- Detach button -->
      <button class="detach-btn" onclick={openDetachedWindow} title="Open in new window">
        Detach
      </button>

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

  .viewport-frame {
    position: relative;
    width: 800px;
    height: 400px;
    min-width: 0;
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

  .detach-btn {
    padding: 6px 12px;
    font-size: 11px;
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--text-primary);
    cursor: pointer;
    white-space: nowrap;
  }

  .detach-btn:hover {
    background: var(--bg-secondary);
    border-color: var(--accent);
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
