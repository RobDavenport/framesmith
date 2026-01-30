import { invoke } from "@tauri-apps/api/core";
import type { AnimationClip, CharacterAssets } from "$lib/types";
import { getProjectPath } from "./project.svelte";

let assets = $state<CharacterAssets | null>(null);
let loading = $state(false);
let error = $state<string | null>(null);

let loadedForCharacterId: string | null = null;
let loadedForCharactersDir: string | null = null;
let loadSeq = 0;

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

export function getAssets(): CharacterAssets | null {
  return assets;
}

export function getAssetsError(): string | null {
  return error;
}

export function isAssetsLoading(): boolean {
  return loading;
}

export function getAnimationClip(name: string): AnimationClip | null {
  return assets?.animations[name] ?? null;
}

export function resetAssetsState(): void {
  loadSeq++;
  assets = null;
  loading = false;
  error = null;
  loadedForCharacterId = null;
  loadedForCharactersDir = null;
}

export async function loadAssets(characterId: string): Promise<void> {
  const charactersDir = getCharactersDir();
  if (!charactersDir) {
    resetAssetsState();
    error = "No project open";
    return;
  }

  if (assets && loadedForCharacterId === characterId && loadedForCharactersDir === charactersDir) {
    return;
  }

  const seq = ++loadSeq;
  loading = true;
  error = null;
  try {
    const nextAssets = await invoke<CharacterAssets>("load_character_assets", {
      charactersDir,
      characterId,
    });
    if (seq !== loadSeq) return;

    assets = nextAssets;
    loadedForCharacterId = characterId;
    loadedForCharactersDir = charactersDir;
  } catch (e) {
    if (seq !== loadSeq) return;
    error = formatError(e);
    assets = null;
    loadedForCharacterId = null;
    loadedForCharactersDir = null;
  } finally {
    if (seq === loadSeq) {
      loading = false;
    }
  }
}

export async function readAssetBase64(
  characterId: string,
  relativePath: string
): Promise<string> {
  const charactersDir = getCharactersDir();
  if (!charactersDir) {
    throw new Error("No project open");
  }

  return await invoke<string>("read_character_asset_base64", {
    charactersDir,
    characterId,
    relativePath,
  });
}
