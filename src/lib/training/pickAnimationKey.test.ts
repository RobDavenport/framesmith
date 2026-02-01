import { describe, it, expect } from 'vitest';

import { pickAnimationKey } from './pickAnimationKey';

describe('pickAnimationKey', () => {
  it('returns preferred key when present', () => {
    const assets = { animations: { a: {}, idle: {} } } as any;
    expect(pickAnimationKey(assets, 'a')).toEqual({ key: 'a', note: null });
  });

  it('falls back to idle when preferred missing', () => {
    const assets = { animations: { idle: {}, other: {} } } as any;
    const res = pickAnimationKey(assets, 'missing');
    expect(res.key).toBe('idle');
    expect(res.note).toMatch(/fallback: 'idle'/);
  });

  it('falls back to first available animation when idle missing', () => {
    const assets = { animations: { z: {}, a: {} } } as any;
    const res = pickAnimationKey(assets, 'missing');
    expect(res.key).toBe('z');
    expect(res.note).toMatch(/fallback/);
  });

  it('returns null when there are no animations', () => {
    const assets = { animations: {} } as any;
    const res = pickAnimationKey(assets, 'missing');
    expect(res.key).toBeNull();
    expect(res.note).toMatch(/no animations/i);
  });
});
