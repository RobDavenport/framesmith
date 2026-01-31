/**
 * Tests for MoveResolver - matches input buffer to move names and checks cancels.
 */
import { describe, it, expect, beforeEach } from 'vitest';
import { MoveResolver, type MoveDefinition, type MoveList } from './MoveResolver';
import { InputBuffer, type InputSnapshot } from './InputBuffer';

describe('MoveResolver', () => {
  let resolver: MoveResolver;
  let buffer: InputBuffer;

  // Sample move list for testing
  const testMoves: MoveList = {
    moves: [
      // Index 0: Standing light (5L)
      { name: '5L', input: { type: 'simple', direction: 5, button: 'L' }, priority: 0 },
      // Index 1: Crouching light (2L)
      { name: '2L', input: { type: 'simple', direction: 2, button: 'L' }, priority: 0 },
      // Index 2: Forward medium (6M)
      { name: '6M', input: { type: 'simple', direction: 6, button: 'M' }, priority: 1 },
      // Index 3: Standing medium (5M)
      { name: '5M', input: { type: 'simple', direction: 5, button: 'M' }, priority: 0 },
      // Index 4: Fireball (236P)
      { name: '236P', input: { type: 'motion', sequence: [2, 3, 6], button: 'P' }, priority: 10 },
      // Index 5: Dragon punch (623P)
      { name: '623P', input: { type: 'motion', sequence: [6, 2, 3], button: 'P' }, priority: 15 },
      // Index 6: Standing punch (5P) - lower priority than motions
      { name: '5P', input: { type: 'simple', direction: 5, button: 'P' }, priority: 0 },
      // Index 7: Charge move (4]6P)
      { name: 'SonicBoom', input: { type: 'charge', holdDirection: 4, releaseDirection: 6, chargeFrames: 30, button: 'P' }, priority: 12 },
      // Index 8: Super (236236P)
      { name: 'Super', input: { type: 'motion', sequence: [2, 3, 6, 2, 3, 6], button: 'P' }, priority: 20 },
    ],
    moveNameToIndex: new Map([
      ['5L', 0],
      ['2L', 1],
      ['6M', 2],
      ['5M', 3],
      ['236P', 4],
      ['623P', 5],
      ['5P', 6],
      ['SonicBoom', 7],
      ['Super', 8],
    ]),
  };

  beforeEach(() => {
    resolver = new MoveResolver(testMoves);
    buffer = new InputBuffer();
  });

  describe('simple move resolution', () => {
    it('should resolve standing light (5L)', () => {
      buffer.push({ direction: 5, buttons: ['L'] });
      const result = resolver.resolve(buffer, ['L']);
      expect(result).not.toBeNull();
      expect(result?.name).toBe('5L');
      expect(result?.index).toBe(0);
    });

    it('should resolve crouching light (2L)', () => {
      buffer.push({ direction: 2, buttons: ['L'] });
      const result = resolver.resolve(buffer, ['L']);
      expect(result?.name).toBe('2L');
    });

    it('should resolve forward medium (6M)', () => {
      buffer.push({ direction: 6, buttons: ['M'] });
      const result = resolver.resolve(buffer, ['M']);
      expect(result?.name).toBe('6M');
    });

    it('should return null when no move matches', () => {
      buffer.push({ direction: 5, buttons: ['H'] }); // No 5H defined
      const result = resolver.resolve(buffer, ['H']);
      expect(result).toBeNull();
    });

    it('should return null when no button is pressed', () => {
      buffer.push({ direction: 5, buttons: [] });
      const result = resolver.resolve(buffer, []);
      expect(result).toBeNull();
    });
  });

  describe('motion move resolution', () => {
    it('should resolve quarter circle forward (236P)', () => {
      buffer.push({ direction: 2, buttons: [] });
      buffer.push({ direction: 3, buttons: [] });
      buffer.push({ direction: 6, buttons: ['P'] });

      const result = resolver.resolve(buffer, ['P']);
      expect(result?.name).toBe('236P');
    });

    it('should resolve dragon punch (623P)', () => {
      buffer.push({ direction: 6, buttons: [] });
      buffer.push({ direction: 2, buttons: [] });
      buffer.push({ direction: 3, buttons: ['P'] });

      const result = resolver.resolve(buffer, ['P']);
      expect(result?.name).toBe('623P');
    });

    it('should resolve double quarter circle super (236236P)', () => {
      buffer.push({ direction: 2, buttons: [] });
      buffer.push({ direction: 3, buttons: [] });
      buffer.push({ direction: 6, buttons: [] });
      buffer.push({ direction: 2, buttons: [] });
      buffer.push({ direction: 3, buttons: [] });
      buffer.push({ direction: 6, buttons: ['P'] });

      const result = resolver.resolve(buffer, ['P']);
      expect(result?.name).toBe('Super');
    });
  });

  describe('priority resolution', () => {
    it('should prefer higher priority move (motion over simple)', () => {
      // Input 236+P - should match 236P, not 5P
      buffer.push({ direction: 2, buttons: [] });
      buffer.push({ direction: 3, buttons: [] });
      buffer.push({ direction: 6, buttons: ['P'] });

      const result = resolver.resolve(buffer, ['P']);
      expect(result?.name).toBe('236P');
    });

    it('should prefer super (236236) over single motion (236)', () => {
      buffer.push({ direction: 2, buttons: [] });
      buffer.push({ direction: 3, buttons: [] });
      buffer.push({ direction: 6, buttons: [] });
      buffer.push({ direction: 2, buttons: [] });
      buffer.push({ direction: 3, buttons: [] });
      buffer.push({ direction: 6, buttons: ['P'] });

      const result = resolver.resolve(buffer, ['P']);
      expect(result?.name).toBe('Super'); // Priority 20 > 10
    });

    it('should prefer DP (623) over fireball (236) when both match', () => {
      // 6236 input can match both 236 and 623
      buffer.push({ direction: 6, buttons: [] });
      buffer.push({ direction: 2, buttons: [] });
      buffer.push({ direction: 3, buttons: [] });
      buffer.push({ direction: 6, buttons: ['P'] });

      const result = resolver.resolve(buffer, ['P']);
      expect(result?.name).toBe('623P'); // Priority 15 > 10
    });

    it('should prefer forward normal (6M) over standing normal (5M)', () => {
      buffer.push({ direction: 6, buttons: ['M'] });

      const result = resolver.resolve(buffer, ['M']);
      expect(result?.name).toBe('6M'); // Priority 1 > 0
    });
  });

  describe('charge move resolution', () => {
    it('should resolve charge move when properly charged', () => {
      // Hold back for 30 frames
      for (let i = 0; i < 30; i++) {
        buffer.push({ direction: 4, buttons: [] });
      }
      // Release forward + button
      buffer.push({ direction: 6, buttons: ['P'] });

      const result = resolver.resolve(buffer, ['P']);
      expect(result?.name).toBe('SonicBoom');
    });

    it('should fall back to simple move when not fully charged', () => {
      // Only hold back for 10 frames (not enough)
      for (let i = 0; i < 10; i++) {
        buffer.push({ direction: 4, buttons: [] });
      }
      buffer.push({ direction: 6, buttons: ['P'] });

      const result = resolver.resolve(buffer, ['P']);
      // Should not get SonicBoom, might get 5P or nothing depending on buffer state
      expect(result?.name).not.toBe('SonicBoom');
    });
  });

  describe('cancel filtering', () => {
    it('should filter by available cancels', () => {
      // Press P in neutral - both 5P and potentially motions could match
      buffer.push({ direction: 5, buttons: ['P'] });

      // Only allow cancel to index 6 (5P), not 236P
      const result = resolver.resolve(buffer, ['P'], [6]);
      expect(result?.name).toBe('5P');
    });

    it('should return null if no matching move is in cancel list', () => {
      buffer.push({ direction: 5, buttons: ['L'] });

      // Only allow cancel to index 4 (236P)
      const result = resolver.resolve(buffer, ['L'], [4]);
      expect(result).toBeNull();
    });

    it('should check all moves when no cancel list provided', () => {
      buffer.push({ direction: 5, buttons: ['L'] });

      const result = resolver.resolve(buffer, ['L']);
      expect(result?.name).toBe('5L');
    });

    it('should still respect priority within available cancels', () => {
      buffer.push({ direction: 2, buttons: [] });
      buffer.push({ direction: 3, buttons: [] });
      buffer.push({ direction: 6, buttons: ['P'] });

      // Allow both 236P and 5P
      const result = resolver.resolve(buffer, ['P'], [4, 6]);
      expect(result?.name).toBe('236P'); // Higher priority
    });
  });

  describe('move index lookup', () => {
    it('should provide index for resolved move', () => {
      buffer.push({ direction: 5, buttons: ['L'] });
      const result = resolver.resolve(buffer, ['L']);
      expect(result?.index).toBe(0);
    });

    it('should look up move by name', () => {
      const index = resolver.getMoveIndex('236P');
      expect(index).toBe(4);
    });

    it('should return undefined for unknown move name', () => {
      const index = resolver.getMoveIndex('UnknownMove');
      expect(index).toBeUndefined();
    });
  });

  describe('getMatchingMoves', () => {
    it('should return all matching moves sorted by priority', () => {
      buffer.push({ direction: 2, buttons: [] });
      buffer.push({ direction: 3, buttons: [] });
      buffer.push({ direction: 6, buttons: ['P'] });

      const matches = resolver.getMatchingMoves(buffer, ['P']);
      expect(matches.length).toBeGreaterThan(0);
      // Verify sorted by priority (descending)
      for (let i = 0; i < matches.length - 1; i++) {
        expect(matches[i].priority).toBeGreaterThanOrEqual(matches[i + 1].priority);
      }
    });
  });
});
