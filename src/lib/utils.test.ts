import { describe, expect, it } from 'vitest';
import { getStateKey, isResolvedVariantState } from './utils';
import type { State } from './types';

const baseState: State = {
  input: '5H',
  name: 'Standing Heavy',
  tags: [],
  startup: 10,
  active: 3,
  recovery: 20,
  damage: 700,
  hitstun: 20,
  blockstun: 15,
  hitstop: 10,
  guard: 'mid',
  hitboxes: [],
  hurtboxes: [],
  pushback: { hit: 5, block: 8 },
  meter_gain: { hit: 100, whiff: 20 },
  animation: '5H',
};

describe('state identity helpers', () => {
  it('uses input for base states', () => {
    expect(getStateKey(baseState)).toBe('5H');
    expect(isResolvedVariantState(baseState)).toBe(false);
  });

  it('uses id for resolved variants that share gameplay input', () => {
    const variant = { ...baseState, id: '5H~level1' };

    expect(getStateKey(variant)).toBe('5H~level1');
    expect(isResolvedVariantState(variant)).toBe(true);
  });
});
