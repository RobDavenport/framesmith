import { describe, it, expect } from 'vitest';
import {
  matchesFilterGroup,
  getTotal,
  getAdvantageHit,
  getAdvantageBlock,
  formatAdvantage,
  sortMoves,
  filterMoves,
  buildFilterOptions,
} from './frameDataUtils';
import type { State } from '$lib/types';

// Helper to create a minimal State for testing
function makeState(overrides: Partial<State> & { input: string; name: string }): State {
  return {
    startup: 5,
    active: 2,
    recovery: 10,
    damage: 50,
    hitstun: 15,
    blockstun: 10,
    hitstop: 8,
    guard: 'mid',
    hitboxes: [],
    hurtboxes: [],
    pushback: { hit: 2, block: 2 },
    meter_gain: { hit: 5, whiff: 2 },
    animation: 'test',
    ...overrides,
  };
}

describe('matchesFilterGroup', () => {
  it('matches normals by type field', () => {
    const move = makeState({ input: '5L', name: 'Light', type: 'normal' });
    expect(matchesFilterGroup(move, 'normals')).toBe(true);
    expect(matchesFilterGroup(move, 'specials')).toBe(false);
  });

  it('matches specials by type field', () => {
    const move = makeState({ input: '236P', name: 'Fireball', type: 'special' });
    expect(matchesFilterGroup(move, 'specials')).toBe(true);
    expect(matchesFilterGroup(move, 'normals')).toBe(false);
  });

  it('falls back to input pattern when no type set', () => {
    const normal = makeState({ input: '5L', name: 'Light' });
    const special = makeState({ input: '236P', name: 'Fireball' });
    expect(matchesFilterGroup(normal, 'normals')).toBe(true);
    expect(matchesFilterGroup(special, 'specials')).toBe(true);
  });

  it('uses registry filter_groups when available', () => {
    const move = makeState({ input: '5L', name: 'Light', type: 'custom_normal' });
    const registry = {
      move_types: { filter_groups: { normals: ['custom_normal', 'normal'] } },
    };
    expect(matchesFilterGroup(move, 'normals', registry)).toBe(true);
  });

  it('returns false for unknown group without registry', () => {
    const move = makeState({ input: '5L', name: 'Light', type: 'normal' });
    expect(matchesFilterGroup(move, 'grapplers')).toBe(false);
  });
});

describe('getTotal', () => {
  it('sums startup + active + recovery', () => {
    const move = makeState({ input: '5L', name: 'Light', startup: 7, active: 3, recovery: 8 });
    expect(getTotal(move)).toBe(18);
  });

  it('handles zero values', () => {
    const move = makeState({ input: '0_idle', name: 'Idle', startup: 1, active: 1, recovery: 0 });
    expect(getTotal(move)).toBe(2);
  });
});

describe('getAdvantageHit', () => {
  it('returns positive advantage when hitstun > recovery', () => {
    const move = makeState({ input: '5L', name: 'Light', hitstun: 17, recovery: 8 });
    expect(getAdvantageHit(move)).toBe(9);
  });

  it('returns negative advantage when recovery > hitstun', () => {
    const move = makeState({ input: '5H', name: 'Heavy', hitstun: 5, recovery: 20 });
    expect(getAdvantageHit(move)).toBe(-15);
  });

  it('returns zero when equal', () => {
    const move = makeState({ input: '5M', name: 'Medium', hitstun: 10, recovery: 10 });
    expect(getAdvantageHit(move)).toBe(0);
  });
});

describe('getAdvantageBlock', () => {
  it('returns blockstun minus recovery', () => {
    const move = makeState({ input: '5L', name: 'Light', blockstun: 11, recovery: 8 });
    expect(getAdvantageBlock(move)).toBe(3);
  });

  it('returns negative when unsafe', () => {
    const move = makeState({ input: '5H', name: 'Heavy', blockstun: 5, recovery: 20 });
    expect(getAdvantageBlock(move)).toBe(-15);
  });
});

describe('formatAdvantage', () => {
  it('adds + prefix for positive values', () => {
    expect(formatAdvantage(5)).toBe('+5');
  });

  it('keeps - prefix for negative values', () => {
    expect(formatAdvantage(-3)).toBe('-3');
  });

  it('adds + prefix for zero', () => {
    expect(formatAdvantage(0)).toBe('+0');
  });
});

describe('sortMoves', () => {
  const moves = [
    makeState({ input: '5H', name: 'Heavy', startup: 12, damage: 80 }),
    makeState({ input: '5L', name: 'Light', startup: 7, damage: 30 }),
    makeState({ input: '5M', name: 'Medium', startup: 9, damage: 50 }),
  ];

  it('sorts by string column ascending', () => {
    const sorted = sortMoves(moves, 'input', 'asc');
    expect(sorted.map((m) => m.input)).toEqual(['5H', '5L', '5M']);
  });

  it('sorts by string column descending', () => {
    const sorted = sortMoves(moves, 'input', 'desc');
    expect(sorted.map((m) => m.input)).toEqual(['5M', '5L', '5H']);
  });

  it('sorts by numeric column ascending', () => {
    const sorted = sortMoves(moves, 'startup', 'asc');
    expect(sorted.map((m) => m.input)).toEqual(['5L', '5M', '5H']);
  });

  it('sorts by numeric column descending', () => {
    const sorted = sortMoves(moves, 'damage', 'desc');
    expect(sorted.map((m) => m.input)).toEqual(['5H', '5M', '5L']);
  });

  it('sorts by computed total column', () => {
    const sorted = sortMoves(moves, 'total', 'asc');
    // All use default recovery=10, active=2, so total = startup + 2 + 10
    expect(sorted.map((m) => m.input)).toEqual(['5L', '5M', '5H']);
  });

  it('sorts by computed advantage_hit column', () => {
    const testMoves = [
      makeState({ input: 'a', name: 'A', hitstun: 20, recovery: 10 }), // +10
      makeState({ input: 'b', name: 'B', hitstun: 5, recovery: 10 }),  // -5
      makeState({ input: 'c', name: 'C', hitstun: 10, recovery: 10 }), // 0
    ];
    const sorted = sortMoves(testMoves, 'advantage_hit', 'asc');
    expect(sorted.map((m) => m.input)).toEqual(['b', 'c', 'a']);
  });

  it('does not mutate original array', () => {
    const original = [...moves];
    sortMoves(moves, 'startup', 'asc');
    expect(moves.map((m) => m.input)).toEqual(original.map((m) => m.input));
  });
});

describe('filterMoves', () => {
  const moves = [
    makeState({ input: '5L', name: 'Light', type: 'normal' }),
    makeState({ input: '236P', name: 'Fireball', type: 'special' }),
    makeState({ input: '5M', name: 'Medium', type: 'normal' }),
  ];

  it('returns all moves when filter is "all"', () => {
    expect(filterMoves(moves, 'all')).toHaveLength(3);
  });

  it('filters normals', () => {
    const filtered = filterMoves(moves, 'normals');
    expect(filtered).toHaveLength(2);
    expect(filtered.every((m) => m.type === 'normal')).toBe(true);
  });

  it('filters specials', () => {
    const filtered = filterMoves(moves, 'specials');
    expect(filtered).toHaveLength(1);
    expect(filtered[0].input).toBe('236P');
  });

  it('uses registry filter groups', () => {
    const registry = {
      move_types: { filter_groups: { normals: ['normal', 'command_normal'] } },
    };
    const filtered = filterMoves(moves, 'normals', registry);
    expect(filtered).toHaveLength(2);
  });
});

describe('buildFilterOptions', () => {
  it('returns default options without registry', () => {
    const options = buildFilterOptions();
    expect(options).toEqual([
      { value: 'all', label: 'All Moves' },
      { value: 'normals', label: 'Normals' },
      { value: 'specials', label: 'Specials' },
    ]);
  });

  it('uses registry filter groups when available', () => {
    const registry = {
      move_types: {
        filter_groups: {
          normals: ['normal'],
          specials: ['special'],
          supers: ['super'],
        },
      },
    };
    const options = buildFilterOptions(registry);
    expect(options[0]).toEqual({ value: 'all', label: 'All Moves' });
    expect(options.length).toBe(4); // all + 3 groups
    expect(options.find((o) => o.value === 'supers')).toEqual({
      value: 'supers',
      label: 'Supers',
    });
  });
});
