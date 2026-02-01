export function clampSpriteFrame(frameIndex: number, frames: number): number {
  if (!Number.isFinite(frameIndex)) return 0;
  if (!Number.isFinite(frames) || frames <= 0) return 0;

  const i = Math.floor(frameIndex);
  const max = Math.floor(frames) - 1;
  if (max <= 0) return 0;
  return Math.min(Math.max(i, 0), max);
}

export function frameIndexToSeconds(frameIndex: number, fps: number): number {
  if (!Number.isFinite(frameIndex)) return 0;
  if (!Number.isFinite(fps) || fps <= 0) return 0;
  return frameIndex / fps;
}

export type SpriteSheetFrameUvArgs = {
  frameIndex: number;
  frames: number;
  frameW: number;
  frameH: number;
  imageW: number;
  imageH: number;
};

export type SpriteSheetFrameUv = {
  offsetX: number;
  offsetY: number;
  repeatX: number;
  repeatY: number;
};

// Returns UV repeat/offset for a sprite frame, treating frameIndex=0 as the
// top-left frame in the spritesheet.
export function spriteSheetFrameUv(args: SpriteSheetFrameUvArgs): SpriteSheetFrameUv {
  const frameW = Number.isFinite(args.frameW) ? Math.floor(args.frameW) : 0;
  const frameH = Number.isFinite(args.frameH) ? Math.floor(args.frameH) : 0;
  const imageW = Number.isFinite(args.imageW) ? Math.floor(args.imageW) : 0;
  const imageH = Number.isFinite(args.imageH) ? Math.floor(args.imageH) : 0;
  const frames = Number.isFinite(args.frames) ? Math.floor(args.frames) : 0;

  if (frameW <= 0 || frameH <= 0 || imageW <= 0 || imageH <= 0 || frames <= 0) {
    return { offsetX: 0, offsetY: 0, repeatX: 1, repeatY: 1 };
  }

  const cols = Math.max(1, Math.floor(imageW / frameW));
  const rows = Math.max(1, Math.floor(imageH / frameH));
  const capacity = cols * rows;
  const usableFrames = Math.max(1, Math.min(frames, capacity));

  const idx = clampSpriteFrame(args.frameIndex, usableFrames);
  const col = idx % cols;
  const row = Math.floor(idx / cols);

  const repeatX = frameW / imageW;
  const repeatY = frameH / imageH;

  // Texture.offset is measured from the bottom-left in Three.js UV space.
  const offsetX = col * repeatX;
  const offsetY = 1 - (row + 1) * repeatY;

  return { offsetX, offsetY, repeatX, repeatY };
}
