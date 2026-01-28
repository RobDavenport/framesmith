export interface Character {
  id: string;
  name: string;
  archetype: string;
  health: number;
  walk_speed: number;
  back_walk_speed: number;
  jump_height: number;
  jump_duration: number;
  dash_distance: number;
  dash_duration: number;
}

export interface Move {
  input: string;
  name: string;
  startup: number;
  active: number;
  recovery: number;
  damage: number;
  hitstun: number;
  blockstun: number;
  hitstop: number;
  guard: "high" | "mid" | "low" | "unblockable";
  hitboxes: FrameHitbox[];
  hurtboxes: FrameHitbox[];
  pushback: Pushback;
  meter_gain: MeterGain;
  animation: string;
}

export interface FrameHitbox {
  frames: [number, number];
  box: Rect;
}

export interface Rect {
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface Pushback {
  hit: number;
  block: number;
}

export interface MeterGain {
  hit: number;
  whiff: number;
}

export interface CancelTable {
  chains: Record<string, string[]>;
  special_cancels: string[];
  super_cancels: string[];
  jump_cancels: string[];
}

export interface CharacterData {
  character: Character;
  moves: Move[];
  cancel_table: CancelTable;
}

export interface CharacterSummary {
  id: string;
  name: string;
  archetype: string;
  move_count: number;
}
