import { invoke } from "@tauri-apps/api/core";
import type { CharacterData, CharacterSummary, State, MergedRegistry } from "$lib/types";
import { loadAssets, resetAssetsState } from "./assets.svelte";
import { getProjectPath } from "./project.svelte";
import { TrainingSync, createMainWindowSync } from "$lib/training";

// Training sync instance (created lazily)
let trainingSync: TrainingSync | null = null;

/**
 * Get or create the training sync instance.
 * This is used by the main window to send updates to detached training windows.
 */
export function getTrainingSync(): TrainingSync {
  if (!trainingSync) {
    trainingSync = createMainWindowSync(
      () => currentCharacter,
      () => getProjectPath()
    );
  }
  return trainingSync;
}

/** Clean up training sync on app shutdown. */
export function destroyTrainingSync(): void {
  if (trainingSync) {
    trainingSync.destroy();
    trainingSync = null;
  }
}

/** Notify training windows of a character change (for live sync). */
function notifyCharacterChange(): void {
  if (currentCharacter && trainingSync) {
    trainingSync.sendCharacterChange(currentCharacter);
  }
}

/** Notify training windows of a character save (for sync-on-save). */
function notifyCharacterSave(): void {
  if (currentCharacter && trainingSync) {
    trainingSync.sendCharacterSave(currentCharacter);
  }
}

// Reactive state using Svelte 5 runes
let characterList = $state<CharacterSummary[]>([]);
let currentCharacter = $state<CharacterData | null>(null);
let rulesRegistry = $state<MergedRegistry | null>(null);
let selectedMoveInput = $state<string | null>(null);
let loading = $state(false);
let error = $state<string | null>(null);

let selectSeq = 0;

function formatError(e: unknown): string {
  if (e instanceof Error) return e.message;
  if (typeof e === "string") return e;
  try {
    return JSON.stringify(e);
  } catch {
    return String(e);
  }
}

function getCharactersDir(): string | null {
  const projectPath = getProjectPath();
  if (!projectPath) return null;
  return `${projectPath}/characters`;
}

export function getCharacterList() {
  return characterList;
}

export function getCurrentCharacter() {
  return currentCharacter;
}

export function getRulesRegistry() {
  return rulesRegistry;
}

export function getSelectedMove(): State | null {
  if (!currentCharacter || !selectedMoveInput) return null;
  return currentCharacter.moves.find((m) => m.input === selectedMoveInput) ?? null;
}

export function getSelectedMoveInput() {
  return selectedMoveInput;
}

export function isLoading() {
  return loading;
}

export function getError() {
  return error;
}

export async function loadCharacterList(): Promise<void> {
  const charactersDir = getCharactersDir();
  if (!charactersDir) {
    characterList = [];
    return;
  }

  loading = true;
  error = null;
  try {
    characterList = await invoke<CharacterSummary[]>("list_characters", {
      charactersDir,
    });
  } catch (e) {
    error = String(e);
  } finally {
    loading = false;
  }
}

export async function selectCharacter(characterId: string): Promise<void> {
  const charactersDir = getCharactersDir();
  if (!charactersDir) {
    error = "No project open";
    resetAssetsState();
    return;
  }

  if (currentCharacter?.character.id === characterId) {
    return;
  }

  const seq = ++selectSeq;
  loading = true;
  error = null;
  selectedMoveInput = null;
  rulesRegistry = null;
  resetAssetsState();
  try {
    const [nextCharacter, registry] = await Promise.all([
      invoke<CharacterData>("load_character", {
        charactersDir,
        characterId,
      }),
      invoke<MergedRegistry>("load_rules_registry", {
        charactersDir,
        characterId,
      }),
    ]);

    if (seq !== selectSeq) return;

    currentCharacter = nextCharacter;
    rulesRegistry = registry;
    void loadAssets(characterId);

    // Notify training windows of character change
    notifyCharacterChange();
  } catch (e) {
    if (seq !== selectSeq) return;
    error = formatError(e);
    currentCharacter = null;
    rulesRegistry = null;
    resetAssetsState();
  } finally {
    if (seq === selectSeq) {
      loading = false;
    }
  }
}

export function selectMove(input: string): void {
  selectedMoveInput = input;
}

export function clearSelection(): void {
  selectSeq++;
  currentCharacter = null;
  rulesRegistry = null;
  selectedMoveInput = null;
  resetAssetsState();
}

export function resetCharacterState(): void {
  selectSeq++;
  characterList = [];
  currentCharacter = null;
  rulesRegistry = null;
  selectedMoveInput = null;
  error = null;
  resetAssetsState();
}

export async function saveMove(mv: State): Promise<void> {
  const charactersDir = getCharactersDir();
  if (!charactersDir) {
    throw new Error("No project open");
  }
  if (!currentCharacter) {
    throw new Error("No character selected");
  }

  loading = true;
  error = null;
  try {
    await invoke("save_move", {
      charactersDir,
      characterId: currentCharacter.character.id,
      mv,
    });

    // Update local state
    const index = currentCharacter.moves.findIndex((m) => m.input === mv.input);
    if (index >= 0) {
      currentCharacter.moves[index] = mv;
    }

    // Notify training windows of save
    notifyCharacterSave();
  } catch (e) {
    error = String(e);
    throw e;
  } finally {
    loading = false;
  }
}

export async function exportCharacter(
  adapter: string,
  outputPath: string,
  pretty: boolean = false
): Promise<void> {
  const charactersDir = getCharactersDir();
  if (!charactersDir) {
    throw new Error("No project open");
  }
  if (!currentCharacter) {
    throw new Error("No character selected");
  }

  await invoke("export_character", {
    charactersDir,
    characterId: currentCharacter.character.id,
    adapter,
    outputPath,
    pretty,
  });
}

export async function createCharacter(
  id: string,
  name: string,
  archetype: string
): Promise<void> {
  const charactersDir = getCharactersDir();
  if (!charactersDir) {
    throw new Error("No project open");
  }

  await invoke("create_character", {
    charactersDir,
    id,
    name,
    archetype,
  });

  // Reload character list
  await loadCharacterList();
}

export async function cloneCharacter(
  sourceId: string,
  newId: string,
  newName: string
): Promise<void> {
  const charactersDir = getCharactersDir();
  if (!charactersDir) {
    throw new Error("No project open");
  }

  await invoke("clone_character", {
    charactersDir,
    sourceId,
    newId,
    newName,
  });

  // Reload character list
  await loadCharacterList();
}

export async function deleteCharacter(characterId: string): Promise<void> {
  const charactersDir = getCharactersDir();
  if (!charactersDir) {
    throw new Error("No project open");
  }

  await invoke("delete_character", {
    charactersDir,
    characterId,
  });

  // Clear selection if deleted character was selected
  if (currentCharacter?.character.id === characterId) {
    currentCharacter = null;
    selectedMoveInput = null;
    resetAssetsState();
  }

  // Reload character list
  await loadCharacterList();
}

export async function createMove(input: string, name: string): Promise<State> {
  const charactersDir = getCharactersDir();
  if (!charactersDir) {
    throw new Error("No project open");
  }
  if (!currentCharacter) {
    throw new Error("No character selected");
  }

  const mv = await invoke<State>("create_move", {
    charactersDir,
    characterId: currentCharacter.character.id,
    input,
    name,
  });

  // Add move to local state
  currentCharacter.moves = [...currentCharacter.moves, mv];

  // Select the new move
  selectedMoveInput = mv.input;

  return mv;
}
