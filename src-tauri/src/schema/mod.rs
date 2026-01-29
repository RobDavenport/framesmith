use serde::{Deserialize, Serialize};

/// Complete character definition
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Character {
    pub id: String,
    pub name: String,
    pub archetype: String,
    pub health: u32,
    pub walk_speed: f32,
    pub back_walk_speed: f32,
    pub jump_height: u32,
    pub jump_duration: u32,
    pub dash_distance: u32,
    pub dash_duration: u32,
}

/// Single move definition
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(default)]
pub struct Move {
    pub input: String,
    pub name: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub startup: u8,
    pub active: u8,
    pub recovery: u8,
    pub damage: u16,
    pub hitstun: u8,
    pub blockstun: u8,
    pub hitstop: u8,
    pub guard: GuardType,
    pub hitboxes: Vec<FrameHitbox>,
    pub hurtboxes: Vec<FrameHitbox>,
    pub pushback: Pushback,
    pub meter_gain: MeterGain,
    pub animation: String,
    // Advanced fields (all optional for backward compatibility)
    #[serde(rename = "type")]
    pub move_type: Option<MoveType>,
    pub trigger: Option<TriggerType>,
    pub parent: Option<String>,
    pub total: Option<u8>,
    pub hits: Option<Vec<Hit>>,
    pub preconditions: Option<Vec<Precondition>>,
    pub costs: Option<Vec<Cost>>,
    pub movement: Option<Movement>,
    pub super_freeze: Option<SuperFreeze>,
    pub on_use: Option<OnUse>,
    pub on_hit: Option<OnHit>,
    pub on_block: Option<OnBlock>,
    pub advanced_hurtboxes: Option<Vec<FrameHurtbox>>,
}

impl Default for Move {
    fn default() -> Self {
        Self {
            input: String::new(),
            name: String::new(),
            tags: Vec::new(),
            startup: 0,
            active: 0,
            recovery: 0,
            damage: 0,
            hitstun: 0,
            blockstun: 0,
            hitstop: 0,
            guard: GuardType::Mid,
            hitboxes: Vec::new(),
            hurtboxes: Vec::new(),
            pushback: Pushback { hit: 0, block: 0 },
            meter_gain: MeterGain { hit: 0, whiff: 0 },
            animation: String::new(),
            move_type: None,
            trigger: None,
            parent: None,
            total: None,
            hits: None,
            preconditions: None,
            costs: None,
            movement: None,
            super_freeze: None,
            on_use: None,
            on_hit: None,
            on_block: None,
            advanced_hurtboxes: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum GuardType {
    High,
    Mid,
    Low,
    Unblockable,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct FrameHitbox {
    pub frames: (u8, u8),
    pub r#box: Rect,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Pushback {
    pub hit: i32,
    pub block: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct MeterGain {
    pub hit: u16,
    pub whiff: u16,
}

/// Cancel table defining all move relationships
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct CancelTable {
    pub chains: std::collections::HashMap<String, Vec<String>>,
    pub special_cancels: Vec<String>,
    pub super_cancels: Vec<String>,
    pub jump_cancels: Vec<String>,
}

// ============================================================================
// Advanced Move Data Types
// ============================================================================

/// Type of move
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MoveType {
    Normal,
    CommandNormal,
    Special,
    Super,
    Movement,
    Throw,
}

/// Input trigger type
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum TriggerType {
    Press,
    Release,
    Hold,
}

/// Hitbox shape variants
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum HitboxShape {
    Aabb {
        x: i32,
        y: i32,
        w: u32,
        h: u32,
    },
    Rect {
        x: i32,
        y: i32,
        w: u32,
        h: u32,
        angle: f32,
    },
    Circle {
        x: i32,
        y: i32,
        r: u32,
    },
    Capsule {
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        r: u32,
    },
}

/// A single hit within a move (for multi-hit moves)
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Hit {
    pub frames: (u8, u8),
    pub damage: u16,
    pub chip_damage: Option<u16>,
    pub hitstun: u8,
    pub blockstun: u8,
    pub hitstop: u8,
    pub guard: GuardType,
    pub hitboxes: Vec<HitboxShape>,
    pub cancels: Vec<String>,
}

/// Preconditions required for a move to be available
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Precondition {
    Meter {
        min: Option<u16>,
        max: Option<u16>,
    },
    Charge {
        direction: String,
        min_frames: u8,
    },
    State {
        r#in: String,
    },
    Grounded,
    Airborne,
    Health {
        min_percent: Option<u8>,
        max_percent: Option<u8>,
    },
    EntityCount {
        tag: String,
        min: Option<u8>,
        max: Option<u8>,
    },
    Resource {
        name: String,
        min: Option<u16>,
        max: Option<u16>,
    },
    ComboCount {
        min: Option<u8>,
        max: Option<u8>,
    },
    OpponentState {
        r#in: Vec<String>,
    },
    Distance {
        min: Option<u16>,
        max: Option<u16>,
    },
}

/// Cost to use a move
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Cost {
    Meter { amount: u16 },
    Health { amount: u16 },
    Resource { name: String, amount: u16 },
}

/// 2D vector
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

/// Movement properties for a move
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(default)]
pub struct Movement {
    pub distance: Option<u16>,
    pub direction: Option<String>,
    pub curve: Option<String>,
    pub airborne: Option<bool>,
    pub velocity: Option<Vec2>,
    pub acceleration: Option<Vec2>,
    pub frames: Option<(u8, u8)>,
}

impl Default for Movement {
    fn default() -> Self {
        Self {
            distance: None,
            direction: None,
            curve: None,
            airborne: None,
            velocity: None,
            acceleration: None,
            frames: None,
        }
    }
}

/// Super freeze effect
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct SuperFreeze {
    pub frames: u8,
    pub zoom: Option<f32>,
    pub darken: Option<f32>,
    pub flash: Option<bool>,
}

/// Status effects that can be applied
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum StatusEffect {
    Poison {
        damage_per_frame: u8,
        duration: u16,
    },
    Burn {
        damage_per_frame: u8,
        duration: u16,
    },
    Stun {
        duration: u16,
    },
    Slow {
        multiplier: f32,
        duration: u16,
    },
    Weaken {
        damage_multiplier: f32,
        duration: u16,
    },
    Seal {
        move_types: Vec<MoveType>,
        duration: u16,
    },
}

/// State transition on move use
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct EntersState {
    pub name: String,
    /// None = permanent
    pub duration: Option<u16>,
    pub persistent: Option<bool>,
    pub exit_input: Option<String>,
}

/// Position for entity spawning
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

/// Entity spawning configuration
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct SpawnEntity {
    #[serde(rename = "type")]
    pub entity_type: String,
    pub tag: String,
    pub data: String,
    pub position: Option<Position>,
}

/// Knockback configuration
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Knockback {
    /// "launch", "push", "pull"
    #[serde(rename = "type")]
    pub knockback_type: String,
    pub x: Option<i32>,
    pub y: Option<i32>,
}

/// Effects triggered on move use
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(default)]
pub struct OnUse {
    pub enters_state: Option<EntersState>,
    pub spawn_entity: Option<SpawnEntity>,
    pub gain_meter: Option<u16>,
}

impl Default for OnUse {
    fn default() -> Self {
        Self {
            enters_state: None,
            spawn_entity: None,
            gain_meter: None,
        }
    }
}

/// Effects triggered on hit
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(default)]
pub struct OnHit {
    pub gain_meter: Option<u16>,
    pub heal: Option<u16>,
    pub status: Option<Vec<StatusEffect>>,
    pub knockback: Option<Knockback>,
    pub wall_bounce: Option<bool>,
    pub ground_bounce: Option<bool>,
}

impl Default for OnHit {
    fn default() -> Self {
        Self {
            gain_meter: None,
            heal: None,
            status: None,
            knockback: None,
            wall_bounce: None,
            ground_bounce: None,
        }
    }
}

/// Effects triggered on block
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(default)]
pub struct OnBlock {
    pub gain_meter: Option<u16>,
    pub pushback: Option<i32>,
}

impl Default for OnBlock {
    fn default() -> Self {
        Self {
            gain_meter: None,
            pushback: None,
        }
    }
}

/// Hurtbox flags for invulnerability and armor
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HurtboxFlag {
    StrikeInvuln,
    ThrowInvuln,
    ProjectileInvuln,
    FullInvuln,
    Armor,
}

/// Advanced hurtbox definition with shapes and flags
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct FrameHurtbox {
    pub frames: (u8, u8),
    pub boxes: Vec<HitboxShape>,
    pub flags: Option<Vec<HurtboxFlag>>,
}
