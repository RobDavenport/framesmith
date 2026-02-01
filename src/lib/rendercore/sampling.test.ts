import { describe, it, expect } from 'vitest';

import { clampSpriteFrame, frameIndexToSeconds, spriteSheetFrameUv } from './sampling';

describe('rendercore sampling', () => {
  it('clamps sprite frameIndex into [0..frames-1]', () => {
    expect(clampSpriteFrame(-1, 10)).toBe(0);
    expect(clampSpriteFrame(0, 10)).toBe(0);
    expect(clampSpriteFrame(9, 10)).toBe(9);
    expect(clampSpriteFrame(10, 10)).toBe(9);
  });

  it('maps frameIndex to seconds using fps', () => {
    expect(frameIndexToSeconds(0, 60)).toBe(0);
    expect(frameIndexToSeconds(30, 60)).toBeCloseTo(0.5);
    expect(frameIndexToSeconds(60, 60)).toBeCloseTo(1);
  });

  it('computes sprite sheet UVs for multi-row atlases', () => {
    // image: 128x64, frames: 32x32 -> cols=4, rows=2
    const uv0 = spriteSheetFrameUv({
      frameIndex: 0,
      frames: 6,
      frameW: 32,
      frameH: 32,
      imageW: 128,
      imageH: 64,
    });
    expect(uv0.repeatX).toBeCloseTo(0.25);
    expect(uv0.repeatY).toBeCloseTo(0.5);
    expect(uv0.offsetX).toBeCloseTo(0);
    expect(uv0.offsetY).toBeCloseTo(0.5);

    const uv5 = spriteSheetFrameUv({
      frameIndex: 5,
      frames: 6,
      frameW: 32,
      frameH: 32,
      imageW: 128,
      imageH: 64,
    });
    expect(uv5.offsetX).toBeCloseTo(0.25);
    expect(uv5.offsetY).toBeCloseTo(0);
  });
});
