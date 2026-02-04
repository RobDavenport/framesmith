use serde::{Deserialize, Serialize};

/// Custom schema for Option<(u8, u8)>
fn optional_frame_range_schema(gen: &mut schemars::SchemaGenerator) -> schemars::Schema {
    gen.subschema_for::<Option<[u8; 2]>>()
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct MeterGain {
    pub hit: u16,
    pub whiff: u16,
}

/// Named resource pool definition for a character.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct CharacterResource {
    pub name: String,
    pub start: u16,
    pub max: u16,
}

/// Resource delta applied by a trigger.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ResourceDelta {
    pub name: String,
    pub delta: i32,
}

/// One event emission: `emit_event(id, args)`.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct EventEmit {
    pub id: String,
    #[serde(default)]
    pub args: std::collections::BTreeMap<String, EventArgValue>,
}

/// Flat primitive arg values for event args.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
pub enum EventArgValue {
    Bool(bool),
    I64(i64),
    F32(f32),
    String(String),
}

/// Timeline-triggered notification events.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct StateNotify {
    pub frame: u16,
    #[serde(default)]
    pub events: Vec<EventEmit>,
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
        move_types: Vec<String>,
        duration: u16,
    },
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

/// Movement properties for a move
#[derive(Debug, Clone, Default, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(default)]
pub struct Movement {
    pub distance: Option<u16>,
    pub direction: Option<String>,
    pub curve: Option<String>,
    pub airborne: Option<bool>,
    pub velocity: Option<super::Vec2>,
    pub acceleration: Option<super::Vec2>,
    #[schemars(schema_with = "optional_frame_range_schema")]
    pub frames: Option<(u8, u8)>,
}

/// Super freeze effect
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct SuperFreeze {
    pub frames: u8,
    pub zoom: Option<f32>,
    pub darken: Option<f32>,
    pub flash: Option<bool>,
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

/// Entity spawning configuration
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct SpawnEntity {
    #[serde(rename = "type")]
    pub entity_type: String,
    pub tag: String,
    pub data: String,
    pub position: Option<super::Position>,
}

/// Effects triggered on move use
#[derive(Debug, Clone, Default, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(default)]
pub struct OnUse {
    pub enters_state: Option<EntersState>,
    pub spawn_entity: Option<SpawnEntity>,
    pub gain_meter: Option<u16>,
    #[serde(default)]
    pub events: Vec<EventEmit>,
    #[serde(default)]
    pub resource_deltas: Vec<ResourceDelta>,
}

/// Effects triggered on hit
#[derive(Debug, Clone, Default, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(default)]
pub struct OnHit {
    pub gain_meter: Option<u16>,
    pub heal: Option<u16>,
    pub status: Option<Vec<StatusEffect>>,
    pub knockback: Option<super::Knockback>,
    pub wall_bounce: Option<bool>,
    pub ground_bounce: Option<bool>,
    #[serde(default)]
    pub events: Vec<EventEmit>,
    #[serde(default)]
    pub resource_deltas: Vec<ResourceDelta>,
}

/// Effects triggered on block
#[derive(Debug, Clone, Default, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(default)]
pub struct OnBlock {
    pub gain_meter: Option<u16>,
    pub pushback: Option<i32>,
    #[serde(default)]
    pub events: Vec<EventEmit>,
    #[serde(default)]
    pub resource_deltas: Vec<ResourceDelta>,
}

/// Input trigger type
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum TriggerType {
    Press,
    Release,
    Hold,
}
