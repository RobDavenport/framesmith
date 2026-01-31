/**
 * InputBuffer - Stores recent inputs and detects motion sequences.
 *
 * This module stores a history of input snapshots and provides methods
 * to detect fighting game motion inputs (quarter circles, dragon punches, etc.)
 */

/** Button names used in the input system. */
export type ButtonName = 'L' | 'M' | 'H' | 'P' | 'K' | 'S';

/**
 * A snapshot of input state at a single frame.
 *
 * Direction uses numpad notation:
 * ```
 * 7 8 9    (up-back, up, up-forward)
 * 4 5 6    (back, neutral, forward)
 * 1 2 3    (down-back, down, down-forward)
 * ```
 */
export interface InputSnapshot {
  /** Numpad direction (1-9, where 5 is neutral). */
  direction: number;
  /** Buttons currently pressed this frame. */
  buttons: ButtonName[];
}

/**
 * Pattern for detecting motion inputs (quarter circles, dragon punches, etc.)
 */
export interface MotionPattern {
  /** Sequence of numpad directions to match. */
  sequence: number[];
  /** Button that must be pressed at the end. */
  button: ButtonName;
  /** Maximum frames to complete the motion (default: 15). */
  windowFrames?: number;
}

/**
 * Pattern for detecting charge inputs (hold back, then forward + button).
 */
export interface ChargePattern {
  /** Direction to hold. */
  holdDirection: number;
  /** Direction to release to. */
  releaseDirection: number;
  /** Minimum frames to hold charge direction. */
  chargeFrames: number;
  /** Button that must be pressed on release. */
  button: ButtonName;
}

/**
 * Pattern for simple direction + button inputs.
 */
export interface SimplePattern {
  /** Direction to match (null for any direction). */
  direction: number | null;
  /** Button that must be pressed. */
  button: ButtonName;
}

/** Default motion input window in frames. */
const DEFAULT_MOTION_WINDOW = 15;

/** Default buffer capacity (1 second at 60fps). */
const DEFAULT_CAPACITY = 60;

/**
 * InputBuffer stores recent inputs and detects motion sequences.
 */
export class InputBuffer {
  private buffer: InputSnapshot[] = [];
  private capacity: number;

  constructor(capacity: number = DEFAULT_CAPACITY) {
    this.capacity = capacity;
  }

  /** Number of inputs in the buffer. */
  get length(): number {
    return this.buffer.length;
  }

  /**
   * Add an input snapshot to the buffer.
   */
  push(snapshot: InputSnapshot): void {
    if (this.buffer.length >= this.capacity) {
      this.buffer.shift();
    }
    this.buffer.push(snapshot);
  }

  /**
   * Get the most recent input snapshot.
   */
  latest(): InputSnapshot | null {
    if (this.buffer.length === 0) {
      return null;
    }
    return this.buffer[this.buffer.length - 1];
  }

  /**
   * Clear all inputs from the buffer.
   */
  clear(): void {
    this.buffer = [];
  }

  /**
   * Detect a motion input (e.g., 236P for fireball).
   *
   * Searches backwards through the buffer to find the motion sequence,
   * allowing for intermediate frames where the player holds a direction.
   *
   * @param pattern - The motion pattern to detect.
   * @returns true if the motion was detected.
   */
  detectMotion(pattern: MotionPattern): boolean {
    const window = pattern.windowFrames ?? DEFAULT_MOTION_WINDOW;
    const latest = this.latest();

    // Must have button pressed in latest frame
    if (!latest || !latest.buttons.includes(pattern.button)) {
      return false;
    }

    // Search backwards through buffer for the motion sequence
    const sequence = pattern.sequence;
    const searchStart = Math.max(0, this.buffer.length - window);
    let seqIndex = sequence.length - 1;

    for (let i = this.buffer.length - 1; i >= searchStart && seqIndex >= 0; i--) {
      const snapshot = this.buffer[i];
      if (snapshot.direction === sequence[seqIndex]) {
        seqIndex--;
      }
    }

    // Motion detected if we matched all directions in sequence
    return seqIndex < 0;
  }

  /**
   * Detect a charge input (e.g., hold back, then forward + button).
   *
   * @param pattern - The charge pattern to detect.
   * @returns true if the charge was detected.
   */
  detectCharge(pattern: ChargePattern): boolean {
    const latest = this.latest();

    // Must have button pressed in latest frame
    if (!latest || !latest.buttons.includes(pattern.button)) {
      return false;
    }

    // Latest frame must be release direction
    if (latest.direction !== pattern.releaseDirection) {
      return false;
    }

    // Count consecutive hold direction frames before release
    // Start from second-to-last frame
    let chargeCount = 0;
    for (let i = this.buffer.length - 2; i >= 0; i--) {
      const snapshot = this.buffer[i];
      if (snapshot.direction === pattern.holdDirection) {
        chargeCount++;
      } else {
        // Allow diagonal directions that include the hold direction
        // e.g., holding 1 or 7 counts as holding 4 (back)
        const isValidHold = this.directionContains(snapshot.direction, pattern.holdDirection);
        if (isValidHold) {
          chargeCount++;
        } else {
          break;
        }
      }
    }

    return chargeCount >= pattern.chargeFrames;
  }

  /**
   * Check if a diagonal direction contains a cardinal direction.
   * e.g., 1 (down-back) contains both 2 (down) and 4 (back).
   */
  private directionContains(diagonal: number, cardinal: number): boolean {
    // Map diagonals to their cardinal components
    const diagonalComponents: Record<number, number[]> = {
      1: [2, 4], // down-back
      3: [2, 6], // down-forward
      7: [4, 8], // up-back
      9: [6, 8], // up-forward
    };

    const components = diagonalComponents[diagonal];
    return components ? components.includes(cardinal) : false;
  }

  /**
   * Detect a simple direction + button input.
   *
   * @param pattern - The simple pattern to detect.
   * @returns true if the input matches.
   */
  detectSimple(pattern: SimplePattern): boolean {
    const latest = this.latest();

    if (!latest || !latest.buttons.includes(pattern.button)) {
      return false;
    }

    // null direction matches any
    if (pattern.direction === null) {
      return true;
    }

    return latest.direction === pattern.direction;
  }
}
