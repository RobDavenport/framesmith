import type { State } from "$lib/types";

// Registry type subset needed for these utils
interface FilterGroups {
  [groupName: string]: string[];
}

interface MoveTypesConfig {
  filter_groups?: FilterGroups;
}

interface Registry {
  move_types?: MoveTypesConfig;
}

// Default type arrays
const defaultNormalTypes = ["normal", "command_normal"];
const defaultSpecialTypes = ["special", "super", "ex", "rekka"];

/**
 * Check if a state matches a named filter group.
 * Uses registry filter_groups if available, falls back to defaults + input pattern matching.
 */
export function matchesFilterGroup(
  move: State,
  groupName: string,
  registry?: Registry | null,
): boolean {
  const groups = registry?.move_types?.filter_groups;
  const types =
    groups?.[groupName] ??
    (groupName === "normals"
      ? defaultNormalTypes
      : groupName === "specials"
        ? defaultSpecialTypes
        : []);

  if (move.type) {
    return types.includes(move.type);
  }
  // Fallback: use input pattern if type not set
  if (groupName === "normals") {
    return !/\d{3,}/.test(move.input);
  }
  if (groupName === "specials") {
    return /\d{3,}/.test(move.input);
  }
  return false;
}

/** Total frames = startup + active + recovery */
export function getTotal(move: State): number {
  return move.startup + move.active + move.recovery;
}

/** Frame advantage on hit = hitstun - recovery */
export function getAdvantageHit(move: State): number {
  return move.hitstun - move.recovery;
}

/** Frame advantage on block = blockstun - recovery */
export function getAdvantageBlock(move: State): number {
  return move.blockstun - move.recovery;
}

/** Format advantage with +/- prefix */
export function formatAdvantage(value: number): string {
  return value >= 0 ? `+${value}` : String(value);
}

// Sortable columns
export type SortableColumn =
  | "input"
  | "name"
  | "startup"
  | "active"
  | "recovery"
  | "damage"
  | "hitstun"
  | "blockstun"
  | "hitstop"
  | "guard"
  | "animation"
  | "total"
  | "advantage_hit"
  | "advantage_block";

/** Sort moves by the given column and direction */
export function sortMoves(
  moves: State[],
  column: SortableColumn,
  direction: "asc" | "desc",
): State[] {
  return [...moves].sort((a, b) => {
    let aVal: number | string;
    let bVal: number | string;

    if (column === "total") {
      aVal = getTotal(a);
      bVal = getTotal(b);
    } else if (column === "advantage_hit") {
      aVal = getAdvantageHit(a);
      bVal = getAdvantageHit(b);
    } else if (column === "advantage_block") {
      aVal = getAdvantageBlock(a);
      bVal = getAdvantageBlock(b);
    } else {
      aVal = a[column] as string | number;
      bVal = b[column] as string | number;
    }

    if (typeof aVal === "string" && typeof bVal === "string") {
      return direction === "asc"
        ? aVal.localeCompare(bVal)
        : bVal.localeCompare(aVal);
    }
    return direction === "asc"
      ? Number(aVal) - Number(bVal)
      : Number(bVal) - Number(aVal);
  });
}

/** Filter moves by filter type, using registry or defaults */
export function filterMoves(
  moves: State[],
  filterType: string,
  registry?: Registry | null,
): State[] {
  if (filterType === "all") {
    return moves;
  }
  return moves.filter((m) => matchesFilterGroup(m, filterType, registry));
}

/** Build the list of filter dropdown options */
export function buildFilterOptions(
  registry?: Registry | null,
): { value: string; label: string }[] {
  const options = [{ value: "all", label: "All Moves" }];

  if (registry?.move_types?.filter_groups) {
    for (const groupName of Object.keys(registry.move_types.filter_groups)) {
      options.push({
        value: groupName,
        label: groupName.charAt(0).toUpperCase() + groupName.slice(1),
      });
    }
  } else {
    options.push(
      { value: "normals", label: "Normals" },
      { value: "specials", label: "Specials" },
    );
  }

  return options;
}
