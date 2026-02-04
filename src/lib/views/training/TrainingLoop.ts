/**
 * TrainingLoop - Game loop and simulation logic for training mode.
 *
 * Handles:
 * - Game loop management (start, stop, tick)
 * - WASM runtime interaction
 * - Frame stepping and playback speed
 * - Input processing and move resolution
 * - Hit detection and combo tracking
 * - Movement and position updates
 */

import { writable, type Writable } from 'svelte/store';
import type { PlaybackSpeed } from '$lib/components/training/PlaybackControls.svelte';
import {
  TrainingSession,
  NO_INPUT,
  type CharacterState,
  type FrameResult,
} from '$lib/training/TrainingSession';
import {
  InputManager,
  InputBuffer,
  MoveResolver,
  DummyController,
  type InputSnapshot,
} from '$lib/training';
import type { Character, State } from '$lib/types';
import { getCharProp } from '$lib/utils';

// Constants
const INPUT_HISTORY_MAX = 30;
const COMBO_RESET_FRAMES = 60; // Reset combo after 1 second of no hits

// Stage boundaries
const MIN_X = 50;
const MAX_X = 750;

/**
 * Game state managed by the training loop
 */
export interface TrainingLoopState {
  // Frame tracking
  frameCount: number;

  // Character states
  playerState: CharacterState | null;
  dummyState: CharacterState | null;

  // Positions (in screen pixels)
  playerX: number;
  playerY: number;
  dummyX: number;
  dummyY: number;

  // Health tracking
  playerHealth: number;
  dummyHealth: number;
  maxHealth: number;

  // Combo tracking
  comboHits: number;
  comboDamage: number;
  comboResetTimer: number;

  // Input history
  inputHistory: InputSnapshot[];

  // Playback controls
  isPlaying: boolean;
  playbackSpeed: PlaybackSpeed;
  frameAccumulator: number;
}

/**
 * Configuration for the training loop
 */
export interface TrainingLoopConfig {
  session: TrainingSession;
  inputManager: InputManager;
  inputBuffer: InputBuffer;
  moveResolver: MoveResolver;
  dummyController: DummyController;
  character: Character;
  moves: State[];
  onError?: (error: string) => void;
}

/**
 * TrainingLoop manages the game loop and simulation state.
 */
export class TrainingLoop {
  public session: TrainingSession;
  public inputManager: InputManager;
  private inputBuffer: InputBuffer;
  private moveResolver: MoveResolver;
  private dummyController: DummyController;
  private character: Character;
  private moves: State[];
  private onError?: (error: string) => void;

  // Animation frame ID for game loop
  private animationFrameId: number | null = null;
  private loopSeq = 0;
  private lastTime = 0;

  // Reactive stores for state
  public state: Writable<TrainingLoopState>;

  constructor(config: TrainingLoopConfig) {
    this.session = config.session;
    this.inputManager = config.inputManager;
    this.inputBuffer = config.inputBuffer;
    this.moveResolver = config.moveResolver;
    this.dummyController = config.dummyController;
    this.character = config.character;
    this.moves = config.moves;
    this.onError = config.onError;

    // Initialize state with defaults
    const maxHealth = getCharProp(this.character, 'health', 1000);
    this.state = writable<TrainingLoopState>({
      frameCount: 0,
      playerState: this.session.playerState(),
      dummyState: this.session.dummyState(),
      playerX: 350,
      playerY: 0,
      dummyX: 450,
      dummyY: 0,
      playerHealth: maxHealth,
      dummyHealth: maxHealth,
      maxHealth,
      comboHits: 0,
      comboDamage: 0,
      comboResetTimer: 0,
      inputHistory: [],
      isPlaying: true,
      playbackSpeed: 1,
      frameAccumulator: 0,
    });
  }

  /**
   * Start the game loop
   */
  start(): void {
    this.stop();
    this.loopSeq++;
    const currentSeq = this.loopSeq;
    this.lastTime = performance.now();

    const gameLoop = (currentTime: number) => {
      if (currentSeq !== this.loopSeq) return;

      this.state.update(state => {
        // Handle playback paused
        if (!state.isPlaying) {
          this.animationFrameId = requestAnimationFrame(gameLoop);
          this.lastTime = currentTime;
          return state;
        }

        // Handle frame-by-frame mode (speed = 0)
        if (state.playbackSpeed === 0) {
          this.animationFrameId = requestAnimationFrame(gameLoop);
          this.lastTime = currentTime;
          return state;
        }

        // Accumulate time for sub-speed playback
        const deltaTime = currentTime - this.lastTime;
        this.lastTime = currentTime;

        // Calculate if we should run a frame based on speed
        // At 60fps, one frame is ~16.67ms
        const frameTime = 16.67;
        state.frameAccumulator += deltaTime * state.playbackSpeed;

        if (state.frameAccumulator < frameTime) {
          this.animationFrameId = requestAnimationFrame(gameLoop);
          return state;
        }

        // Run one frame (don't accumulate multiple to keep it smooth)
        state.frameAccumulator = Math.min(state.frameAccumulator - frameTime, frameTime);

        // Tick the simulation
        const newState = this.tickOneFrame(state);

        // Schedule next frame
        this.animationFrameId = requestAnimationFrame(gameLoop);

        return newState;
      });
    };

    this.animationFrameId = requestAnimationFrame(gameLoop);
  }

  /**
   * Stop the game loop
   */
  stop(): void {
    if (this.animationFrameId !== null) {
      cancelAnimationFrame(this.animationFrameId);
      this.animationFrameId = null;
    }
    this.loopSeq++;
  }

  /**
   * Toggle play/pause
   */
  togglePlayPause(): void {
    this.state.update(state => ({
      ...state,
      isPlaying: !state.isPlaying,
    }));
  }

  /**
   * Step forward one frame
   */
  stepForward(): void {
    this.state.update(state => {
      // Pause if playing
      if (state.isPlaying) {
        state = { ...state, isPlaying: false };
      }
      // Tick one frame
      return this.tickOneFrame(state);
    });
  }

  /**
   * Set playback speed
   */
  setPlaybackSpeed(speed: PlaybackSpeed): void {
    this.state.update(state => ({
      ...state,
      playbackSpeed: speed,
    }));
  }

  /**
   * Reset health and session state
   */
  resetHealth(): void {
    this.session.reset();
    this.state.update(state => ({
      ...state,
      playerHealth: state.maxHealth,
      dummyHealth: state.maxHealth,
      comboHits: 0,
      comboDamage: 0,
      comboResetTimer: 0,
    }));
  }

  /**
   * Tick one frame of simulation
   */
  private tickOneFrame(state: TrainingLoopState): TrainingLoopState {
    // Get input snapshot and add to buffer
    const snapshot = this.inputManager.getSnapshot();
    this.inputBuffer.push(snapshot);

    // Store input in history for display
    const inputHistory = [...state.inputHistory.slice(-(INPUT_HISTORY_MAX - 1)), snapshot];

    // Resolve move from input
    const newlyPressed = this.inputManager.newlyPressedButtons;
    const resolved = this.moveResolver.resolve(
      this.inputBuffer,
      newlyPressed,
      this.session.availableCancels()
    );
    this.inputManager.consumeNewlyPressed();

    // Get move index or NO_INPUT
    const playerInput = resolved ? resolved.index : NO_INPUT;

    // Get dummy state
    const wasmDummyState = this.dummyController.getWasmState();

    // Sync positions to WASM for hit detection
    this.session.setPositions(state.playerX, state.playerY, state.dummyX, state.dummyY);

    // Tick simulation with error handling for WASM errors
    let result: FrameResult;
    try {
      result = this.session.tick(playerInput, wasmDummyState);
    } catch (e) {
      console.error('WASM tick error:', e);
      this.stop();
      if (this.onError) {
        this.onError(`WASM error: ${e instanceof Error ? e.message : String(e)}`);
      }
      return state;
    }

    // Log state transitions for debugging
    const prevState = state.playerState?.current_state;
    if (prevState !== result.player.current_state) {
      const move = this.moves[result.player.current_state];
      console.log('[STATE]', {
        from: prevState,
        to: result.player.current_state,
        moveName: move?.input ?? 'unknown',
        hitboxes: move?.hitboxes?.length ?? 0,
      });
    }

    // Apply movement
    let { playerX, playerY } = state;
    const moveResult = this.applyMovement(snapshot, result.player, playerX, playerY);
    playerX = moveResult.x;
    playerY = moveResult.y;

    // Process hits and track combos
    let { dummyHealth, comboHits, comboDamage, comboResetTimer } = state;
    const hits = result.hits;

    if (hits.length > 0) {
      console.log('[HIT]', {
        playerPos: { x: playerX, y: playerY },
        dummyPos: { x: state.dummyX, y: state.dummyY },
        playerState: result.player.current_state,
        playerFrame: result.player.frame,
        hits: hits.map(h => ({ damage: h.damage, move: h.attacker_move })),
      });

      for (const hit of hits) {
        // Apply damage to dummy (player attacking)
        dummyHealth = Math.max(0, dummyHealth - hit.damage);
        // Track combo
        comboHits++;
        comboDamage += hit.damage;
      }
      // Reset combo timer on hit
      comboResetTimer = COMBO_RESET_FRAMES;
    } else {
      // Decrease combo timer
      if (comboResetTimer > 0) {
        comboResetTimer--;
        if (comboResetTimer === 0) {
          // Reset combo
          comboHits = 0;
          comboDamage = 0;
        }
      }
    }

    // Apply push separation when characters' pushboxes overlap
    let dummyX = state.dummyX;
    if (result.push_separation) {
      playerX += result.push_separation.player_dx;
      dummyX += result.push_separation.dummy_dx;
    }

    return {
      ...state,
      frameCount: state.frameCount + 1,
      playerState: result.player,
      dummyState: result.dummy,
      playerX,
      playerY,
      dummyX,
      dummyY: state.dummyY,
      dummyHealth,
      comboHits,
      comboDamage,
      comboResetTimer,
      inputHistory,
    };
  }

  /**
   * Apply movement based on current state and input
   */
  private applyMovement(
    snapshot: InputSnapshot,
    charState: CharacterState,
    x: number,
    y: number
  ): { x: number; y: number } {
    const move = this.moves[charState.current_state];

    // Check if in a movement state with movement data
    if (move?.movement) {
      const movement = move.movement;
      const totalFrames = move.total ?? (move.startup + move.active + move.recovery);

      if (movement.distance && movement.direction) {
        // Calculate per-frame movement
        const perFrame = movement.distance / totalFrames;
        const direction = movement.direction === 'forward' ? 1 : -1;

        // Apply movement (player faces right)
        x = Math.max(MIN_X, Math.min(MAX_X, x + perFrame * direction));
      }
    }
    // Walking: apply when in system state (idle/crouch) and holding direction
    else if (charState.current_state <= 1) {
      // Direction 4 = back (left), 6 = forward (right)
      // Also handle diagonals: 1, 4, 7 = back; 3, 6, 9 = forward
      const isHoldingBack = [1, 4, 7].includes(snapshot.direction);
      const isHoldingForward = [3, 6, 9].includes(snapshot.direction);

      if (isHoldingBack) {
        const backWalkSpeed = getCharProp(this.character, 'back_walk_speed', 3.2);
        x = Math.max(MIN_X, x - backWalkSpeed);
      } else if (isHoldingForward) {
        const walkSpeed = getCharProp(this.character, 'walk_speed', 4.5);
        x = Math.min(MAX_X, x + walkSpeed);
      }
    }

    return { x, y };
  }

  /**
   * Clean up resources
   */
  dispose(): void {
    this.stop();
  }
}
