import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import type { CancelTable, Character } from '$lib/types';

function readJson<T>(relativePath: string): T {
  return JSON.parse(readFileSync(join(process.cwd(), relativePath), 'utf8')) as T;
}

function hasOwn(value: object, key: string): boolean {
  return Object.prototype.hasOwnProperty.call(value, key);
}

describe('checked sample schema fixtures', () => {
  it('uses the property-based character schema for test_char', () => {
    const character = readJson<Character>('characters/test_char/character.json');

    expect(character.id).toBe('test_char');
    expect(character.properties.archetype).toBe('all-rounder');
    expect(character.properties.health).toBe(1000);
    expect(Array.isArray(character.resources)).toBe(true);
    expect(character.resources[0]).toEqual({ name: 'heat', start: 0, max: 100 });

    expect(hasOwn(character, 'archetype')).toBe(false);
    expect(hasOwn(character, 'health')).toBe(false);
    expect(hasOwn(character, 'walk_speed')).toBe(false);
  });

  it('uses tag-rule cancel tables for test_char', () => {
    const cancelTable = readJson<CancelTable>('characters/test_char/cancel_table.json');

    expect(cancelTable.tag_rules.length).toBeGreaterThan(0);
    expect(cancelTable.tag_rules).toContainEqual({ from: 'system', to: 'any', on: 'always' });
    expect(cancelTable.deny).toEqual({});

    expect(hasOwn(cancelTable, 'chains')).toBe(false);
    expect(hasOwn(cancelTable, 'special_cancels')).toBe(false);
    expect(hasOwn(cancelTable, 'super_cancels')).toBe(false);
    expect(hasOwn(cancelTable, 'jump_cancels')).toBe(false);
  });
});
