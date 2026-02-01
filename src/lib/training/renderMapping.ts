import type { CharacterAssets } from "$lib/types";

import { buildActorSpec } from "$lib/rendercore/buildActorSpec";
import type { ActorSpec, Facing } from "$lib/rendercore/types";

export function getMoveForStateIndex<T>(moves: readonly T[], idx: number): T | null {
  if (!Number.isFinite(idx)) return null;
  const i = Math.trunc(idx);
  if (i < 0 || i >= moves.length) return null;
  return moves[i] ?? null;
}

type BuildActorSpecForMoveAnimationInput = {
  assets: CharacterAssets;
  animationKey: string;
  actorId: string;
  pos: { x: number; y: number };
  facing: Facing;
  frameIndex: number;
};

type BuildActorSpecForMoveAnimationResult = {
  spec: ActorSpec | null;
  error: string | null;
};

function clampSpriteFrameIndex(frameIndex: number, frames: number): number {
  const f = Number.isFinite(frames) ? Math.floor(frames) : 0;
  const max = Math.max(0, f - 1);
  const v = Number.isFinite(frameIndex) ? Math.floor(frameIndex) : 0;
  return Math.max(0, Math.min(max, v));
}

export function buildActorSpecForMoveAnimation(
  input: BuildActorSpecForMoveAnimationInput
): BuildActorSpecForMoveAnimationResult {
  const animationKey = input.animationKey.trim();
  const clip = input.assets.animations[animationKey];
  if (!clip) {
    return { spec: null, error: `Animation key not found: '${animationKey}'` };
  }

  if (clip.mode === "sprite") {
    const texturePath = input.assets.textures[clip.texture] ?? null;
    return buildActorSpec({
      id: input.actorId,
      pos: input.pos,
      facing: input.facing,
      clip,
      texturePath,
      frameIndex: clampSpriteFrameIndex(input.frameIndex, clip.frames),
    });
  }

  const modelPath = input.assets.models[clip.model] ?? null;
  const v = Number.isFinite(input.frameIndex) ? Math.floor(input.frameIndex) : 0;
  return buildActorSpec({
    id: input.actorId,
    pos: input.pos,
    facing: input.facing,
    clip,
    modelPath,
    frameIndex: Math.max(0, v),
  });
}
