// =============================================================================
// Core Enums / Literal Types
// =============================================================================

// MoveType is now a flexible string - custom types can be defined per-project via rules registry.
// Common built-in types: "normal", "command_normal", "special", "super", "movement", "throw"
// Custom types examples: "ex", "rekka", "install", etc.

export type TriggerType = "press" | "release" | "hold";

export type GuardType = "high" | "mid" | "low" | "unblockable";

export type HurtboxFlag = "strike_invuln" | "throw_invuln" | "projectile_invuln" | "full_invuln" | "armor";

// =============================================================================
// Hitbox Shapes (Discriminated Union)
// =============================================================================

export interface AabbHitbox {
  type: "aabb";
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface RectHitbox {
  type: "rect";
  x: number;
  y: number;
  w: number;
  h: number;
  angle: number;
}

export interface CircleHitbox {
  type: "circle";
  x: number;
  y: number;
  r: number;
}

export interface CapsuleHitbox {
  type: "capsule";
  x1: number;
  y1: number;
  x2: number;
  y2: number;
  r: number;
}

export type HitboxShape = AabbHitbox | RectHitbox | CircleHitbox | CapsuleHitbox;

// =============================================================================
// Preconditions (Discriminated Union)
// =============================================================================

export interface MeterPrecondition {
  type: "meter";
  min?: number;
  max?: number;
}

export interface ChargePrecondition {
  type: "charge";
  direction: string;
  min_frames: number;
}

export interface StatePrecondition {
  type: "state";
  in: string;
}

export interface GroundedPrecondition {
  type: "grounded";
}

export interface AirbornePrecondition {
  type: "airborne";
}

export interface HealthPrecondition {
  type: "health";
  min_percent?: number;
  max_percent?: number;
}

export interface EntityCountPrecondition {
  type: "entity_count";
  tag: string;
  min?: number;
  max?: number;
}

export interface ResourcePrecondition {
  type: "resource";
  name: string;
  min?: number;
  max?: number;
}

export interface ComboCountPrecondition {
  type: "combo_count";
  min?: number;
  max?: number;
}

export interface OpponentStatePrecondition {
  type: "opponent_state";
  in: string[];
}

export interface DistancePrecondition {
  type: "distance";
  min?: number;
  max?: number;
}

export type Precondition =
  | MeterPrecondition
  | ChargePrecondition
  | StatePrecondition
  | GroundedPrecondition
  | AirbornePrecondition
  | HealthPrecondition
  | EntityCountPrecondition
  | ResourcePrecondition
  | ComboCountPrecondition
  | OpponentStatePrecondition
  | DistancePrecondition;

// =============================================================================
// Costs (Discriminated Union)
// =============================================================================

export interface MeterCost {
  type: "meter";
  amount: number;
}

export interface HealthCost {
  type: "health";
  amount: number;
}

export interface ResourceCost {
  type: "resource";
  name: string;
  amount: number;
}

export type Cost = MeterCost | HealthCost | ResourceCost;

// =============================================================================
// Status Effects (Discriminated Union)
// =============================================================================

export interface PoisonEffect {
  type: "poison";
  damage_per_frame: number;
  duration: number;
}

export interface BurnEffect {
  type: "burn";
  damage_per_frame: number;
  duration: number;
}

export interface StunEffect {
  type: "stun";
  duration: number;
}

export interface SlowEffect {
  type: "slow";
  multiplier: number;
  duration: number;
}

export interface WeakenEffect {
  type: "weaken";
  damage_multiplier: number;
  duration: number;
}

export interface SealEffect {
  type: "seal";
  move_types: string[];
  duration: number;
}

export type StatusEffect =
  | PoisonEffect
  | BurnEffect
  | StunEffect
  | SlowEffect
  | WeakenEffect
  | SealEffect;

// =============================================================================
// Movement Types
// =============================================================================

export interface Movement {
  // Distance-based movement (simple)
  distance?: number;
  direction?: "forward" | "backward";
  curve?: string;
  airborne?: boolean;
  // Velocity-based movement (complex)
  velocity?: { x: number; y: number };
  acceleration?: { x: number; y: number };
  frames?: [number, number];
}

// =============================================================================
// Advanced Move Data Structures
// =============================================================================

export interface Hit {
  frames: [number, number];
  damage: number;
  chip_damage?: number;
  hitstun: number;
  blockstun: number;
  hitstop: number;
  guard: GuardType;
  hitboxes: HitboxShape[];
  cancels: string[];
}

export interface SuperFreeze {
  frames: number;
  zoom?: number;
  darken?: number;
  flash?: boolean;
}

export interface EntersState {
  name: string;
  duration?: number | null;
  persistent?: boolean;
  exit_input?: string;
}

export interface SpawnEntity {
  type: "projectile";
  tag: string;
  data: Record<string, unknown>;
  position?: { x: number; y: number };
}

export interface Knockback {
  type: "launch" | "push" | "pull";
  x?: number;
  y?: number;
}

export interface OnUse {
  enters_state?: EntersState;
  spawn_entity?: SpawnEntity;
  gain_meter?: number;
}

export interface OnHit {
  gain_meter?: number;
  heal?: number;
  status?: StatusEffect[];
  knockback?: Knockback;
  wall_bounce?: boolean;
  ground_bounce?: boolean;
}

export interface OnBlock {
  gain_meter?: number;
  pushback?: number;
}

export interface FrameHurtbox {
  frames: [number, number];
  boxes: HitboxShape[];
  flags?: HurtboxFlag[];
}

// =============================================================================
// Legacy Types (Preserved for Backward Compatibility)
// =============================================================================

export interface Rect {
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface FrameHitbox {
  frames: [number, number];
  box: Rect;
}

export interface Pushback {
  hit: number;
  block: number;
}

export interface MeterGain {
  hit: number;
  whiff: number;
}

// =============================================================================
// Character Types
// =============================================================================

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

// =============================================================================
// Move Interface (Updated with v2 Schema Fields)
// =============================================================================

export interface Move {
  // Core identification
  input: string;
  name: string;

  // v2: Move classification (flexible string type)
  type?: string;
  trigger?: TriggerType; // default "press"
  parent?: string | null;

  // Frame data (legacy fields preserved)
  startup: number;
  active: number;
  recovery: number;
  total?: number; // v2: explicit total frame count

  // Damage and stun (legacy fields preserved)
  damage: number;
  hitstun: number;
  blockstun: number;
  hitstop: number;
  guard: GuardType;

  // Legacy hitbox/hurtbox (simple rect-based)
  hitboxes: FrameHitbox[];
  hurtboxes: FrameHitbox[];

  // v2: Advanced multi-hit support with shaped hitboxes
  hits?: Hit[];

  // v2: Advanced hurtboxes with shapes and flags
  advanced_hurtboxes?: FrameHurtbox[];

  // Legacy pushback and meter
  pushback: Pushback;
  meter_gain: MeterGain;

  // Animation reference
  animation: string;

  // v2: Requirements and costs
  preconditions?: Precondition[];
  costs?: Cost[];

  // v2: Movement during move
  movement?: Movement;

  // v2: Super freeze effect
  super_freeze?: SuperFreeze;

  // v2: Event callbacks
  on_use?: OnUse;
  on_hit?: OnHit;
  on_block?: OnBlock;
}

// =============================================================================
// Cancel Table and Character Data
// =============================================================================

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

// =============================================================================
// Character Assets
// =============================================================================

export type AnimationClip =
  | {
      mode: "sprite";
      texture: string;
      frame_size: { w: number; h: number };
      frames: number;
      pivot: { x: number; y: number };
    }
  | {
      mode: "gltf";
      model: string;
      clip: string;
      fps: number;
      pivot: { x: number; y: number; z: number };
    };

export interface CharacterAssets {
  version: number;
  textures: Record<string, string>;
  models: Record<string, string>;
  animations: Record<string, AnimationClip>;
}

// =============================================================================
// Rules Registry Types (for filtering and cancel graph)
// =============================================================================

export interface MoveTypesConfig {
  types: string[];
  filter_groups: Record<string, string[]>;
}

export interface MergedRegistry {
  resources: string[];
  move_types?: MoveTypesConfig;
  chain_order?: string[];
}
