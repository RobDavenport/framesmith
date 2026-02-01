<script lang="ts">
  /**
   * Detached Training Mode Page
   *
   * This route is opened in a separate Tauri window and receives character
   * data via BroadcastChannel from the main editor window.
   */

  import { onMount, onDestroy } from 'svelte';
  import { page } from '$app/stores';
  import { invoke } from '@tauri-apps/api/core';
  import "$lib/../app.css";
  import TrainingViewport from '$lib/components/training/TrainingViewport.svelte';
  import TrainingHUD from '$lib/components/training/TrainingHUD.svelte';
  import InputHistory from '$lib/components/training/InputHistory.svelte';
  import PlaybackControls, { type PlaybackSpeed } from '$lib/components/training/PlaybackControls.svelte';
  import DummySettings from '$lib/components/training/DummySettings.svelte';
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
    createDetachedWindowSync,
    type TrainingInputConfig,
    type InputSnapshot,
    type SyncMode,
  } from '$lib/training';
  import { pickAnimationKey } from '$lib/training/pickAnimationKey';
  import { buildMoveList } from '$lib/training/buildMoveList';
  import type { CharacterAssets, CharacterData } from '$lib/types';
  import type { ActorSpec, Facing } from '$lib/rendercore/types';
  import { buildActorSpecForMoveAnimation, getMoveForStateIndex } from '$lib/training/renderMapping';

  const devLog = (...args: unknown[]) => {
    if (import.meta.env.DEV) console.debug(...args);
  };

  // Get URL params
  let characterId = $derived($page.url.searchParams.get('character') ?? '');
  let isDetached = $derived($page.url.searchParams.get('detached') === 'true');

  // Sync settings
  let syncMode = $state<SyncMode>('live');
  let projectPath = $state<string | null>(null);
  let currentCharacter = $state<CharacterData | null>(null);

  const charactersDir = $derived.by((): string | null => {
    if (!projectPath) return null;
    return `${projectPath}/characters`;
  });

  const activeCharacterId = $derived.by((): string | null => {
    return currentCharacter?.character.id ?? null;
  });

  // Training state
  let session: TrainingSession | null = $state(null);
  let inputManager: InputManager | null = $state(null);
  let inputBuffer: InputBuffer | null = $state(null);
  let moveResolver: MoveResolver | null = $state(null);
  let dummyController: DummyController | null = $state(null);
  let animationFrameId: number | null = $state(null);
  let isInitializing = $state(true);
  let initError = $state<string | null>(null);

  let initSeq = 0;
  let sessionSeq = 0;
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

  // Character positions
  let playerX = $state(200);
  let playerY = $state(0);
  let dummyX = $state(600);
  let dummyY = $state(0);

  // Health tracking
  let playerHealth = $state(10000);
  let dummyHealth = $state(10000);
  let maxHealth = $state(10000);

  // Combo tracking
  let comboHits = $state(0);
  let comboDamage = $state(0);
  let comboResetTimer = $state(0);
  const COMBO_RESET_FRAMES = 60;

  // Input history
  let inputHistory = $state<InputSnapshot[]>([]);
  const INPUT_HISTORY_MAX = 30;

  // Playback controls
  let isPlaying = $state(true);
  let playbackSpeed = $state<PlaybackSpeed>(1);
  let frameAccumulator = $state(0);

  // Developer overlay toggles
  let showHitboxes = $state(false);
  let dummySettingsCollapsed = $state(false);

  // Training sync
  let trainingSync = $state<ReturnType<typeof createDetachedWindowSync> | null>(null);

  // Default input config
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
    const id = activeCharacterId;

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


  // Handle character update from sync
  async function handleCharacterUpdate(character: CharacterData) {
    if (destroyed) return;
    currentCharacter = character;

    // Always (re)initialize on new character payload.
    // Don't await here so burst updates coalesce via sessionSeq.
    void reinitializeSession();
  }

  // Initialize or reinitialize the training session
  async function reinitializeSession() {
    const seq = ++sessionSeq;
    if (!currentCharacter || !projectPath) return;

    try {
      // Stop current game loop
      stopGameLoop();

      // Free old session
      session?.free();
      session = null;

      // Get FSPK bytes
      const charactersDir = `${projectPath}/characters`;
      const fspkBase64 = await invoke<string>('get_character_fspk', {
        charactersDir,
        characterId: currentCharacter.character.id,
      });

      if (destroyed || seq !== sessionSeq) return;

      const binaryString = atob(fspkBase64);
      const fspkBytes = new Uint8Array(binaryString.length);
      for (let i = 0; i < binaryString.length; i++) {
        fspkBytes[i] = binaryString.charCodeAt(i);
      }

      // Create new session
      const nextSession = await TrainingSession.create(fspkBytes, fspkBytes);

      if (destroyed || seq !== sessionSeq) {
        nextSession.free();
        return;
      }

      session = nextSession;

      // Update move resolver
      moveResolver = new MoveResolver(buildMoveList(currentCharacter?.moves));

      // Reset health
      maxHealth = currentCharacter.character.health;
      playerHealth = maxHealth;
      dummyHealth = maxHealth;

      // Get initial state
      playerState = session.playerState();
      dummyState = session.dummyState();

      // Restart game loop
      startGameLoop(seq);
    } catch (e) {
      console.error('Failed to reinitialize session:', e);
      if (destroyed || seq !== sessionSeq) return;
      initError = e instanceof Error ? e.message : String(e);
    }
  }

  // Initialize training mode
  async function initialize() {
    const seq = ++initSeq;
    isInitializing = true;
    initError = null;

    cleanup();

    try {
      // Initialize WASM
      await initWasm();

      if (destroyed || seq !== initSeq) return;

      // Create sync channel
      const nextSync = createDetachedWindowSync(
        handleCharacterUpdate,
        (path) => {
          projectPath = path;
          // Project path and character data can arrive in either order.
          void reinitializeSession();
        },
        syncMode
      );

      if (destroyed || seq !== initSeq) {
        nextSync.destroy();
        return;
      }

      trainingSync = nextSync;

      // Request initial data from main window
      trainingSync.requestSync();

      // Initialize input system
      const nextInputManager = new InputManager(defaultInputConfig);
      const nextInputBuffer = new InputBuffer();
      const nextDummyController = new DummyController();
      const nextMoveResolver = new MoveResolver(buildMoveList(currentCharacter?.moves));

      if (destroyed || seq !== initSeq) {
        nextInputManager.reset();
        return;
      }

      inputManager = nextInputManager;
      inputBuffer = nextInputBuffer;
      dummyController = nextDummyController;
      moveResolver = nextMoveResolver;

      // Wait for main window to respond using ping/pong retry mechanism
      const mainWindowReady = await trainingSync.waitForMainWindow(5, 100);

      if (destroyed || seq !== initSeq) return;

      if (!currentCharacter && characterId) {
        if (!mainWindowReady) {
          devLog('No sync data received, main window may not be ready...');
        } else {
          devLog('Main window responded but no character data yet, waiting...');
        }
      }

      isInitializing = false;

      // If we have character data, start the game loop
      if (currentCharacter) {
        await reinitializeSession();
      }
    } catch (e) {
      console.error('Failed to initialize:', e);
      if (destroyed || seq !== initSeq) return;
      initError = e instanceof Error ? e.message : String(e);
      isInitializing = false;
    }
  }

  // Game loop
  function startGameLoop(loopSeq: number) {
    stopGameLoop();
    let lastTime = performance.now();

    function gameLoop(currentTime: number) {
      if (destroyed || loopSeq !== sessionSeq) return;
      if (!session || !inputManager || !inputBuffer || !moveResolver || !dummyController) {
        return;
      }

      if (!isPlaying) {
        animationFrameId = requestAnimationFrame(gameLoop);
        lastTime = currentTime;
        return;
      }

      if (playbackSpeed === 0) {
        animationFrameId = requestAnimationFrame(gameLoop);
        lastTime = currentTime;
        return;
      }

      const deltaTime = currentTime - lastTime;
      lastTime = currentTime;

      const frameTime = 16.67;
      frameAccumulator += deltaTime * playbackSpeed;

      if (frameAccumulator < frameTime) {
        animationFrameId = requestAnimationFrame(gameLoop);
        return;
      }

      frameAccumulator = Math.min(frameAccumulator - frameTime, frameTime);
      tickOneFrame();

      animationFrameId = requestAnimationFrame(gameLoop);
    }

    animationFrameId = requestAnimationFrame(gameLoop);
  }

  function stopGameLoop() {
    if (animationFrameId !== null) {
      cancelAnimationFrame(animationFrameId);
      animationFrameId = null;
    }
  }

  function tickOneFrame() {
    if (!session || !inputManager || !inputBuffer || !moveResolver || !dummyController) {
      return;
    }

    const snapshot = inputManager.getSnapshot();
    inputBuffer.push(snapshot);
    inputHistory = [...inputHistory.slice(-(INPUT_HISTORY_MAX - 1)), snapshot];

    const newlyPressed = inputManager.newlyPressedButtons;
    const resolved = moveResolver.resolve(inputBuffer, newlyPressed, session.availableCancels());
    inputManager.consumeNewlyPressed();

    const playerInput = resolved ? resolved.index : NO_INPUT;
    const wasmDummyState = dummyController.getWasmState();

    let result: FrameResult;
    try {
      result = session.tick(playerInput, wasmDummyState);
    } catch (e) {
      console.error('WASM tick error:', e);
      if (animationFrameId !== null) {
        cancelAnimationFrame(animationFrameId);
        animationFrameId = null;
      }
      initError = `WASM error: ${e instanceof Error ? e.message : String(e)}`;
      return;
    }

    playerState = result.player;
    dummyState = result.dummy;

    const hits = result.hits;
    if (hits.length > 0) {
      for (const hit of hits) {
        dummyHealth = Math.max(0, dummyHealth - hit.damage);
        comboHits++;
        comboDamage += hit.damage;
      }
      comboResetTimer = COMBO_RESET_FRAMES;
    } else {
      if (comboResetTimer > 0) {
        comboResetTimer--;
        if (comboResetTimer === 0) {
          comboHits = 0;
          comboDamage = 0;
        }
      }
    }

    frameCount++;
  }

  // Keyboard handlers
  function handleKeyDown(event: KeyboardEvent) {
    if (!inputManager) return;

    if (event.code === 'KeyR') {
      resetHealth();
      return;
    }
    if (event.code === 'Space') {
      event.preventDefault();
      togglePlayPause();
      return;
    }
    if (event.code === 'Period') {
      stepForward();
      return;
    }
    if (event.code === 'KeyH') {
      showHitboxes = !showHitboxes;
      return;
    }

    inputManager.handleKeyDown(event.code);
  }

  function handleKeyUp(event: KeyboardEvent) {
    if (!inputManager) return;
    inputManager.handleKeyUp(event.code);
  }

  // Playback controls
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
    // Not implemented - requires state history
  }

  function setPlaybackSpeed(speed: PlaybackSpeed) {
    playbackSpeed = speed;
  }

  function resetHealth() {
    playerHealth = maxHealth;
    dummyHealth = maxHealth;
    session?.reset();
  }

  // Sync mode toggle
  function handleSyncModeChange(event: Event) {
    const target = event.target as HTMLSelectElement;
    syncMode = target.value as SyncMode;

    // Recreate sync with new mode
    trainingSync?.destroy();
    trainingSync = createDetachedWindowSync(
      handleCharacterUpdate,
      (path) => {
        projectPath = path;
      },
      syncMode
    );

    // Request fresh data with the new sync mode
    trainingSync.requestSync();
  }

  // Cleanup
  function cleanup() {
    sessionSeq++;
    stopGameLoop();
    inputManager?.reset();
    inputManager = null;
    inputBuffer = null;
    moveResolver = null;
    dummyController = null;
    session?.free();
    session = null;
    trainingSync?.destroy();
    trainingSync = null;
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
    sessionSeq++;
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

    const totalFrames = move.startup + move.active + move.recovery;
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

  const availableCancelNames = $derived.by(() => {
    if (!session || !moveResolver) return [];
    const cancelIndices = session.availableCancels();
    const resolver = moveResolver;
    return cancelIndices
      .map(idx => resolver.getMove(idx)?.name)
      .filter((name): name is string => name !== undefined);
  });

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

  const availableMoveNames = $derived(currentCharacter?.moves.map(m => m.input) ?? []);
</script>

<div class="training-page">
  <!-- Sync mode indicator (only in detached mode) -->
  {#if isDetached}
    <div class="sync-indicator">
      <span class="sync-label">Sync:</span>
      <select value={syncMode} onchange={handleSyncModeChange}>
        <option value="live">Live</option>
        <option value="on-save">On Save</option>
      </select>
      <span class="character-label">
        {currentCharacter ? currentCharacter.character.name : 'Waiting for data...'}
      </span>
    </div>
  {/if}

  {#if isInitializing}
    <div class="loading">
      <p>Initializing training mode...</p>
      {#if isDetached}
        <p class="hint">Waiting for data from main window...</p>
      {/if}
    </div>
  {:else if initError}
    <div class="error">
      <h3>Failed to initialize training mode</h3>
      <p>{initError}</p>
    </div>
  {:else if !currentCharacter}
    <div class="waiting">
      <h3>Waiting for character data</h3>
      <p>Make sure the main Framesmith window is open with a character selected.</p>
      <p class="hint">Sync mode: {syncMode === 'live' ? 'Live (updates on every change)' : 'On Save (updates when you save)'}</p>
    </div>
  {:else}
    <!-- HUD -->
    <TrainingHUD
      player={playerStatus}
      dummy={dummyStatusState}
      currentMove={currentMoveInfo}
      dummyStatus={dummyStatusDisplay}
      availableCancels={availableCancelNames}
      comboInfo={comboInfo}
      onResetHealth={resetHealth}
    />

    <!-- Main content -->
    <div class="main-content">
      <div class="viewport-container">
        <div class="viewport-frame">
          <TrainingViewport
            charactersDir={charactersDir ?? ''}
            characterId={activeCharacterId ?? ''}
            actors={renderBuild.actors}
            error={renderBuild.error}
          />
        </div>
      </div>

      <div class="sidebar">
        <InputHistory inputs={inputHistory} maxDisplay={15} />
        {#if dummyController}
          <DummySettings
            config={dummyController.config}
            availableMoves={availableMoveNames}
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

    <!-- Bottom bar -->
    <div class="bottom-bar">
      <PlaybackControls
        isPlaying={isPlaying}
        speed={playbackSpeed}
        onPlayPause={togglePlayPause}
        onStepBack={stepBack}
        onStepForward={stepForward}
        onSpeedChange={setPlaybackSpeed}
      />

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
      </div>
    </div>

    <div class="frame-counter">
      Frame: {frameCount}
    </div>
  {/if}
</div>

<style>
  .training-page {
    display: flex;
    flex-direction: column;
    height: 100vh;
    gap: 8px;
    padding: 8px;
    background: var(--bg-primary);
  }

  .sync-indicator {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 6px 12px;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 4px;
  }

  .sync-label {
    font-size: 11px;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .sync-indicator select {
    font-size: 12px;
    padding: 2px 6px;
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: 3px;
    color: var(--text-primary);
  }

  .character-label {
    font-size: 12px;
    color: var(--text-primary);
    margin-left: auto;
  }

  .loading,
  .error,
  .waiting {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    gap: 12px;
  }

  .error h3 {
    color: var(--accent);
  }

  .error p,
  .waiting p {
    color: var(--text-secondary);
    max-width: 400px;
    text-align: center;
  }

  .hint {
    font-size: 12px;
    font-style: italic;
    opacity: 0.7;
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
    width: 800px;
    height: 400px;
    min-width: 0;
    min-height: 0;
  }

  .sidebar {
    display: flex;
    flex-direction: column;
    gap: 8px;
    width: 180px;
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

  .frame-counter {
    position: absolute;
    bottom: 50px;
    right: 200px;
    font-size: 10px;
    font-variant-numeric: tabular-nums;
    color: var(--text-secondary);
    padding: 2px 6px;
    background: var(--bg-tertiary);
    border-radius: 3px;
  }
</style>
