import type { ButtonName } from './InputBuffer';
import type { MoveDefinition, MoveList } from './MoveResolver';

export interface CanonicalMoveRef {
  input: string;
  type?: string;
}

const NEVER_MATCHING_INPUT: MoveDefinition['input'] = {
  type: 'simple',
  // NaN is never equal to any direction we might read from input.
  direction: Number.NaN,
  button: 'L',
};

function isButtonName(value: string): value is ButtonName {
  return value === 'L' || value === 'M' || value === 'H' || value === 'P' || value === 'K' || value === 'S';
}

export function buildMoveList(moves?: CanonicalMoveRef[] | null): MoveList {
  const defs: MoveDefinition[] = [];
  const moveNameToIndex = new Map<string, number>();

  if (!moves) {
    return { moves: defs, moveNameToIndex };
  }

  for (let index = 0; index < moves.length; index++) {
    const move = moves[index];
    const parsed = parseInputNotation(move.input);

    defs.push({
      name: move.input,
      input: parsed ?? NEVER_MATCHING_INPUT,
      priority: getMoveTypePriority(move.type),
    });
    moveNameToIndex.set(move.input, index);
  }

  return { moves: defs, moveNameToIndex };
}

function getMoveTypePriority(type: string | undefined): number {
  switch (type) {
    case 'super':
      return 100;
    case 'ex':
      return 90;
    case 'special':
      return 80;
    case 'rekka':
      return 70;
    case 'command_normal':
      return 60;
    case 'normal':
    default:
      return 50;
  }
}

function parseInputNotation(input: string): MoveDefinition['input'] | null {
  const simpleMatch = input.match(/^([1-9])([LMHPKS])$/);
  if (simpleMatch) {
    const button = simpleMatch[2];
    if (!isButtonName(button)) {
      return null;
    }
    return {
      type: 'simple',
      direction: parseInt(simpleMatch[1]),
      button,
    };
  }

  // Reject any digits outside 1-9 (e.g. 0) to avoid parsing invalid motions.
  const motionMatch = input.match(/^([1-9]{3,})([LMHPKS])$/);
  if (motionMatch) {
    const button = motionMatch[2];
    if (!isButtonName(button)) {
      return null;
    }
    const sequence = motionMatch[1].split('').map(d => parseInt(d));
    return {
      type: 'motion',
      sequence,
      button,
    };
  }

  // Double-tap dash inputs (44, 66) - direction-only, no button
  // These are triggered by tapping a direction twice quickly
  const dashMatch = input.match(/^([46])\1$/);
  if (dashMatch) {
    const direction = parseInt(dashMatch[1]);
    return {
      type: 'dash',
      direction,
    } as MoveDefinition['input'];
  }

  return null;
}
