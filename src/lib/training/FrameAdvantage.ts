/**
 * Frame advantage calculation utilities.
 *
 * Frame advantage is the difference between when the attacker recovers
 * and when the defender recovers from hitstun/blockstun.
 *
 * Positive advantage means the attacker recovers first.
 * Negative advantage means the defender recovers first.
 */

export interface FrameAdvantageInput {
  /** Startup frames of the move. */
  startup: number;
  /** Active frames of the move. */
  active: number;
  /** Recovery frames of the move. */
  recovery: number;
  /** Hitstun inflicted on the defender. */
  hitstun: number;
  /** Blockstun inflicted on the defender. */
  blockstun: number;
}

export interface FrameAdvantageResult {
  /** Frame advantage on hit. */
  onHit: number;
  /** Frame advantage on block. */
  onBlock: number;
}

/**
 * Calculate frame advantage for a move.
 *
 * The calculation assumes the hit lands on the first active frame.
 *
 * On hit:
 *   - Attacker has (active - 1) + recovery frames remaining
 *   - Defender is in hitstun for hitstun frames
 *   - Advantage = hitstun - (active - 1 + recovery)
 *
 * On block:
 *   - Same as hit but with blockstun instead
 *   - Advantage = blockstun - (active - 1 + recovery)
 *
 * @param input - The frame data and stun values
 * @returns Frame advantage on hit and on block
 */
export function calculateFrameAdvantage(input: FrameAdvantageInput): FrameAdvantageResult {
  // Frames attacker needs to finish after hit lands on first active frame
  const attackerRemaining = (input.active - 1) + input.recovery;

  return {
    onHit: input.hitstun - attackerRemaining,
    onBlock: input.blockstun - attackerRemaining,
  };
}

/**
 * Simplified frame advantage calculation using only recovery.
 *
 * This is a simpler approximation that assumes the hit lands instantly.
 * It's useful when you don't have full active frame data.
 *
 * On hit: Advantage = hitstun - recovery
 * On block: Advantage = blockstun - recovery
 *
 * @param recovery - Recovery frames of the move
 * @param hitstun - Hitstun inflicted on the defender
 * @param blockstun - Blockstun inflicted on the defender
 * @returns Frame advantage on hit and on block
 */
export function calculateSimpleFrameAdvantage(
  recovery: number,
  hitstun: number,
  blockstun: number
): FrameAdvantageResult {
  return {
    onHit: hitstun - recovery,
    onBlock: blockstun - recovery,
  };
}

/**
 * Format frame advantage for display.
 *
 * @param value - The frame advantage value
 * @returns Formatted string with + prefix for positive values
 */
export function formatFrameAdvantage(value: number): string {
  if (value > 0) return `+${value}`;
  return String(value);
}
