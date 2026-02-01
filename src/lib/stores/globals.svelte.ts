/**
 * Store for managing global states at the project level
 */
import { invoke } from '@tauri-apps/api/core';
import type { State, GlobalStateSummary } from '$lib/types';
import { getProjectPath } from './project.svelte';

// Reactive state using Svelte 5 runes
let globalStateList = $state<GlobalStateSummary[]>([]);
let currentGlobalState = $state<State | null>(null);
let selectedGlobalId = $state<string | null>(null);
let loading = $state(false);
let error = $state<string | null>(null);

// Getters
export function getGlobalStateList(): GlobalStateSummary[] {
  return globalStateList;
}

export function getCurrentGlobalState(): State | null {
  return currentGlobalState;
}

export function getSelectedGlobalId(): string | null {
  return selectedGlobalId;
}

export function isLoading(): boolean {
  return loading;
}

export function getError(): string | null {
  return error;
}

// Actions
export async function loadGlobalStateList(): Promise<void> {
  const projectPath = getProjectPath();
  if (!projectPath) {
    globalStateList = [];
    return;
  }

  loading = true;
  error = null;

  try {
    globalStateList = await invoke<GlobalStateSummary[]>('list_global_states', {
      projectPath,
    });
  } catch (e) {
    error = e instanceof Error ? e.message : String(e);
    globalStateList = [];
  } finally {
    loading = false;
  }
}

export async function selectGlobalState(id: string | null): Promise<void> {
  selectedGlobalId = id;

  if (!id) {
    currentGlobalState = null;
    return;
  }

  const projectPath = getProjectPath();
  if (!projectPath) {
    error = 'No project open';
    return;
  }

  loading = true;
  error = null;

  try {
    currentGlobalState = await invoke<State>('get_global_state', {
      projectPath,
      stateId: id,
    });
  } catch (e) {
    error = e instanceof Error ? e.message : String(e);
    currentGlobalState = null;
  } finally {
    loading = false;
  }
}

export async function saveGlobalState(id: string, state: State): Promise<boolean> {
  const projectPath = getProjectPath();
  if (!projectPath) {
    error = 'No project open';
    return false;
  }

  loading = true;
  error = null;

  try {
    await invoke('save_global_state', {
      projectPath,
      stateId: id,
      state,
    });

    // Refresh list
    await loadGlobalStateList();

    // If this is the current state, refresh it
    if (selectedGlobalId === id) {
      await selectGlobalState(id);
    }

    return true;
  } catch (e) {
    error = e instanceof Error ? e.message : String(e);
    return false;
  } finally {
    loading = false;
  }
}

export async function deleteGlobalState(id: string): Promise<boolean> {
  const projectPath = getProjectPath();
  if (!projectPath) {
    error = 'No project open';
    return false;
  }

  loading = true;
  error = null;

  try {
    await invoke('delete_global_state', {
      projectPath,
      stateId: id,
    });

    // Clear selection if deleted
    if (selectedGlobalId === id) {
      selectedGlobalId = null;
      currentGlobalState = null;
    }

    // Refresh list
    await loadGlobalStateList();

    return true;
  } catch (e) {
    error = e instanceof Error ? e.message : String(e);
    return false;
  } finally {
    loading = false;
  }
}

export async function createGlobalState(id: string, state: State): Promise<boolean> {
  return saveGlobalState(id, state);
}

// Reset on project change
export function resetGlobalsStore(): void {
  globalStateList = [];
  currentGlobalState = null;
  selectedGlobalId = null;
  loading = false;
  error = null;
}
