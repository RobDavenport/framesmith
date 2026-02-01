import { describe, it, expect } from 'vitest';

import { getMoveForStateIndex, buildActorSpecForMoveAnimation } from './renderMapping';

describe('training renderMapping', () => {
  it('returns null when current_state index is out of bounds', () => {
    const moves = [{ input: '5L', animation: 'a' } as any];
    expect(getMoveForStateIndex(moves, -1)).toBeNull();
    expect(getMoveForStateIndex(moves, 1)).toBeNull();
  });

  it('builds a sprite ActorSpec and clamps frameIndex', () => {
    const assets = {
      version: 1,
      textures: { atlas: 'assets/atlas.png' },
      models: {},
      animations: {
        a: {
          mode: 'sprite',
          texture: 'atlas',
          frame_size: { w: 32, h: 32 },
          frames: 4,
          pivot: { x: 16, y: 24 },
        },
      },
    } as any;

    const res = buildActorSpecForMoveAnimation({
      assets,
      animationKey: 'a',
      actorId: 'p1',
      pos: { x: 0, y: 0 },
      facing: 'right',
      frameIndex: 99,
    });

    expect(res.error).toBeNull();
    expect(res.spec).not.toBeNull();
    if (!res.spec) throw new Error('Expected spec');
    expect(res.spec.visual.kind).toBe('sprite');
    if (res.spec.visual.kind !== 'sprite') throw new Error('Expected sprite visual');
    expect(res.spec.visual.texturePath).toBe('assets/atlas.png');
    expect(res.spec.visual.frameIndex).toBe(3);
  });

  it('returns an error when the animation key is missing', () => {
    const assets = { version: 1, textures: {}, models: {}, animations: {} } as any;
    const res = buildActorSpecForMoveAnimation({
      assets,
      animationKey: 'missing',
      actorId: 'p1',
      pos: { x: 0, y: 0 },
      facing: 'right',
      frameIndex: 0,
    });
    expect(res.spec).toBeNull();
    expect(res.error).toMatch(/not found/i);
  });
});
