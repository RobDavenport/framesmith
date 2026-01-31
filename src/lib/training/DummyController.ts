/**
 * DummyController - Manages training mode dummy behavior.
 *
 * This module provides a state machine for controlling dummy behavior
 * in training mode, including blocking, stance, and recovery options.
 */

import { DummyState as WasmDummyState } from './TrainingSession';

/**
 * Dummy stance/behavior state for training mode.
 */
export type DummyState =
  | 'stand'
  | 'crouch'
  | 'jump'
  | 'block_stand'
  | 'block_crouch'
  | 'block_auto';

/**
 * Dummy recovery behavior after getting hit.
 */
export type DummyRecovery = 'neutral' | 'reversal';

/**
 * Configuration for dummy behavior in training mode.
 */
export interface DummyConfig {
  /** Current stance/behavior state. */
  state: DummyState;
  /** Recovery behavior after hitstun/blockstun ends. */
  recovery: DummyRecovery;
  /** Move to perform on reversal recovery (if recovery is 'reversal'). */
  reversal_move?: string;
  /** Whether dummy should perform a counter move on hit. */
  counter_on_hit: boolean;
}

/**
 * Default configuration for the dummy.
 */
const DEFAULT_CONFIG: DummyConfig = {
  state: 'stand',
  recovery: 'neutral',
  reversal_move: undefined,
  counter_on_hit: false,
};

/**
 * DummyController manages training mode dummy behavior configuration.
 *
 * This is a simple state machine that tracks dummy settings and converts
 * them to the appropriate WASM DummyState for the runtime.
 */
export class DummyController {
  private _config: DummyConfig;

  constructor(config?: Partial<DummyConfig>) {
    this._config = { ...DEFAULT_CONFIG, ...config };
  }

  /**
   * Get the current dummy configuration.
   *
   * Returns a copy to prevent external mutation.
   */
  get config(): DummyConfig {
    return { ...this._config };
  }

  /**
   * Set the dummy state (stance/behavior).
   */
  setState(state: DummyState): void {
    this._config.state = state;
  }

  /**
   * Set the recovery behavior.
   */
  setRecovery(recovery: DummyRecovery): void {
    this._config.recovery = recovery;
  }

  /**
   * Set the reversal move to perform on recovery.
   */
  setReversalMove(move: string | undefined): void {
    this._config.reversal_move = move;
  }

  /**
   * Set whether dummy should counter on hit.
   */
  setCounterOnHit(enabled: boolean): void {
    this._config.counter_on_hit = enabled;
  }

  /**
   * Set the full configuration at once.
   */
  setConfig(config: DummyConfig): void {
    this._config = { ...config };
  }

  /**
   * Reset configuration to defaults.
   */
  reset(): void {
    this._config = { ...DEFAULT_CONFIG };
  }

  /**
   * Convert the current state to a WASM DummyState value.
   *
   * This maps the TypeScript state to the numeric enum expected
   * by the WASM runtime.
   */
  getWasmState(): WasmDummyState {
    switch (this._config.state) {
      case 'stand':
        return WasmDummyState.Stand;
      case 'crouch':
        return WasmDummyState.Crouch;
      case 'jump':
        return WasmDummyState.Jump;
      case 'block_stand':
        return WasmDummyState.BlockStand;
      case 'block_crouch':
        return WasmDummyState.BlockCrouch;
      case 'block_auto':
        return WasmDummyState.BlockAuto;
    }
  }
}
