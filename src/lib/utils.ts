/**
 * Shared utility functions for Framesmith
 */

import type { Character } from './types';

/**
 * Get a character property with fallback.
 * Prefers the dynamic properties map, falls back to legacy fixed fields, then the default.
 *
 * @param char - The character object
 * @param key - Property key (e.g., 'health', 'walk_speed')
 * @param fallback - Default value if property not found
 * @returns The property value or fallback
 */
export function getCharProp(char: Character, key: string, fallback: number): number {
  // Prefer properties map
  const val = char.properties?.[key];
  if (typeof val === 'number') return val;
  // Fall back to legacy fixed fields (cast through unknown to avoid type error)
  const legacyVal = (char as unknown as Record<string, unknown>)[key];
  if (typeof legacyVal === 'number') return legacyVal;
  return fallback;
}
