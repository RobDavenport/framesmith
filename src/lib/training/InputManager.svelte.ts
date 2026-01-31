/**
 * InputManager - Tracks held keys and converts to numpad notation + buttons.
 *
 * This module handles keyboard input and converts it to fighting game notation.
 * It supports:
 * - WASD or arrow key direction mapping
 * - Configurable button mappings
 * - Facing direction mirroring
 * - Newly pressed button tracking for motion detection
 */

import type { InputSnapshot, ButtonName } from './InputBuffer';

/**
 * Configuration for training mode input mappings.
 */
export interface TrainingInputConfig {
  directions: {
    up: string;
    down: string;
    left: string;
    right: string;
  };
  buttons: Record<ButtonName, string>;
}

/**
 * InputManager tracks keyboard state and converts to fighting game notation.
 */
export class InputManager {
  private config: TrainingInputConfig;
  private heldKeys = new Set<string>();
  private _newlyPressedButtons: ButtonName[] = [];
  private facingRight = true;

  // Reverse mappings for fast lookup
  private keyToDirection = new Map<string, 'up' | 'down' | 'left' | 'right'>();
  private keyToButton = new Map<string, ButtonName>();

  constructor(config: TrainingInputConfig) {
    this.config = config;
    this.buildReverseMappings();
  }

  /**
   * Build reverse mappings from config for fast lookup.
   */
  private buildReverseMappings(): void {
    this.keyToDirection.clear();
    this.keyToButton.clear();

    // Direction mappings
    this.keyToDirection.set(this.config.directions.up, 'up');
    this.keyToDirection.set(this.config.directions.down, 'down');
    this.keyToDirection.set(this.config.directions.left, 'left');
    this.keyToDirection.set(this.config.directions.right, 'right');

    // Button mappings
    for (const [button, key] of Object.entries(this.config.buttons)) {
      this.keyToButton.set(key, button as ButtonName);
    }
  }

  /**
   * Set a new configuration.
   */
  setConfig(config: TrainingInputConfig): void {
    this.config = config;
    this.buildReverseMappings();
    this.reset();
  }

  /**
   * Set the facing direction for input mirroring.
   * When facing left, left/right inputs are swapped.
   */
  setFacingRight(facingRight: boolean): void {
    this.facingRight = facingRight;
  }

  /**
   * Handle a key down event.
   */
  handleKeyDown(code: string): void {
    if (this.heldKeys.has(code)) {
      return; // Already held, ignore repeat
    }

    this.heldKeys.add(code);

    // Track newly pressed buttons
    const button = this.keyToButton.get(code);
    if (button) {
      this._newlyPressedButtons.push(button);
    }
  }

  /**
   * Handle a key up event.
   */
  handleKeyUp(code: string): void {
    this.heldKeys.delete(code);
  }

  /**
   * Get the current numpad direction based on held keys.
   *
   * Returns 1-9 using numpad notation:
   * ```
   * 7 8 9
   * 4 5 6
   * 1 2 3
   * ```
   */
  get currentDirection(): number {
    let up = this.heldKeys.has(this.config.directions.up);
    let down = this.heldKeys.has(this.config.directions.down);
    let left = this.heldKeys.has(this.config.directions.left);
    let right = this.heldKeys.has(this.config.directions.right);

    // Cancel opposing directions
    if (up && down) {
      up = false;
      down = false;
    }
    if (left && right) {
      left = false;
      right = false;
    }

    // Mirror left/right when facing left
    if (!this.facingRight) {
      [left, right] = [right, left];
    }

    // Convert to numpad notation
    if (up) {
      if (left) return 7;
      if (right) return 9;
      return 8;
    }
    if (down) {
      if (left) return 1;
      if (right) return 3;
      return 2;
    }
    if (left) return 4;
    if (right) return 6;
    return 5; // Neutral
  }

  /**
   * Get the list of currently pressed buttons.
   */
  get currentButtons(): ButtonName[] {
    const buttons: ButtonName[] = [];
    for (const [key, button] of this.keyToButton) {
      if (this.heldKeys.has(key)) {
        buttons.push(button);
      }
    }
    return buttons;
  }

  /**
   * Get buttons that were newly pressed since last consume.
   */
  get newlyPressedButtons(): ButtonName[] {
    return [...this._newlyPressedButtons];
  }

  /**
   * Clear the newly pressed buttons list.
   * Call this after processing inputs each frame.
   */
  consumeNewlyPressed(): void {
    this._newlyPressedButtons = [];
  }

  /**
   * Get a snapshot of the current input state.
   */
  getSnapshot(): InputSnapshot {
    return {
      direction: this.currentDirection,
      buttons: this.currentButtons,
    };
  }

  /**
   * Reset all input state.
   */
  reset(): void {
    this.heldKeys.clear();
    this._newlyPressedButtons = [];
  }
}
