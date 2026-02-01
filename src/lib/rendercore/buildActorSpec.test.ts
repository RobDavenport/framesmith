import { describe, it, expect } from 'vitest';

import { buildActorSpec } from './buildActorSpec';

describe('buildActorSpec', () => {
  it('builds sprite actor spec when texturePath is present', () => {
    const clip = {
      mode: 'sprite',
      texture: 'atlas',
      frame_size: { w: 32, h: 32 },
      frames: 6,
      pivot: { x: 16, y: 24 },
    } as const;

    const result = buildActorSpec({
      id: 'p1',
      pos: { x: 0, y: 0 },
      facing: 'right',
      clip,
      texturePath: 'assets/atlas.png',
      frameIndex: 3,
    });

    expect(result.error).toBeNull();
    expect(result.spec?.visual.kind).toBe('sprite');
    expect(result.spec?.visual.frameIndex).toBe(3);
  });

  it('returns an error when sprite texturePath is missing', () => {
    const clip = {
      mode: 'sprite',
      texture: 'atlas',
      frame_size: { w: 32, h: 32 },
      frames: 6,
      pivot: { x: 16, y: 24 },
    } as const;

    const result = buildActorSpec({
      id: 'p1',
      pos: { x: 0, y: 0 },
      facing: 'right',
      clip,
      texturePath: null,
      frameIndex: 0,
    });

    expect(result.spec).toBeNull();
    expect(result.error).toMatch(/texture/i);
  });

  it('builds gltf actor spec when modelPath is present', () => {
    const clip = {
      mode: 'gltf',
      model: 'body',
      clip: 'Idle',
      fps: 60,
      pivot: { x: 0, y: 0, z: 0 },
    } as const;

    const result = buildActorSpec({
      id: 'p1',
      pos: { x: 1, y: 2 },
      facing: 'left',
      clip,
      modelPath: 'assets/body.glb',
      frameIndex: 10,
    });

    expect(result.error).toBeNull();
    expect(result.spec?.visual.kind).toBe('gltf');
    expect(result.spec?.visual.frameIndex).toBe(10);
  });

  it('returns an error when gltf modelPath is missing', () => {
    const clip = {
      mode: 'gltf',
      model: 'body',
      clip: 'Idle',
      fps: 60,
      pivot: { x: 0, y: 0, z: 0 },
    } as const;

    const result = buildActorSpec({
      id: 'p1',
      pos: { x: 0, y: 0 },
      facing: 'right',
      clip,
      modelPath: null,
      frameIndex: 0,
    });

    expect(result.spec).toBeNull();
    expect(result.error).toMatch(/model/i);
  });
});
