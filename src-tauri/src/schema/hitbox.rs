use serde::{Deserialize, Serialize};

/// Custom schema for (u8, u8) tuple to fix schemars 1.0 missing `items` field
fn frame_range_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
    serde_json::from_value(serde_json::json!({
        "type": "array",
        "items": { "type": "integer", "minimum": 0, "maximum": 255 },
        "minItems": 2,
        "maxItems": 2,
        "description": "Frame range as [start, end]"
    }))
    .unwrap()
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

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct FrameHitbox {
    #[schemars(schema_with = "frame_range_schema")]
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
    #[schemars(schema_with = "frame_range_schema")]
    pub frames: (u8, u8),
    pub boxes: Vec<HitboxShape>,
    pub flags: Option<Vec<HurtboxFlag>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Pushback {
    pub hit: i32,
    pub block: i32,
}

/// 2D vector
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

/// Position for entity spawning
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Position {
    pub x: i32,
    pub y: i32,
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

/// A single hit within a move (for multi-hit moves)
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Hit {
    #[schemars(schema_with = "frame_range_schema")]
    pub frames: (u8, u8),
    pub damage: u16,
    pub chip_damage: Option<u16>,
    pub hitstun: u8,
    pub blockstun: u8,
    pub hitstop: u8,
    pub guard: super::GuardType,
    pub hitboxes: Vec<HitboxShape>,
    pub cancels: Vec<String>,
}
