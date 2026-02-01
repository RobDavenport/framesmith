<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import type { GlobalInclude, GlobalsManifest, GlobalStateSummary } from '$lib/types';
  import { getCurrentCharacter } from '$lib/stores/character.svelte';
  import { getGlobalStateList, loadGlobalStateList } from '$lib/stores/globals.svelte';
  import { getProjectPath } from '$lib/stores/project.svelte';
  import { onMount } from 'svelte';

  let manifest = $state<GlobalsManifest>({ includes: [] });
  let availableGlobals = $state<GlobalStateSummary[]>([]);
  let loading = $state(false);
  let saving = $state(false);
  let error = $state<string | null>(null);

  const character = $derived(getCurrentCharacter());

  onMount(async () => {
    await loadGlobalStateList();
    availableGlobals = getGlobalStateList();
    if (character) {
      await loadManifest();
    }
  });

  async function loadManifest() {
    if (!character) return;

    const projectPath = getProjectPath();
    if (!projectPath) return;

    loading = true;
    error = null;

    try {
      manifest = await invoke<GlobalsManifest>('get_character_globals', {
        projectPath,
        characterId: character.character.id,
      });
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
      manifest = { includes: [] };
    } finally {
      loading = false;
    }
  }

  async function saveManifest() {
    if (!character) return;

    const projectPath = getProjectPath();
    if (!projectPath) return;

    saving = true;
    error = null;

    try {
      await invoke('save_character_globals', {
        projectPath,
        characterId: character.character.id,
        manifest,
      });
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      saving = false;
    }
  }

  function addInclude(globalId: string) {
    if (manifest.includes.some(i => i.state === globalId)) return;

    manifest.includes = [...manifest.includes, {
      state: globalId,
      as: globalId,
    }];
  }

  function removeInclude(index: number) {
    manifest.includes = manifest.includes.filter((_, i) => i !== index);
  }

  function updateAlias(index: number, alias: string) {
    manifest.includes[index].as = alias;
  }
</script>

<div class="character-globals-editor">
  <h4>Global State Includes</h4>

  {#if error}
    <div class="error">{error}</div>
  {/if}

  {#if loading}
    <p class="loading">Loading...</p>
  {:else}
    <div class="includes-list">
      {#each manifest.includes as include, index (index)}
        <div class="include-item">
          <span class="global-name">{include.state}</span>
          <label>
            as:
            <input
              type="text"
              value={include.as}
              oninput={(e) => updateAlias(index, e.currentTarget.value)}
            />
          </label>
          <button class="remove-btn" onclick={() => removeInclude(index)}>Remove</button>
        </div>
      {/each}
    </div>

    {#if manifest.includes.length === 0}
      <p class="empty">No global states included. Add one below.</p>
    {/if}

    <div class="add-global">
      <select onchange={(e) => {
        if (e.currentTarget.value) {
          addInclude(e.currentTarget.value);
          e.currentTarget.value = '';
        }
      }}>
        <option value="">Add global state...</option>
        {#each availableGlobals as global (global.id)}
          {#if !manifest.includes.some(i => i.state === global.id)}
            <option value={global.id}>{global.id} - {global.name ?? global.id}</option>
          {/if}
        {/each}
      </select>
    </div>

    <div class="actions">
      <button onclick={saveManifest} disabled={saving}>
        {saving ? 'Saving...' : 'Save'}
      </button>
    </div>
  {/if}
</div>

<style>
  .character-globals-editor {
    padding: 1rem;
    border: 1px solid var(--border-color, #333);
    border-radius: 4px;
    margin: 1rem 0;
  }

  h4 {
    margin: 0 0 1rem;
  }

  .error {
    padding: 0.5rem;
    background: rgba(255, 68, 68, 0.1);
    color: var(--danger-color, #ff4444);
    margin-bottom: 1rem;
    border-radius: 4px;
  }

  .loading, .empty {
    color: var(--text-muted, #888);
    font-size: 0.9rem;
  }

  .includes-list {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    margin-bottom: 1rem;
  }

  .include-item {
    display: flex;
    align-items: center;
    gap: 1rem;
    padding: 0.5rem;
    background: var(--bg-secondary, #222);
    border-radius: 4px;
  }

  .global-name {
    font-family: monospace;
    font-weight: 500;
  }

  .include-item label {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    color: var(--text-muted, #888);
    font-size: 0.9rem;
  }

  .include-item input {
    width: 150px;
    padding: 0.25rem 0.5rem;
    border: 1px solid var(--border-color, #333);
    border-radius: 4px;
    background: var(--bg-primary, #1a1a1a);
    color: inherit;
  }

  .remove-btn {
    margin-left: auto;
    padding: 0.25rem 0.5rem;
    font-size: 0.85rem;
    background: transparent;
    border: 1px solid var(--danger-color, #ff4444);
    color: var(--danger-color, #ff4444);
    border-radius: 4px;
    cursor: pointer;
  }

  .remove-btn:hover {
    background: rgba(255, 68, 68, 0.1);
  }

  .add-global select {
    width: 100%;
    padding: 0.5rem;
    border: 1px solid var(--border-color, #333);
    border-radius: 4px;
    background: var(--bg-secondary, #222);
    color: inherit;
  }

  .actions {
    margin-top: 1rem;
    display: flex;
    justify-content: flex-end;
  }

  .actions button {
    padding: 0.5rem 1rem;
    background: var(--accent-color, #4a9eff);
    border: none;
    border-radius: 4px;
    color: white;
    cursor: pointer;
  }

  .actions button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
