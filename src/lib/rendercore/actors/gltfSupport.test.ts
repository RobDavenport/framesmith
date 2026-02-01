import { describe, it, expect } from 'vitest';

import { isSupportedGltfModelPath } from './gltfSupport';

describe('gltfSupport', () => {
  it('supports .glb and rejects .gltf', () => {
    expect(isSupportedGltfModelPath('models/char.glb')).toBe(true);
    expect(isSupportedGltfModelPath('models/char.gltf')).toBe(false);
  });
});
