import { invoke } from "@tauri-apps/api/core";
import type { CharacterData, CharacterSummary, Move } from "$lib/types";
import { loadAssets, resetAssetsState } from "./assets.svelte";
import { getProjectPath } from "./project.svelte";

// Reactive state using Svelte 5 runes
let characterList = $state<CharacterSummary[]>([]);
let currentCharacter = $state<CharacterData | null>(null);
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

export function getSelectedMove(): Move | null {
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
  resetAssetsState();
  try {
    const nextCharacter = await invoke<CharacterData>("load_character", {
      charactersDir,
      characterId,
    });

    if (seq !== selectSeq) return;

    currentCharacter = nextCharacter;
    void loadAssets(characterId);
  } catch (e) {
    if (seq !== selectSeq) return;
    error = formatError(e);
    currentCharacter = null;
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
  selectedMoveInput = null;
  resetAssetsState();
}

export function resetCharacterState(): void {
  selectSeq++;
  characterList = [];
  currentCharacter = null;
  selectedMoveInput = null;
  error = null;
  resetAssetsState();
}

export async function saveMove(mv: Move): Promise<void> {
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

export async function createMove(input: string, name: string): Promise<Move> {
  const charactersDir = getCharactersDir();
  if (!charactersDir) {
    throw new Error("No project open");
  }
  if (!currentCharacter) {
    throw new Error("No character selected");
  }

  const mv = await invoke<Move>("create_move", {
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
