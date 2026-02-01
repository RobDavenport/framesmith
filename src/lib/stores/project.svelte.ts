import { invoke } from "@tauri-apps/api/core";
import { resetCharacterState, loadCharacterList } from "./character.svelte";
import { resetGlobalsStore } from "./globals.svelte";

interface ProjectInfo {
  name: string;
  path: string;
  character_count: number;
}

// Reactive state using Svelte 5 runes
let projectPath = $state<string | null>(null);
let projectName = $state<string | null>(null);
let loading = $state(false);
let error = $state<string | null>(null);

export function getProjectPath(): string | null {
  return projectPath;
}

export function getProjectName(): string | null {
  return projectName;
}

export function isProjectOpen(): boolean {
  return projectPath !== null;
}

export function isProjectLoading(): boolean {
  return loading;
}

export function getProjectError(): string | null {
  return error;
}

export async function openProject(): Promise<boolean> {
  loading = true;
  error = null;

  try {
    // Show native folder dialog
    const selectedPath = await invoke<string | null>("open_folder_dialog");
    if (!selectedPath) {
      // User cancelled
      loading = false;
      return false;
    }

    // Validate the project
    const info = await invoke<ProjectInfo>("validate_project", {
      path: selectedPath,
    });

    projectPath = info.path;
    projectName = info.name;

    // Load character list for the new project
    await loadCharacterList();

    loading = false;
    return true;
  } catch (e) {
    error = String(e);
    loading = false;
    return false;
  }
}

export async function createProject(): Promise<boolean> {
  loading = true;
  error = null;

  try {
    // Show native folder dialog
    const selectedPath = await invoke<string | null>("open_folder_dialog");
    if (!selectedPath) {
      // User cancelled
      loading = false;
      return false;
    }

    // Create the project structure
    await invoke("create_project", { path: selectedPath });

    // Validate and load the project
    const info = await invoke<ProjectInfo>("validate_project", {
      path: selectedPath,
    });

    projectPath = info.path;
    projectName = info.name;

    // Load character list for the new project (will be empty)
    await loadCharacterList();

    loading = false;
    return true;
  } catch (e) {
    error = String(e);
    loading = false;
    return false;
  }
}

export function closeProject(): void {
  resetCharacterState();
  resetGlobalsStore();
  projectPath = null;
  projectName = null;
  error = null;
}

export function clearProjectError(): void {
  error = null;
}
