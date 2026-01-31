/**
 * TypeScript wrapper for the framesmith-runtime WASM module.
 *
 * This module provides a high-level interface for running character
 * simulations in training mode.
 */

// Type imports from the generated WASM bindings
import type {
  TrainingSession as WasmTrainingSession,
  DummyState as WasmDummyState,
} from '$lib/wasm/framesmith_runtime_wasm.js';

/**
 * Dummy behavior states for training mode.
 */
export enum DummyState {
  Stand = 0,
  Crouch = 1,
  Jump = 2,
  BlockStand = 3,
  BlockCrouch = 4,
  BlockAuto = 5,
}

/**
 * Character state returned from the WASM runtime.
 */
export interface CharacterState {
  current_move: number;
  frame: number;
  hit_confirmed: boolean;
  block_confirmed: boolean;
  resources: number[];
}

/**
 * Result of a hit interaction.
 */
export interface HitResult {
  attacker_move: number;
  window_index: number;
  damage: number;
  chip_damage: number;
  hitstun: number;
  blockstun: number;
  hitstop: number;
  guard: number;
  hit_pushback: number;
  block_pushback: number;
}

/**
 * Result of a single frame tick.
 */
export interface FrameResult {
  player: CharacterState;
  dummy: CharacterState;
  hits: HitResult[];
}

/**
 * No move requested (continue current move).
 */
export const NO_INPUT = 0xffff;

// Module-level WASM module reference
let wasmModule: typeof import('$lib/wasm/framesmith_runtime_wasm.js') | null = null;

/**
 * Initialize the WASM module.
 *
 * Must be called before creating any TrainingSession instances.
 */
export async function initWasm(): Promise<void> {
  if (wasmModule !== null) {
    return; // Already initialized
  }

  try {
    // Dynamic import of the WASM module
    const wasm = await import('$lib/wasm/framesmith_runtime_wasm.js');
    await wasm.default(); // Initialize the WASM module
    wasmModule = wasm;
  } catch (e) {
    console.error('Failed to initialize WASM module:', e);
    throw new Error(`Failed to initialize WASM module: ${e}`);
  }
}

/**
 * Check if the WASM module is initialized.
 */
export function isWasmReady(): boolean {
  return wasmModule !== null;
}

/**
 * Training session for simulating a player character against a dummy.
 *
 * This is a TypeScript wrapper around the WASM TrainingSession class.
 */
export class TrainingSession {
  private session: WasmTrainingSession;

  private constructor(session: WasmTrainingSession) {
    this.session = session;
  }

  /**
   * Create a new training session with the given FSPK data.
   *
   * @param playerFspk - FSPK binary data for the player character
   * @param dummyFspk - FSPK binary data for the dummy character
   * @returns A new TrainingSession instance
   * @throws Error if the WASM module is not initialized or if the FSPK data is invalid
   */
  static async create(
    playerFspk: Uint8Array,
    dummyFspk: Uint8Array
  ): Promise<TrainingSession> {
    // Ensure WASM is initialized
    await initWasm();

    if (!wasmModule) {
      throw new Error('WASM module not initialized');
    }

    // Create the WASM session
    const session = new wasmModule.TrainingSession(playerFspk, dummyFspk);
    return new TrainingSession(session);
  }

  /**
   * Advance the simulation by one frame.
   *
   * @param playerInput - Move index the player wants to perform (use NO_INPUT for no input)
   * @param dummyState - How the dummy should behave this frame
   * @returns The frame result containing new states and any hits
   */
  tick(playerInput: number, dummyState: DummyState): FrameResult {
    return this.session.tick(playerInput, dummyState as unknown as WasmDummyState);
  }

  /**
   * Get the current player state.
   */
  playerState(): CharacterState {
    return this.session.player_state();
  }

  /**
   * Get the current dummy state.
   */
  dummyState(): CharacterState {
    return this.session.dummy_state();
  }

  /**
   * Get available cancel targets for the player's current state.
   *
   * @returns Array of move indices that can be cancelled into
   */
  availableCancels(): number[] {
    return this.session.available_cancels();
  }

  /**
   * Get the hit results from the last tick.
   */
  hitResults(): HitResult[] {
    return this.session.hit_results();
  }

  /**
   * Reset the session to initial state.
   */
  reset(): void {
    this.session.reset();
  }

  /**
   * Set character positions (for collision checking).
   *
   * @param playerX - Player X position in pixels
   * @param playerY - Player Y position in pixels
   * @param dummyX - Dummy X position in pixels
   * @param dummyY - Dummy Y position in pixels
   */
  setPositions(playerX: number, playerY: number, dummyX: number, dummyY: number): void {
    this.session.set_positions(playerX, playerY, dummyX, dummyY);
  }

  /**
   * Free the WASM resources.
   *
   * Call this when the session is no longer needed.
   */
  free(): void {
    this.session.free();
  }
}
