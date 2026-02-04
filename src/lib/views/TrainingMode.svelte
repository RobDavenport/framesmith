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
    type CharacterState,
  } from '$lib/training/TrainingSession';
  import {
    InputManager,
    InputBuffer,
    MoveResolver,
    DummyController,
    calculateSimpleFrameAdvantage,
    type TrainingInputConfig,
  } from '$lib/training';
  import { pickAnimationKey } from '$lib/training/pickAnimationKey';
  import { buildMoveList } from '$lib/training/buildMoveList';
  import { TrainingLoop } from './training/TrainingLoop';
  import { getCurrentCharacter, getTrainingSync } from '$lib/stores/character.svelte';
  import { getProjectPath } from '$lib/stores/project.svelte';
  import type { CharacterAssets } from '$lib/types';
  import type { ActorSpec, Facing } from '$lib/rendercore/types';
  import { buildActorSpecForMoveAnimation, getMoveForStateIndex } from '$lib/training/renderMapping';

  // Props
  interface Props {
    onExit?: () => void;
  }

  let { onExit }: Props = $props();

  // State
  let trainingLoop: TrainingLoop | null = $state(null);
  let dummyController: DummyController | null = $state(null);
  let moveResolver: MoveResolver | null = $state(null);
  let isInitializing = $state(true);
  let initError = $state<string | null>(null);

  let initSeq = 0;
  let destroyed = false;

  // Rendering assets (loaded via Tauri)
  let renderAssets = $state<CharacterAssets | null>(null);
  let renderAssetsLoading = $state(false);
  let renderAssetsError = $state<string | null>(null);
  let renderAssetsSeq = 0;

  // Developer overlay toggles
  let showBoxOverlay = $state(false);
  let showHitboxes = $state(true);
  let showHurtboxes = $state(true);
  let showPushboxes = $state(true);
  let dummySettingsCollapsed = $state(false);

  // Subscribe to training loop state
  import type { TrainingLoopState } from './training/TrainingLoop';
  let loopState = $state<TrainingLoopState | null>(null);

  $effect(() => {
    if (!trainingLoop) {
      loopState = null;
      return;
    }

    const unsubscribe = trainingLoop.state.subscribe(value => {
      loopState = value;
    });

    return unsubscribe;
  });

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
      const session = await TrainingSession.create(fspkBytes, fspkBytes);

      if (destroyed || seq !== initSeq) {
        session.free();
        return;
      }

      // Initialize input system
      const inputManager = new InputManager(defaultInputConfig);
      const inputBuffer = new InputBuffer();
      const nextMoveResolver = new MoveResolver(buildMoveList(currentCharacter?.moves));
      const nextDummyController = new DummyController();

      if (destroyed || seq !== initSeq) {
        inputManager.reset();
        session.free();
        return;
      }

      moveResolver = nextMoveResolver;
      dummyController = nextDummyController;

      // Create training loop
      const loop = new TrainingLoop({
        session,
        inputManager,
        inputBuffer,
        moveResolver: nextMoveResolver,
        dummyController: nextDummyController,
        character: currentCharacter.character,
        moves: currentCharacter.moves,
        onError: (error) => {
          initError = error;
        },
      });

      if (destroyed || seq !== initSeq) {
        loop.dispose();
        inputManager.reset();
        session.free();
        return;
      }

      trainingLoop = loop;

      // Start game loop
      loop.start();

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


  // Handle keyboard events
  function handleKeyDown(event: KeyboardEvent) {
    if (!trainingLoop) return;

    // Handle special keys
    if (event.code === 'Escape') {
      onExit?.();
      return;
    }
    if (event.code === 'KeyR') {
      trainingLoop.resetHealth();
      return;
    }
    // Playback controls
    if (event.code === 'Space') {
      event.preventDefault();
      trainingLoop.togglePlayPause();
      return;
    }
    if (event.code === 'Period') {
      // Step forward
      trainingLoop.stepForward();
      return;
    }
    if (event.code === 'Comma') {
      // Step back (not implemented - would need state history)
      return;
    }
    // Box overlay toggles
    if (event.code === 'KeyH') {
      showBoxOverlay = !showBoxOverlay;
      return;
    }
    // Individual box type toggles (only when overlay is visible)
    if (event.code === 'Digit1' && showBoxOverlay) {
      showHitboxes = !showHitboxes;
      return;
    }
    if (event.code === 'Digit2' && showBoxOverlay) {
      showHurtboxes = !showHurtboxes;
      return;
    }
    if (event.code === 'Digit3' && showBoxOverlay) {
      showPushboxes = !showPushboxes;
      return;
    }

    // Forward input to training loop's input manager
    trainingLoop.inputManager.handleKeyDown(event.code);
  }

  function handleKeyUp(event: KeyboardEvent) {
    if (!trainingLoop) return;
    trainingLoop.inputManager.handleKeyUp(event.code);
  }

  // Cleanup
  function cleanup() {
    trainingLoop?.dispose();
    trainingLoop = null;
    moveResolver = null;
    dummyController = null;
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
    const state = loopState;

    if (!assets) {
      if (renderAssetsLoading) errs.push('Loading assets...');
      else if (renderAssetsError) errs.push(`Assets error: ${renderAssetsError}`);
    }

    if (!assets || !char || !state?.playerState || !state?.dummyState) {
      return { actors: specs, error: errs.length ? errs.join('\n') : null };
    }

    const resolveOne = (actorId: string, charState: CharacterState, pos: { x: number; y: number }, facing: Facing) => {
      const move = getMoveForStateIndex(char.moves, charState.current_state);
      if (!move) {
        errs.push(`${actorId}: State index out of bounds: ${charState.current_state}`);
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
        frameIndex: charState.frame,
      });
      if (built.error) errs.push(`${actorId}: ${built.error}`);
      if (built.spec) specs.push(built.spec);
    };

    resolveOne('p1', state.playerState, { x: state.playerX, y: state.playerY }, 'right');
    resolveOne('cpu', state.dummyState, { x: state.dummyX, y: state.dummyY }, 'left');

    return { actors: specs, error: errs.length ? errs.join('\n') : null };
  });

  const playerStatus = $derived.by(() => {
    const state = loopState;
    if (!state) return { health: 0, maxHealth: 0, resources: [] };
    return {
      health: state.playerHealth,
      maxHealth: state.maxHealth,
      resources: state.playerState?.resources
        ? state.playerState.resources.slice(0, 2).map((v, i) => ({
            name: i === 0 ? 'Meter' : 'Heat',
            value: v,
            max: 100,
          }))
        : [],
    };
  });

  const dummyStatusState = $derived.by(() => {
    const state = loopState;
    if (!state) return { health: 0, maxHealth: 0, resources: [] };
    return {
      health: state.dummyHealth,
      maxHealth: state.maxHealth,
      resources: state.dummyState?.resources
        ? state.dummyState.resources.slice(0, 2).map((v, i) => ({
            name: i === 0 ? 'Meter' : 'Heat',
            value: v,
            max: 100,
          }))
        : [],
    };
  });

  const currentMoveInfo = $derived.by(() => {
    const state = loopState;
    if (!state?.playerState || !currentCharacter || !moveResolver) return null;
    const moveDef = moveResolver.getMove(state.playerState.current_state);
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
      currentFrame: state.playerState.frame,
      totalFrames,
      advantageOnHit: advantage.onHit,
      advantageOnBlock: advantage.onBlock,
    };
  });

  // Available cancels as move names
  const availableCancelNames = $derived.by(() => {
    if (!trainingLoop || !moveResolver) return [];
    const cancelIndices = trainingLoop.session.availableCancels();
    const resolver = moveResolver; // Capture in local variable for TypeScript
    return cancelIndices
      .map(idx => resolver.getMove(idx)?.name)
      .filter((name): name is string => name !== undefined);
  });

  // Combo info
  const comboInfo = $derived.by(() => {
    const state = loopState;
    if (!state) return { hitCount: 0, totalDamage: 0 };
    return {
      hitCount: state.comboHits,
      totalDamage: state.comboDamage,
    };
  });

  // Hitbox overlay data
  const playerHitboxData = $derived.by(() => {
    const state = loopState;
    const move = state?.playerState && currentCharacter
      ? getMoveForStateIndex(currentCharacter.moves, state.playerState.current_state)
      : null;
    return {
      move,
      frame: state?.playerState?.frame ?? 0,
      x: state?.playerX ?? 0,
      y: state?.playerY ?? 0,
      facing: 'right' as const,
    };
  });

  const dummyHitboxData = $derived.by(() => {
    const state = loopState;
    const move = state?.dummyState && currentCharacter
      ? getMoveForStateIndex(currentCharacter.moves, state.dummyState.current_state)
      : null;
    return {
      move,
      frame: state?.dummyState?.frame ?? 0,
      x: state?.dummyX ?? 0,
      y: state?.dummyY ?? 0,
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
      onResetHealth={() => trainingLoop?.resetHealth()}
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
            show={showBoxOverlay}
            showHitboxes={showHitboxes}
            showHurtboxes={showHurtboxes}
            showPushboxes={showPushboxes}
            width={800}
            height={400}
          />
        </div>
        </div>

      <!-- Right sidebar with input history and dummy settings -->
      <div class="sidebar">
        <InputHistory inputs={loopState?.inputHistory ?? []} maxDisplay={15} />
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
        isPlaying={loopState?.isPlaying ?? false}
        speed={loopState?.playbackSpeed ?? 1}
        onPlayPause={() => trainingLoop?.togglePlayPause()}
        onStepBack={() => {}}
        onStepForward={() => trainingLoop?.stepForward()}
        onSpeedChange={(speed) => trainingLoop?.setPlaybackSpeed(speed)}
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
          <span class="control-label">Boxes:</span>
          <span class="control-keys">H</span>
        </div>
        {#if showBoxOverlay}
          <div class="control-group">
            <span class="control-label">Toggle:</span>
            <span class="control-keys">1=Hit 2=Hurt 3=Push</span>
          </div>
        {/if}
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
      Frame: {loopState?.frameCount ?? 0}
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
