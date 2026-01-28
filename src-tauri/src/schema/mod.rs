use serde::{Deserialize, Serialize};

/// Complete character definition
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Move {
    pub input: String,
    pub name: String,
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GuardType {
    High,
    Mid,
    Low,
    Unblockable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameHitbox {
    pub frames: (u8, u8),
    pub r#box: Rect,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pushback {
    pub hit: i32,
    pub block: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeterGain {
    pub hit: u16,
    pub whiff: u16,
}

/// Cancel table defining all move relationships
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelTable {
    pub chains: std::collections::HashMap<String, Vec<String>>,
    pub special_cancels: Vec<String>,
    pub super_cancels: Vec<String>,
    pub jump_cancels: Vec<String>,
}
