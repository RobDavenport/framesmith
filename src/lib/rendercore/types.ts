import type { AnimationClip } from "$lib/types";

export type Facing = "left" | "right";

export interface ActorStatus {
  loading: boolean;
  error?: string | null;
}

export type ActorVisualSprite = {
  kind: "sprite";
  texturePath: string;
  clip: Extract<AnimationClip, { mode: "sprite" }>;
  frameIndex: number;
};

export type ActorVisualGltf = {
  kind: "gltf";
  modelPath: string;
  clip: Extract<AnimationClip, { mode: "gltf" }>;
  frameIndex: number;
};

export type ActorVisual = ActorVisualSprite | ActorVisualGltf;

export interface ActorSpec {
  id: string;
  pos: { x: number; y: number };
  facing: Facing;
  visual: ActorVisual;
}

export type ClockMode = "manual" | "raf";
export type SceneMode = "single" | "training";
