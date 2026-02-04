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
  import DetachedTraining from './DetachedTraining.svelte';
  import { initWasm } from '$lib/training/TrainingSession';
  import { createDetachedWindowSync, type SyncMode } from '$lib/training';
  import type { CharacterAssets, CharacterData } from '$lib/types';

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

  // Initialization state
  let isInitializing = $state(true);
  let initError = $state<string | null>(null);

  let initSeq = 0;
  let fspkSeq = 0;
  let destroyed = false;

  // Training sync
  let trainingSync = $state<ReturnType<typeof createDetachedWindowSync> | null>(null);

  // FSPK bytes for training session
  let fspkBytes = $state<Uint8Array | null>(null);

  // Rendering assets (loaded via Tauri)
  let renderAssets = $state<CharacterAssets | null>(null);
  let renderAssetsLoading = $state(false);
  let renderAssetsError = $state<string | null>(null);
  let renderAssetsSeq = 0;

  function formatError(e: unknown): string {
    if (e instanceof Error) return e.message;
    if (typeof e === 'string') return e;
    try {
      return JSON.stringify(e);
    } catch {
      return String(e);
    }
  }

  // Load rendering assets when character changes
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

  // Load FSPK bytes when character changes
  $effect(() => {
    if (!currentCharacter || !projectPath) {
      fspkBytes = null;
      return;
    }

    const seq = ++fspkSeq;
    const dir = `${projectPath}/characters`;
    const id = currentCharacter.character.id;

    void invoke<string>('get_character_fspk', {
      charactersDir: dir,
      characterId: id,
    })
      .then((fspkBase64) => {
        if (destroyed || seq !== fspkSeq) return;

        const binaryString = atob(fspkBase64);
        const bytes = new Uint8Array(binaryString.length);
        for (let i = 0; i < binaryString.length; i++) {
          bytes[i] = binaryString.charCodeAt(i);
        }
        fspkBytes = bytes;
      })
      .catch((e) => {
        if (destroyed || seq !== fspkSeq) return;
        console.error('Failed to load FSPK:', e);
        initError = formatError(e);
      });
  });

  // Handle character update from sync
  async function handleCharacterUpdate(character: CharacterData) {
    if (destroyed) return;
    currentCharacter = character;
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
    } catch (e) {
      console.error('Failed to initialize:', e);
      if (destroyed || seq !== initSeq) return;
      initError = e instanceof Error ? e.message : String(e);
      isInitializing = false;
    }
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
    trainingSync?.destroy();
    trainingSync = null;
  }

  // Lifecycle
  onMount(() => {
    initialize();
  });

  onDestroy(() => {
    destroyed = true;
    initSeq++;
    fspkSeq++;
    renderAssetsSeq++;
    renderAssetsLoading = false;
    cleanup();
  });
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
    <DetachedTraining
      currentCharacter={currentCharacter}
      projectPath={projectPath}
      fspkBytes={fspkBytes}
      renderAssets={renderAssets}
      renderAssetsLoading={renderAssetsLoading}
      renderAssetsError={renderAssetsError}
    />
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
</style>
