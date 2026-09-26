/**
 * Integration tests for the cancel system.
 *
 * These tests verify that the cancel table structure and MoveResolver
 * work together correctly to allow state transitions from idle.
 *
 * The cancel system has two layers:
 * 1. MoveResolver - determines which move the player input matches
 * 2. WASM Runtime (can_cancel_to) - determines if that move is allowed from current state
 *
 * Key insight: MoveResolver filters against the state indices returned by
 * available_cancels(), while the WASM runtime remains the authority for
 * tag-rule cancel validity.
 */
import { describe, it, expect } from 'vitest';
import { MoveResolver, type MoveList } from './MoveResolver';
import { InputBuffer } from './InputBuffer';

/**
 * Mock cancel table structure matching characters/test_char/cancel_table.json
 */
interface CancelTable {
  tag_rules: Array<{
    from: string;
    to: string;
    on: 'always' | 'hit' | 'block' | 'whiff' | Array<'hit' | 'block' | 'whiff'>;
  }>;
  deny: Record<string, string[]>;
}

/**
 * Test move list simulating the test character's states.
 * Indices must match the actual state order in the character pack.
 */
function createTestMoveList(): MoveList {
  return {
    moves: [
      // Index 0: Idle (system state)
      { name: '0_idle', input: { type: 'simple', direction: null, button: 'L' }, priority: -1000 },
      // Index 1: Crouch (system state)
      { name: '1_crouch', input: { type: 'simple', direction: null, button: 'L' }, priority: -1000 },
      // Index 2: Standing light (5L) - first attack
      { name: '5L', input: { type: 'simple', direction: 5, button: 'L' }, priority: 0 },
      // Index 3: Standing medium (5M)
      { name: '5M', input: { type: 'simple', direction: 5, button: 'M' }, priority: 0 },
      // Index 4: Standing heavy (5H)
      { name: '5H', input: { type: 'simple', direction: 5, button: 'H' }, priority: 0 },
      // Index 5: Crouching light (2L)
      { name: '2L', input: { type: 'simple', direction: 2, button: 'L' }, priority: 0 },
      // Index 6: Forward medium (6M)
      { name: '6M', input: { type: 'simple', direction: 6, button: 'M' }, priority: 1 },
      // Index 7: Fireball (236P) - special
      { name: '236P', input: { type: 'motion', sequence: [2, 3, 6], button: 'P' }, priority: 10 },
    ],
    moveNameToIndex: new Map([
      ['0_idle', 0],
      ['1_crouch', 1],
      ['5L', 2],
      ['5M', 3],
      ['5H', 4],
      ['2L', 5],
      ['6M', 6],
      ['236P', 7],
    ]),
  };
}

describe('Cancel System Integration', () => {
  describe('MoveResolver behavior with empty availableCancels', () => {
    it('allows all moves when availableCancels is empty (neutral state)', () => {
      const resolver = new MoveResolver(createTestMoveList());
      const buffer = new InputBuffer();

      // Press L in neutral (direction 5)
      buffer.push({ direction: 5, buttons: ['L'] });

      // Empty cancels array = neutral state, all moves should be allowed
      const result = resolver.resolve(buffer, ['L'], []);

      // MoveResolver should return 5L because empty array means "allow all"
      expect(result).not.toBeNull();
      expect(result?.name).toBe('5L');
      expect(result?.index).toBe(2);
    });

    it('allows all moves when availableCancels is undefined (neutral state)', () => {
      const resolver = new MoveResolver(createTestMoveList());
      const buffer = new InputBuffer();

      buffer.push({ direction: 5, buttons: ['L'] });

      // Undefined cancels = neutral state
      const result = resolver.resolve(buffer, ['L']);

      expect(result).not.toBeNull();
      expect(result?.name).toBe('5L');
    });

    it('restricts to specific moves when availableCancels has values', () => {
      const resolver = new MoveResolver(createTestMoveList());
      const buffer = new InputBuffer();

      buffer.push({ direction: 5, buttons: ['L'] });

      // Only allow cancel to index 5 (2L), not index 2 (5L)
      const result = resolver.resolve(buffer, ['L'], [5]);

      // Should not match because 5L (index 2) is not in the cancel list
      expect(result).toBeNull();
    });
  });

  describe('Cancel table structure validation', () => {
    it('requires tag_rules for idle-to-attack transitions', () => {
      // This test documents the expected cancel table structure
      const validCancelTable: CancelTable = {
        tag_rules: [
          { from: 'system', to: 'any', on: 'always' },
          { from: 'normal', to: 'special', on: ['hit', 'block'] },
          { from: 'normal', to: 'super', on: ['hit', 'block'] },
          { from: 'special', to: 'super', on: ['hit', 'block'] },
        ],
        deny: {},
      };

      // tag_rules must exist and include system -> any rule
      expect(validCancelTable.tag_rules).toBeDefined();
      expect(validCancelTable.tag_rules).toContainEqual({
        from: 'system',
        to: 'any',
        on: 'always',
      });
    });

    it('documents available_cancels as the enumerated can_cancel_to state targets', () => {
      /**
       * available_cancels() enumerates regular move/state targets accepted by
       * can_cancel_to() under the current tag-rule model.
       *
       * For idle (state 0, type "system"), a "system -> any" rule should make
       * the attack state indices available. MoveResolver then restricts input
       * matching to that explicit list instead of relying on legacy chain fields.
       */

      const resolver = new MoveResolver(createTestMoveList());
      const buffer = new InputBuffer();

      // Simulate player in idle pressing L
      buffer.push({ direction: 5, buttons: ['L'] });

      // Idle can cancel to all attack states through the system -> any tag rule.
      const availableCancels: number[] = [2, 3, 4, 5, 6, 7];

      // MoveResolver allows the input to be resolved
      const resolved = resolver.resolve(buffer, ['L'], availableCancels);
      expect(resolved?.name).toBe('5L');

      // The WASM runtime's can_cancel_to() would then check tag_rules
      // and allow the transition because of "system -> any on always"
    });
  });

  describe('Move resolution from different states', () => {
    it('resolves 5L from idle state', () => {
      const resolver = new MoveResolver(createTestMoveList());
      const buffer = new InputBuffer();

      buffer.push({ direction: 5, buttons: ['L'] });

      // Idle has no explicit cancels, so empty array
      const result = resolver.resolve(buffer, ['L'], []);

      expect(result?.name).toBe('5L');
      expect(result?.index).toBe(2);
    });

    it('resolves 2L from crouching', () => {
      const resolver = new MoveResolver(createTestMoveList());
      const buffer = new InputBuffer();

      buffer.push({ direction: 2, buttons: ['L'] });

      const result = resolver.resolve(buffer, ['L'], []);

      expect(result?.name).toBe('2L');
      expect(result?.index).toBe(5);
    });

    it('resolves 236P motion input', () => {
      const resolver = new MoveResolver(createTestMoveList());
      const buffer = new InputBuffer();

      buffer.push({ direction: 2, buttons: [] });
      buffer.push({ direction: 3, buttons: [] });
      buffer.push({ direction: 6, buttons: ['P'] });

      const result = resolver.resolve(buffer, ['P'], []);

      expect(result?.name).toBe('236P');
      expect(result?.index).toBe(7);
    });

    it('respects explicit chain cancels during combo', () => {
      const resolver = new MoveResolver(createTestMoveList());
      const buffer = new InputBuffer();

      buffer.push({ direction: 5, buttons: ['L'] });

      // During 5L, can only cancel to the runtime's enumerated tag-rule targets.
      // Indices: 5L=2, 5M=3, 2L=5
      const result = resolver.resolve(buffer, ['L'], [2, 3, 5]);

      expect(result?.name).toBe('5L');
      expect(result?.index).toBe(2);
    });

    it('blocks moves not in chain list', () => {
      const resolver = new MoveResolver(createTestMoveList());
      const buffer = new InputBuffer();

      buffer.push({ direction: 5, buttons: ['H'] });

      // During 5L, 5H is not in the available cancel list.
      // Only allow indices 2, 3, 5 (5L, 5M, 2L)
      const result = resolver.resolve(buffer, ['H'], [2, 3, 5]);

      // 5H (index 4) is not allowed, should return null
      expect(result).toBeNull();
    });
  });
});
