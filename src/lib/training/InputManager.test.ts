/**
 * Tests for InputManager - tracks held keys and converts to numpad + buttons.
 */
import { describe, it, expect, beforeEach } from 'vitest';
import { InputManager, type TrainingInputConfig } from './InputManager.svelte';

describe('InputManager', () => {
  let manager: InputManager;
  const defaultConfig: TrainingInputConfig = {
    directions: {
      up: 'KeyW',
      down: 'KeyS',
      left: 'KeyA',
      right: 'KeyD',
    },
    buttons: {
      L: 'KeyU',
      M: 'KeyI',
      H: 'KeyO',
      P: 'KeyJ',
      K: 'KeyK',
      S: 'KeyL',
    },
  };

  beforeEach(() => {
    manager = new InputManager(defaultConfig);
  });

  describe('direction conversion to numpad', () => {
    it('should return 5 (neutral) when no directions are pressed', () => {
      expect(manager.currentDirection).toBe(5);
    });

    it('should return 8 (up) when up is pressed', () => {
      manager.handleKeyDown('KeyW');
      expect(manager.currentDirection).toBe(8);
    });

    it('should return 2 (down) when down is pressed', () => {
      manager.handleKeyDown('KeyS');
      expect(manager.currentDirection).toBe(2);
    });

    it('should return 4 (left/back) when left is pressed', () => {
      manager.handleKeyDown('KeyA');
      expect(manager.currentDirection).toBe(4);
    });

    it('should return 6 (right/forward) when right is pressed', () => {
      manager.handleKeyDown('KeyD');
      expect(manager.currentDirection).toBe(6);
    });

    it('should return 9 (up-forward) when up+right are pressed', () => {
      manager.handleKeyDown('KeyW');
      manager.handleKeyDown('KeyD');
      expect(manager.currentDirection).toBe(9);
    });

    it('should return 7 (up-back) when up+left are pressed', () => {
      manager.handleKeyDown('KeyW');
      manager.handleKeyDown('KeyA');
      expect(manager.currentDirection).toBe(7);
    });

    it('should return 3 (down-forward) when down+right are pressed', () => {
      manager.handleKeyDown('KeyS');
      manager.handleKeyDown('KeyD');
      expect(manager.currentDirection).toBe(3);
    });

    it('should return 1 (down-back) when down+left are pressed', () => {
      manager.handleKeyDown('KeyS');
      manager.handleKeyDown('KeyA');
      expect(manager.currentDirection).toBe(1);
    });

    it('should return 5 when opposing directions cancel out (up+down)', () => {
      manager.handleKeyDown('KeyW');
      manager.handleKeyDown('KeyS');
      expect(manager.currentDirection).toBe(5);
    });

    it('should return 5 when opposing directions cancel out (left+right)', () => {
      manager.handleKeyDown('KeyA');
      manager.handleKeyDown('KeyD');
      expect(manager.currentDirection).toBe(5);
    });

    it('should return 8 when up+down+right pressed (vertical cancels, only right)', () => {
      manager.handleKeyDown('KeyW');
      manager.handleKeyDown('KeyS');
      manager.handleKeyDown('KeyD');
      expect(manager.currentDirection).toBe(6);
    });
  });

  describe('facing direction mirroring', () => {
    it('should mirror left/right when facing left', () => {
      manager.setFacingRight(false);
      manager.handleKeyDown('KeyD'); // Physical right
      expect(manager.currentDirection).toBe(4); // Becomes back (4)
    });

    it('should not mirror when facing right (default)', () => {
      manager.setFacingRight(true);
      manager.handleKeyDown('KeyD');
      expect(manager.currentDirection).toBe(6); // Forward
    });

    it('should mirror diagonals when facing left', () => {
      manager.setFacingRight(false);
      manager.handleKeyDown('KeyS'); // Down
      manager.handleKeyDown('KeyD'); // Physical right
      expect(manager.currentDirection).toBe(1); // Down-back (1)
    });
  });

  describe('button tracking', () => {
    it('should track pressed buttons', () => {
      manager.handleKeyDown('KeyU'); // L
      expect(manager.currentButtons).toContain('L');
    });

    it('should track multiple pressed buttons', () => {
      manager.handleKeyDown('KeyU'); // L
      manager.handleKeyDown('KeyI'); // M
      expect(manager.currentButtons).toContain('L');
      expect(manager.currentButtons).toContain('M');
    });

    it('should release buttons on key up', () => {
      manager.handleKeyDown('KeyU');
      expect(manager.currentButtons).toContain('L');
      manager.handleKeyUp('KeyU');
      expect(manager.currentButtons).not.toContain('L');
    });

    it('should return empty array when no buttons pressed', () => {
      expect(manager.currentButtons).toEqual([]);
    });
  });

  describe('key release', () => {
    it('should update direction when key is released', () => {
      manager.handleKeyDown('KeyW');
      manager.handleKeyDown('KeyD');
      expect(manager.currentDirection).toBe(9); // Up-forward
      manager.handleKeyUp('KeyW');
      expect(manager.currentDirection).toBe(6); // Forward only
    });

    it('should return to neutral when all direction keys released', () => {
      manager.handleKeyDown('KeyW');
      manager.handleKeyDown('KeyD');
      manager.handleKeyUp('KeyW');
      manager.handleKeyUp('KeyD');
      expect(manager.currentDirection).toBe(5);
    });
  });

  describe('snapshot generation', () => {
    it('should generate input snapshot', () => {
      manager.handleKeyDown('KeyS'); // Down
      manager.handleKeyDown('KeyD'); // Right
      manager.handleKeyDown('KeyJ'); // P

      const snapshot = manager.getSnapshot();
      expect(snapshot.direction).toBe(3); // Down-forward
      expect(snapshot.buttons).toContain('P');
    });

    it('should generate empty snapshot when no input', () => {
      const snapshot = manager.getSnapshot();
      expect(snapshot.direction).toBe(5);
      expect(snapshot.buttons).toEqual([]);
    });
  });

  describe('newly pressed buttons', () => {
    it('should track newly pressed buttons for motion detection', () => {
      manager.handleKeyDown('KeyJ'); // P
      expect(manager.newlyPressedButtons).toContain('P');

      // After consuming, should be empty
      manager.consumeNewlyPressed();
      expect(manager.newlyPressedButtons).toEqual([]);
    });

    it('should not re-add held buttons as newly pressed', () => {
      manager.handleKeyDown('KeyJ');
      manager.consumeNewlyPressed();

      // Key still held, no new key down event
      expect(manager.newlyPressedButtons).toEqual([]);
    });

    it('should detect re-press after release', () => {
      manager.handleKeyDown('KeyJ');
      manager.consumeNewlyPressed();
      manager.handleKeyUp('KeyJ');
      manager.handleKeyDown('KeyJ');
      expect(manager.newlyPressedButtons).toContain('P');
    });
  });

  describe('reset', () => {
    it('should reset all state', () => {
      manager.handleKeyDown('KeyW');
      manager.handleKeyDown('KeyU');
      manager.reset();

      expect(manager.currentDirection).toBe(5);
      expect(manager.currentButtons).toEqual([]);
      expect(manager.newlyPressedButtons).toEqual([]);
    });
  });

  describe('config update', () => {
    it('should accept new configuration', () => {
      const newConfig: TrainingInputConfig = {
        directions: {
          up: 'ArrowUp',
          down: 'ArrowDown',
          left: 'ArrowLeft',
          right: 'ArrowRight',
        },
        buttons: {
          L: 'KeyZ',
          M: 'KeyX',
          H: 'KeyC',
          P: 'KeyA',
          K: 'KeyS',
          S: 'KeyD',
        },
      };

      manager.setConfig(newConfig);
      manager.handleKeyDown('ArrowUp');
      expect(manager.currentDirection).toBe(8);

      manager.handleKeyDown('KeyZ');
      expect(manager.currentButtons).toContain('L');
    });
  });

  describe('ignores unbound keys', () => {
    it('should ignore keys not in config', () => {
      manager.handleKeyDown('KeyQ'); // Not bound
      expect(manager.currentDirection).toBe(5);
      expect(manager.currentButtons).toEqual([]);
    });
  });
});
