/**
 * Shared utility functions for Framesmith
 */

import type { Character, State } from './types';

/**
 * Get a character property with fallback.
 *
 * @param char - The character object
 * @param key - Property key (e.g., 'health', 'walk_speed')
 * @param fallback - Default value if property not found
 * @returns The property value or fallback
 */
export function getCharProp(char: Character, key: string, fallback: number): number {
  const val = char.properties[key];
  if (typeof val === 'number') return val;
  return fallback;
}

/** Stable authoring/editor key for states that may be resolved variants. */
export function getStateKey(state: State): string {
  return state.id ?? state.input;
}

/** True when a loaded state is a resolved overlay variant rather than a base state. */
export function isResolvedVariantState(state: State): boolean {
  return typeof state.id === 'string' && state.id.length > 0 && state.id !== state.input;
}
