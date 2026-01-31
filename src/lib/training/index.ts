/**
 * Training mode input system.
 *
 * This module provides input handling for training mode:
 * - InputManager: Tracks held keys, converts to numpad + buttons
 * - InputBuffer: Stores recent inputs, detects motions (236, etc.)
 * - MoveResolver: Matches buffer to move names, checks available_cancels()
 */

export { InputManager, type TrainingInputConfig } from './InputManager.svelte';
export {
  InputBuffer,
  type InputSnapshot,
  type ButtonName,
  type MotionPattern,
  type ChargePattern,
  type SimplePattern,
} from './InputBuffer';
export {
  MoveResolver,
  type MoveDefinition,
  type MoveList,
  type MoveInput,
  type SimpleInput,
  type MotionInput,
  type ChargeInput,
  type ResolvedMove,
} from './MoveResolver';
