/**
 * Tests for frame advantage calculation utilities.
 */
import { describe, it, expect } from 'vitest';
import {
  calculateFrameAdvantage,
  calculateSimpleFrameAdvantage,
  formatFrameAdvantage,
  type FrameAdvantageInput,
} from './FrameAdvantage';

describe('calculateFrameAdvantage', () => {
  it('should calculate positive advantage on hit', () => {
    // A fast jab: startup 5, active 2, recovery 8
    // If it causes 20 hitstun, attacker recovers with 11 frames advantage
    // After hit on frame 1: attacker has 1 + 8 = 9 frames remaining
    // Defender has 20 frames of hitstun
    // Advantage = 20 - 9 = 11
    const input: FrameAdvantageInput = {
      startup: 5,
      active: 2,
      recovery: 8,
      hitstun: 20,
      blockstun: 10,
    };

    const result = calculateFrameAdvantage(input);

    expect(result.onHit).toBe(11); // 20 - (1 + 8) = 11
    expect(result.onBlock).toBe(1); // 10 - (1 + 8) = 1
  });

  it('should calculate negative advantage on block', () => {
    // A slow heavy: startup 20, active 3, recovery 25
    // If it causes 12 blockstun, defender recovers before attacker
    // After hit on frame 1: attacker has 2 + 25 = 27 frames remaining
    // Defender has 12 frames of blockstun
    // Advantage = 12 - 27 = -15
    const input: FrameAdvantageInput = {
      startup: 20,
      active: 3,
      recovery: 25,
      hitstun: 30,
      blockstun: 12,
    };

    const result = calculateFrameAdvantage(input);

    expect(result.onHit).toBe(3); // 30 - (2 + 25) = 3
    expect(result.onBlock).toBe(-15); // 12 - (2 + 25) = -15
  });

  it('should handle single active frame', () => {
    // Move with exactly 1 active frame
    const input: FrameAdvantageInput = {
      startup: 7,
      active: 1,
      recovery: 10,
      hitstun: 15,
      blockstun: 8,
    };

    const result = calculateFrameAdvantage(input);

    // With 1 active frame, remaining = 0 + 10 = 10
    expect(result.onHit).toBe(5); // 15 - 10 = 5
    expect(result.onBlock).toBe(-2); // 8 - 10 = -2
  });

  it('should handle zero frame advantage (even)', () => {
    // A perfectly neutral move
    const input: FrameAdvantageInput = {
      startup: 5,
      active: 2,
      recovery: 10,
      hitstun: 11,
      blockstun: 11,
    };

    const result = calculateFrameAdvantage(input);

    // Remaining = 1 + 10 = 11
    expect(result.onHit).toBe(0); // 11 - 11 = 0
    expect(result.onBlock).toBe(0); // 11 - 11 = 0
  });

  it('should handle very high hitstun (launcher)', () => {
    // A launcher with huge hitstun
    const input: FrameAdvantageInput = {
      startup: 15,
      active: 4,
      recovery: 30,
      hitstun: 60, // Long juggle state
      blockstun: 15,
    };

    const result = calculateFrameAdvantage(input);

    // Remaining = 3 + 30 = 33
    expect(result.onHit).toBe(27); // 60 - 33 = 27
    expect(result.onBlock).toBe(-18); // 15 - 33 = -18
  });
});

describe('calculateSimpleFrameAdvantage', () => {
  it('should calculate simple advantage using only recovery', () => {
    const result = calculateSimpleFrameAdvantage(10, 15, 8);

    expect(result.onHit).toBe(5); // 15 - 10 = 5
    expect(result.onBlock).toBe(-2); // 8 - 10 = -2
  });

  it('should handle zero recovery', () => {
    const result = calculateSimpleFrameAdvantage(0, 10, 5);

    expect(result.onHit).toBe(10);
    expect(result.onBlock).toBe(5);
  });

  it('should handle equal values', () => {
    const result = calculateSimpleFrameAdvantage(12, 12, 12);

    expect(result.onHit).toBe(0);
    expect(result.onBlock).toBe(0);
  });
});

describe('formatFrameAdvantage', () => {
  it('should add + prefix to positive values', () => {
    expect(formatFrameAdvantage(5)).toBe('+5');
    expect(formatFrameAdvantage(10)).toBe('+10');
    expect(formatFrameAdvantage(1)).toBe('+1');
  });

  it('should not add prefix to negative values', () => {
    expect(formatFrameAdvantage(-5)).toBe('-5');
    expect(formatFrameAdvantage(-10)).toBe('-10');
    expect(formatFrameAdvantage(-1)).toBe('-1');
  });

  it('should not add prefix to zero', () => {
    expect(formatFrameAdvantage(0)).toBe('0');
  });
});
