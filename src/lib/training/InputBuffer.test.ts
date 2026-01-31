/**
 * Tests for InputBuffer - stores recent inputs and detects motion sequences.
 */
import { describe, it, expect, beforeEach } from 'vitest';
import { InputBuffer, type InputSnapshot, type MotionPattern } from './InputBuffer';

describe('InputBuffer', () => {
  let buffer: InputBuffer;

  beforeEach(() => {
    buffer = new InputBuffer();
  });

  describe('basic input recording', () => {
    it('should record input snapshots', () => {
      buffer.push({ direction: 5, buttons: [] });
      buffer.push({ direction: 6, buttons: [] });
      expect(buffer.length).toBe(2);
    });

    it('should have a maximum capacity', () => {
      // Push more than capacity
      for (let i = 0; i < 100; i++) {
        buffer.push({ direction: 5, buttons: [] });
      }
      expect(buffer.length).toBeLessThanOrEqual(60); // Default 1 second at 60fps
    });

    it('should clear the buffer', () => {
      buffer.push({ direction: 5, buttons: [] });
      buffer.push({ direction: 6, buttons: [] });
      buffer.clear();
      expect(buffer.length).toBe(0);
    });

    it('should return the latest input', () => {
      buffer.push({ direction: 5, buttons: [] });
      buffer.push({ direction: 6, buttons: ['L'] });
      const latest = buffer.latest();
      expect(latest?.direction).toBe(6);
      expect(latest?.buttons).toContain('L');
    });

    it('should return null for latest when empty', () => {
      expect(buffer.latest()).toBeNull();
    });
  });

  describe('numpad direction values', () => {
    it('should accept all 9 numpad directions', () => {
      // All valid numpad directions
      const directions = [1, 2, 3, 4, 5, 6, 7, 8, 9];
      for (const dir of directions) {
        buffer.push({ direction: dir, buttons: [] });
      }
      expect(buffer.length).toBe(9);
    });
  });

  describe('motion detection - quarter circle forward (236)', () => {
    it('should detect 236 motion ending in button press', () => {
      // Down
      buffer.push({ direction: 2, buttons: [] });
      // Down-forward
      buffer.push({ direction: 3, buttons: [] });
      // Forward + button
      buffer.push({ direction: 6, buttons: ['P'] });

      const match = buffer.detectMotion({ sequence: [2, 3, 6], button: 'P' });
      expect(match).toBe(true);
    });

    it('should detect 236 with intermediate frames', () => {
      // Down for a few frames
      buffer.push({ direction: 2, buttons: [] });
      buffer.push({ direction: 2, buttons: [] });
      buffer.push({ direction: 2, buttons: [] });
      // Down-forward
      buffer.push({ direction: 3, buttons: [] });
      buffer.push({ direction: 3, buttons: [] });
      // Forward + button
      buffer.push({ direction: 6, buttons: ['P'] });

      const match = buffer.detectMotion({ sequence: [2, 3, 6], button: 'P' });
      expect(match).toBe(true);
    });

    it('should not detect 236 if button not pressed', () => {
      buffer.push({ direction: 2, buttons: [] });
      buffer.push({ direction: 3, buttons: [] });
      buffer.push({ direction: 6, buttons: [] });

      const match = buffer.detectMotion({ sequence: [2, 3, 6], button: 'P' });
      expect(match).toBe(false);
    });

    it('should not detect incomplete 236', () => {
      buffer.push({ direction: 2, buttons: [] });
      buffer.push({ direction: 6, buttons: ['P'] }); // Missing 3

      const match = buffer.detectMotion({ sequence: [2, 3, 6], button: 'P' });
      expect(match).toBe(false);
    });
  });

  describe('motion detection - quarter circle back (214)', () => {
    it('should detect 214 motion', () => {
      buffer.push({ direction: 2, buttons: [] });
      buffer.push({ direction: 1, buttons: [] });
      buffer.push({ direction: 4, buttons: ['P'] });

      const match = buffer.detectMotion({ sequence: [2, 1, 4], button: 'P' });
      expect(match).toBe(true);
    });
  });

  describe('motion detection - dragon punch (623)', () => {
    it('should detect 623 motion', () => {
      buffer.push({ direction: 6, buttons: [] });
      buffer.push({ direction: 2, buttons: [] });
      buffer.push({ direction: 3, buttons: ['P'] });

      const match = buffer.detectMotion({ sequence: [6, 2, 3], button: 'P' });
      expect(match).toBe(true);
    });

    it('should detect 623 with 6236 shortcut input', () => {
      // Common shortcut: 6236 instead of pure 623
      buffer.push({ direction: 6, buttons: [] });
      buffer.push({ direction: 2, buttons: [] });
      buffer.push({ direction: 3, buttons: [] });
      buffer.push({ direction: 6, buttons: ['P'] });

      // Should still match 623 pattern (6 appears twice but sequence exists)
      const match = buffer.detectMotion({ sequence: [6, 2, 3], button: 'P' });
      expect(match).toBe(true);
    });
  });

  describe('motion detection - half circle (41236)', () => {
    it('should detect half circle forward motion', () => {
      buffer.push({ direction: 4, buttons: [] });
      buffer.push({ direction: 1, buttons: [] });
      buffer.push({ direction: 2, buttons: [] });
      buffer.push({ direction: 3, buttons: [] });
      buffer.push({ direction: 6, buttons: ['P'] });

      const match = buffer.detectMotion({ sequence: [4, 1, 2, 3, 6], button: 'P' });
      expect(match).toBe(true);
    });
  });

  describe('motion detection - charge motions', () => {
    it('should detect charge back forward (4]6) motion', () => {
      // Hold back for charge time
      for (let i = 0; i < 30; i++) {
        buffer.push({ direction: 4, buttons: [] });
      }
      // Forward + button
      buffer.push({ direction: 6, buttons: ['P'] });

      const match = buffer.detectCharge({ holdDirection: 4, releaseDirection: 6, chargeFrames: 30, button: 'P' });
      expect(match).toBe(true);
    });

    it('should not detect charge if not held long enough', () => {
      // Hold back for only a few frames
      for (let i = 0; i < 10; i++) {
        buffer.push({ direction: 4, buttons: [] });
      }
      buffer.push({ direction: 6, buttons: ['P'] });

      const match = buffer.detectCharge({ holdDirection: 4, releaseDirection: 6, chargeFrames: 30, button: 'P' });
      expect(match).toBe(false);
    });

    it('should detect charge down up (2]8)', () => {
      for (let i = 0; i < 30; i++) {
        buffer.push({ direction: 2, buttons: [] });
      }
      buffer.push({ direction: 8, buttons: ['K'] });

      const match = buffer.detectCharge({ holdDirection: 2, releaseDirection: 8, chargeFrames: 30, button: 'K' });
      expect(match).toBe(true);
    });
  });

  describe('simple direction + button detection', () => {
    it('should detect standing button (5+button)', () => {
      buffer.push({ direction: 5, buttons: ['L'] });

      const match = buffer.detectSimple({ direction: 5, button: 'L' });
      expect(match).toBe(true);
    });

    it('should detect crouching button (2+button)', () => {
      buffer.push({ direction: 2, buttons: ['L'] });

      const match = buffer.detectSimple({ direction: 2, button: 'L' });
      expect(match).toBe(true);
    });

    it('should detect forward button (6+button)', () => {
      buffer.push({ direction: 6, buttons: ['M'] });

      const match = buffer.detectSimple({ direction: 6, button: 'M' });
      expect(match).toBe(true);
    });

    it('should not match if direction is wrong', () => {
      buffer.push({ direction: 5, buttons: ['L'] });

      const match = buffer.detectSimple({ direction: 2, button: 'L' });
      expect(match).toBe(false);
    });

    it('should not match if button is wrong', () => {
      buffer.push({ direction: 5, buttons: ['L'] });

      const match = buffer.detectSimple({ direction: 5, button: 'M' });
      expect(match).toBe(false);
    });

    it('should detect any direction with null direction pattern', () => {
      buffer.push({ direction: 3, buttons: ['L'] });

      const match = buffer.detectSimple({ direction: null, button: 'L' });
      expect(match).toBe(true);
    });
  });

  describe('buffer timing', () => {
    it('should respect motion window for detection', () => {
      // Push motion inputs
      buffer.push({ direction: 2, buttons: [] });
      buffer.push({ direction: 3, buttons: [] });

      // Push many neutral inputs to expire the motion
      for (let i = 0; i < 30; i++) {
        buffer.push({ direction: 5, buttons: [] });
      }

      // Now press button
      buffer.push({ direction: 6, buttons: ['P'] });

      // Motion should be too old
      const match = buffer.detectMotion({ sequence: [2, 3, 6], button: 'P', windowFrames: 15 });
      expect(match).toBe(false);
    });

    it('should use default window if not specified', () => {
      buffer.push({ direction: 2, buttons: [] });
      buffer.push({ direction: 3, buttons: [] });
      buffer.push({ direction: 6, buttons: ['P'] });

      const match = buffer.detectMotion({ sequence: [2, 3, 6], button: 'P' });
      expect(match).toBe(true);
    });
  });

  describe('multiple button detection', () => {
    it('should detect multiple buttons pressed simultaneously', () => {
      buffer.push({ direction: 5, buttons: ['L', 'M'] });

      expect(buffer.detectSimple({ direction: 5, button: 'L' })).toBe(true);
      expect(buffer.detectSimple({ direction: 5, button: 'M' })).toBe(true);
    });

    it('should detect motion with any matching button', () => {
      buffer.push({ direction: 2, buttons: [] });
      buffer.push({ direction: 3, buttons: [] });
      buffer.push({ direction: 6, buttons: ['L', 'M'] });

      expect(buffer.detectMotion({ sequence: [2, 3, 6], button: 'L' })).toBe(true);
      expect(buffer.detectMotion({ sequence: [2, 3, 6], button: 'M' })).toBe(true);
    });
  });
});
