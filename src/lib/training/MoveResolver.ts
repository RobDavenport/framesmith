/**
 * MoveResolver - Matches input buffer to move names and checks cancels.
 *
 * This module takes the current input buffer and resolves it to a specific
 * move based on priority ordering. Higher priority moves (specials, supers)
 * take precedence over lower priority moves (normals).
 */

import type { InputBuffer, ButtonName } from './InputBuffer';

/**
 * Input definition for a simple direction + button move.
 */
export interface SimpleInput {
  type: 'simple';
  /** Direction (1-9) or null for any direction. */
  direction: number | null;
  /** Button required. */
  button: ButtonName;
}

/**
 * Input definition for a motion input move.
 */
export interface MotionInput {
  type: 'motion';
  /** Sequence of numpad directions. */
  sequence: number[];
  /** Button required at the end. */
  button: ButtonName;
  /** Optional window in frames (default 15). */
  windowFrames?: number;
}

/**
 * Input definition for a charge move.
 */
export interface ChargeInput {
  type: 'charge';
  /** Direction to hold. */
  holdDirection: number;
  /** Direction to release to. */
  releaseDirection: number;
  /** Minimum charge frames. */
  chargeFrames: number;
  /** Button required on release. */
  button: ButtonName;
}

/**
 * Input definition for a dash move (double-tap direction).
 */
export interface DashInput {
  type: 'dash';
  /** Direction to double-tap (4 for back, 6 for forward). */
  direction: number;
}

export type MoveInput = SimpleInput | MotionInput | ChargeInput | DashInput;

/**
 * Definition of a move for input resolution.
 */
export interface MoveDefinition {
  /** Move name for display. */
  name: string;
  /** Input pattern for this move. */
  input: MoveInput;
  /** Priority for resolution (higher = checked first). */
  priority: number;
}

/**
 * A list of moves with name-to-index mapping.
 */
export interface MoveList {
  /** All move definitions in index order. */
  moves: MoveDefinition[];
  /** Map from move name to index. */
  moveNameToIndex: Map<string, number>;
}

/**
 * Result of move resolution.
 */
export interface ResolvedMove {
  /** Move name. */
  name: string;
  /** Move index in the move list. */
  index: number;
  /** Priority of the matched move. */
  priority: number;
}

/**
 * MoveResolver matches input buffer contents to move definitions.
 */
export class MoveResolver {
  private moves: MoveDefinition[];
  private moveNameToIndex: Map<string, number>;
  private movesByPriority: Array<{ def: MoveDefinition; index: number }>;

  constructor(moveList: MoveList) {
    this.moves = moveList.moves;
    this.moveNameToIndex = moveList.moveNameToIndex;

    // Pre-sort moves by priority (descending) for efficient resolution
    this.movesByPriority = this.moves.map((def, index) => ({ def, index }));
    this.movesByPriority.sort((a, b) => b.def.priority - a.def.priority);
  }

  /**
   * Resolve the current input buffer to a move.
   *
   * @param buffer - The input buffer to check.
   * @param newlyPressed - Buttons newly pressed this frame.
   * @param availableCancels - Optional list of move indices that are valid cancels.
   *                           If provided, only moves in this list can be returned.
   * @returns The resolved move, or null if no move matches.
   */
  resolve(
    buffer: InputBuffer,
    newlyPressed: ButtonName[],
    availableCancels?: number[]
  ): ResolvedMove | null {
    const matches = this.getMatchingMoves(buffer, newlyPressed, availableCancels);
    return matches.length > 0 ? matches[0] : null;
  }

  /**
   * Get all moves that match the current input, sorted by priority.
   *
   * @param buffer - The input buffer to check.
   * @param newlyPressed - Buttons newly pressed this frame.
   * @param availableCancels - Optional list of valid cancel targets.
   * @returns Array of matching moves sorted by priority (descending).
   */
  getMatchingMoves(
    buffer: InputBuffer,
    newlyPressed: ButtonName[],
    availableCancels?: number[]
  ): ResolvedMove[] {
    const matches: ResolvedMove[] = [];

    for (const { def, index } of this.movesByPriority) {
      // Skip if not in available cancels (but allow all moves if cancels list is empty,
      // which indicates neutral state where any move should be available)
      if (availableCancels !== undefined && availableCancels.length > 0 && !availableCancels.includes(index)) {
        continue;
      }

      // Check if this move's button was newly pressed (dash moves don't need buttons)
      const button = this.getMoveButton(def.input);
      if (button !== null && !newlyPressed.includes(button)) {
        continue;
      }

      // Check if the input pattern matches
      if (this.matchesInput(buffer, def.input)) {
        matches.push({
          name: def.name,
          index,
          priority: def.priority,
        });
      }
    }

    return matches;
  }

  /**
   * Get the button required for a move input.
   * Returns null for moves that don't require a button (e.g., dashes).
   */
  private getMoveButton(input: MoveInput): ButtonName | null {
    if (input.type === 'dash') {
      return null;
    }
    return input.button;
  }

  /**
   * Check if the buffer matches a move's input pattern.
   */
  private matchesInput(buffer: InputBuffer, input: MoveInput): boolean {
    switch (input.type) {
      case 'simple':
        return buffer.detectSimple({
          direction: input.direction,
          button: input.button,
        });

      case 'motion':
        return buffer.detectMotion({
          sequence: input.sequence,
          button: input.button,
          windowFrames: input.windowFrames,
        });

      case 'charge':
        return buffer.detectCharge({
          holdDirection: input.holdDirection,
          releaseDirection: input.releaseDirection,
          chargeFrames: input.chargeFrames,
          button: input.button,
        });

      case 'dash':
        return buffer.detectDash({
          direction: input.direction,
        });
    }
  }

  /**
   * Get the move index for a given move name.
   *
   * @param name - The move name.
   * @returns The move index, or undefined if not found.
   */
  getMoveIndex(name: string): number | undefined {
    return this.moveNameToIndex.get(name);
  }

  /**
   * Get a move definition by index.
   *
   * @param index - The move index.
   * @returns The move definition, or undefined if out of bounds.
   */
  getMove(index: number): MoveDefinition | undefined {
    return this.moves[index];
  }
}
