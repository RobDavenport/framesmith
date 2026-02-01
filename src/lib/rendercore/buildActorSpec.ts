import type { AnimationClip } from "$lib/types";

import type { ActorSpec, Facing } from "./types";

type BuildActorSpecResult = {
  spec: ActorSpec | null;
  error: string | null;
};

type BuildActorSpecInput =
  | {
      id: string;
      pos: { x: number; y: number };
      facing: Facing;
      clip: Extract<AnimationClip, { mode: "sprite" }>;
      texturePath: string | null;
      frameIndex: number;
    }
  | {
      id: string;
      pos: { x: number; y: number };
      facing: Facing;
      clip: Extract<AnimationClip, { mode: "gltf" }>;
      modelPath: string | null;
      frameIndex: number;
    };

export function buildActorSpec(input: BuildActorSpecInput): BuildActorSpecResult {
  if ("texturePath" in input) {
    const texturePath = input.texturePath;
    if (!texturePath) {
      return { spec: null, error: "Missing sprite texture path" };
    }

    return {
      spec: {
        id: input.id,
        pos: input.pos,
        facing: input.facing,
        visual: {
          kind: "sprite",
          texturePath,
          clip: input.clip,
          frameIndex: input.frameIndex,
        },
      },
      error: null,
    };
  }

  const modelPath = input.modelPath;
  if (!modelPath) {
    return { spec: null, error: "Missing glTF model path" };
  }

  return {
    spec: {
      id: input.id,
      pos: input.pos,
      facing: input.facing,
      visual: {
        kind: "gltf",
        modelPath,
        clip: input.clip,
        frameIndex: input.frameIndex,
      },
    },
    error: null,
  };
}
