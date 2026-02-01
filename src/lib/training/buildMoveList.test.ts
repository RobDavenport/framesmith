import { describe, it, expect } from 'vitest';
import { MoveResolver } from './MoveResolver';
import { InputBuffer } from './InputBuffer';
import { buildMoveList } from './buildMoveList';

describe('buildMoveList', () => {
  it('preserves canonical indices from the input moves array', () => {
    const moves = [
      { input: '5L', type: 'normal' },
      { input: '236P', type: 'special' },
      { input: '5M', type: 'normal' },
    ];

    const list = buildMoveList(moves);

    expect(list.moves.map(m => m.name)).toEqual(['5L', '236P', '5M']);
    expect(list.moveNameToIndex.get('5L')).toBe(0);
    expect(list.moveNameToIndex.get('236P')).toBe(1);
    expect(list.moveNameToIndex.get('5M')).toBe(2);
  });

  it('keeps priority-based resolution without changing indices', () => {
    const moves = [
      { input: '5P', type: 'normal' },
      { input: '236P', type: 'special' },
    ];

    const resolver = new MoveResolver(buildMoveList(moves));
    const buffer = new InputBuffer();
    buffer.push({ direction: 2, buttons: [] });
    buffer.push({ direction: 3, buttons: [] });
    buffer.push({ direction: 6, buttons: ['P'] });

    const result = resolver.resolve(buffer, ['P']);
    expect(result?.name).toBe('236P');
    expect(result?.index).toBe(1);
  });

  it('includes unparseable moves to preserve indices but they never match input', () => {
    const moves = [{ input: 'Throw', type: 'special' }];
    const resolver = new MoveResolver(buildMoveList(moves));
    const buffer = new InputBuffer();
    buffer.push({ direction: 5, buttons: ['L'] });

    expect(resolver.getMove(0)?.name).toBe('Throw');
    expect(resolver.getMoveIndex('Throw')).toBe(0);
    expect(resolver.resolve(buffer, ['L'])).toBeNull();
  });

  it('treats motion inputs containing 0 as unparseable (index preserved)', () => {
    const moves = [{ input: '2360P', type: 'special' }];
    const list = buildMoveList(moves);

    expect(list.moveNameToIndex.get('2360P')).toBe(0);
    expect(list.moves[0]?.name).toBe('2360P');
    expect(list.moves[0]?.input.type).not.toBe('motion');
  });
});
