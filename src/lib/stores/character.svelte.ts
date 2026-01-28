import { invoke } from "@tauri-apps/api/core";
import type { CharacterData, CharacterSummary, Move } from "$lib/types";

// Reactive state using Svelte 5 runes
let characterList = $state<CharacterSummary[]>([]);
let currentCharacter = $state<CharacterData | null>(null);
let selectedMoveInput = $state<string | null>(null);
let loading = $state(false);
let error = $state<string | null>(null);

const CHARACTERS_DIR = "characters";

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
  loading = true;
  error = null;
  try {
    characterList = await invoke<CharacterSummary[]>("list_characters", {
      charactersDir: CHARACTERS_DIR,
    });
  } catch (e) {
    error = String(e);
  } finally {
    loading = false;
  }
}

export async function selectCharacter(characterId: string): Promise<void> {
  loading = true;
  error = null;
  selectedMoveInput = null;
  try {
    currentCharacter = await invoke<CharacterData>("load_character", {
      charactersDir: CHARACTERS_DIR,
      characterId,
    });
  } catch (e) {
    error = String(e);
    currentCharacter = null;
  } finally {
    loading = false;
  }
}

export function selectMove(input: string): void {
  selectedMoveInput = input;
}

export function clearSelection(): void {
  currentCharacter = null;
  selectedMoveInput = null;
}

export async function saveMove(mv: Move): Promise<void> {
  if (!currentCharacter) {
    throw new Error("No character selected");
  }

  loading = true;
  error = null;
  try {
    await invoke("save_move", {
      charactersDir: CHARACTERS_DIR,
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
