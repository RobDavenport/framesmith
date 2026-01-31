use serde::{Deserialize, Serialize};

mod assets;
pub use assets::*;

/// Error type for invalid tags
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TagError {
    Empty,
    InvalidChars,
}

impl std::fmt::Display for TagError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TagError::Empty => write!(f, "tag cannot be empty"),
            TagError::InvalidChars => write!(f, "tag must be lowercase alphanumeric with underscores"),
        }
    }
}

impl std::error::Error for TagError {}

/// Validated tag for state categorization.
///
/// Tags are lowercase alphanumeric strings with underscores.
/// Games use tags for cancel rules and filtering.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Tag(String);

impl Tag {
    /// Create a new tag, validating the format.
    pub fn new(s: impl Into<String>) -> Result<Self, TagError> {
        let s = s.into();
        if s.is_empty() {
            return Err(TagError::Empty);
        }
        if !s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_') {
            return Err(TagError::InvalidChars);
        }
        Ok(Tag(s))
    }

    /// Get the tag as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Serialize for Tag {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Tag {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Tag::new(s).map_err(serde::de::Error::custom)
    }
}

impl schemars::JsonSchema for Tag {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed("Tag")
    }

    fn json_schema(gen: &mut schemars::SchemaGenerator) -> schemars::Schema {
        gen.subschema_for::<String>()
    }
}

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

/// Custom schema for Option<(u8, u8)>
fn optional_frame_range_schema(gen: &mut schemars::SchemaGenerator) -> schemars::Schema {
    gen.subschema_for::<Option<[u8; 2]>>()
}

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
    #[serde(default)]
    pub resources: Vec<CharacterResource>,
}

/// Named resource pool definition for a character.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct CharacterResource {
    pub name: String,
    pub start: u16,
    pub max: u16,
}

/// Character assets manifest (textures, models, animations).
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct CharacterAssets {
    #[serde(default = "default_assets_version")]
    pub version: u32,
    #[serde(default)]
    pub textures: Vec<String>,
    #[serde(default)]
    pub models: Vec<String>,
    #[serde(default)]
    pub animations: Vec<String>,
}

fn default_assets_version() -> u32 {
    1
}

impl Default for CharacterAssets {
    fn default() -> Self {
        Self {
            version: 1,
            textures: Vec::new(),
            models: Vec::new(),
            animations: Vec::new(),
        }
    }
}

/// Single move definition
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(default)]
pub struct Move {
    pub input: String,
    pub name: String,
    #[serde(default)]
    pub tags: Vec<Tag>,
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
    /// Move type as a flexible string (e.g., "normal", "special", "super", "ex", "rekka").
    /// Custom types can be defined per-project via rules registry.
    #[serde(rename = "type")]
    pub move_type: Option<String>,
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
    #[serde(default)]
    pub notifies: Vec<MoveNotify>,
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
            notifies: Vec::new(),
            advanced_hurtboxes: None,
        }
    }
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

/// Resource delta applied by a trigger.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ResourceDelta {
    pub name: String,
    pub delta: i32,
}

/// Timeline-triggered notification events.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct MoveNotify {
    pub frame: u16,
    #[serde(default)]
    pub events: Vec<EventEmit>,
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
    #[schemars(schema_with = "frame_range_schema")]
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
    #[schemars(schema_with = "optional_frame_range_schema")]
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
        move_types: Vec<String>,
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
    #[serde(default)]
    pub events: Vec<EventEmit>,
    #[serde(default)]
    pub resource_deltas: Vec<ResourceDelta>,
}

impl Default for OnUse {
    fn default() -> Self {
        Self {
            enters_state: None,
            spawn_entity: None,
            gain_meter: None,
            events: Vec::new(),
            resource_deltas: Vec::new(),
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
    #[serde(default)]
    pub events: Vec<EventEmit>,
    #[serde(default)]
    pub resource_deltas: Vec<ResourceDelta>,
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
            events: Vec::new(),
            resource_deltas: Vec::new(),
        }
    }
}

/// Effects triggered on block
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(default)]
pub struct OnBlock {
    pub gain_meter: Option<u16>,
    pub pushback: Option<i32>,
    #[serde(default)]
    pub events: Vec<EventEmit>,
    #[serde(default)]
    pub resource_deltas: Vec<ResourceDelta>,
}

impl Default for OnBlock {
    fn default() -> Self {
        Self {
            gain_meter: None,
            pushback: None,
            events: Vec::new(),
            resource_deltas: Vec::new(),
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
    #[schemars(schema_with = "frame_range_schema")]
    pub frames: (u8, u8),
    pub boxes: Vec<HitboxShape>,
    pub flags: Option<Vec<HurtboxFlag>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn character_json_without_resources_deserializes() {
        let json = r#"{
          "id": "test",
          "name": "Test",
          "archetype": "rushdown",
          "health": 10000,
          "walk_speed": 4.0,
          "back_walk_speed": 3.0,
          "jump_height": 120,
          "jump_duration": 45,
          "dash_distance": 80,
          "dash_duration": 18
        }"#;

        let character: Character = serde_json::from_str(json).expect("character should parse");
        assert!(character.resources.is_empty());
    }

    #[test]
    fn move_on_hit_events_args_deserialize_typed_values() {
        let json = r#"{
          "input": "5L",
          "name": "Light",
          "tags": [],
          "startup": 5,
          "active": 2,
          "recovery": 10,
          "damage": 500,
          "hitstun": 15,
          "blockstun": 10,
          "hitstop": 10,
          "guard": "mid",
          "hitboxes": [],
          "hurtboxes": [],
          "pushback": { "hit": 5, "block": 8 },
          "meter_gain": { "hit": 100, "whiff": 20 },
          "animation": "5L",
          "on_hit": {
            "events": [
              {
                "id": "vfx.hit_sparks",
                "args": {
                  "enabled": true,
                  "count": 3,
                  "scale": 1.2,
                  "strength": "light"
                }
              }
            ]
          }
        }"#;

        let mv: Move = serde_json::from_str(json).expect("move should parse");
        let on_hit = mv.on_hit.expect("on_hit should exist");
        assert_eq!(on_hit.events.len(), 1);

        let emit = &on_hit.events[0];
        assert_eq!(emit.id, "vfx.hit_sparks");
        assert!(matches!(
            emit.args.get("enabled"),
            Some(EventArgValue::Bool(true))
        ));
        assert!(matches!(
            emit.args.get("count"),
            Some(EventArgValue::I64(3))
        ));
        assert!(
            matches!(emit.args.get("scale"), Some(EventArgValue::F32(v)) if (*v - 1.2).abs() < 1e-6)
        );
        assert!(
            matches!(emit.args.get("strength"), Some(EventArgValue::String(s)) if s == "light")
        );
    }

    #[test]
    fn move_notifies_deserializes() {
        let json = r#"{
          "input": "5L",
          "name": "Light",
          "tags": [],
          "startup": 5,
          "active": 2,
          "recovery": 10,
          "damage": 500,
          "hitstun": 15,
          "blockstun": 10,
          "hitstop": 10,
          "guard": "mid",
          "hitboxes": [],
          "hurtboxes": [],
          "pushback": { "hit": 5, "block": 8 },
          "meter_gain": { "hit": 100, "whiff": 20 },
          "animation": "5L",
          "notifies": [
            {
              "frame": 7,
              "events": [ { "id": "vfx.swing_trail", "args": { "bone": "hand_r" } } ]
            }
          ]
        }"#;

        let mv: Move = serde_json::from_str(json).expect("move should parse");
        assert_eq!(mv.notifies.len(), 1);
        assert_eq!(mv.notifies[0].frame, 7);
        assert_eq!(mv.notifies[0].events.len(), 1);
        assert_eq!(mv.notifies[0].events[0].id, "vfx.swing_trail");
    }

    #[test]
    fn tag_valid_lowercase() {
        let tag = Tag::new("normal").unwrap();
        assert_eq!(tag.as_str(), "normal");
    }

    #[test]
    fn tag_valid_with_underscore() {
        let tag = Tag::new("on_hit").unwrap();
        assert_eq!(tag.as_str(), "on_hit");
    }

    #[test]
    fn tag_valid_with_numbers() {
        let tag = Tag::new("rekka1").unwrap();
        assert_eq!(tag.as_str(), "rekka1");
    }

    #[test]
    fn tag_rejects_empty() {
        assert!(Tag::new("").is_err());
    }

    #[test]
    fn tag_rejects_uppercase() {
        assert!(Tag::new("Normal").is_err());
    }

    #[test]
    fn tag_rejects_spaces() {
        assert!(Tag::new("on hit").is_err());
    }

    #[test]
    fn tag_rejects_special_chars() {
        assert!(Tag::new("normal!").is_err());
    }

    #[test]
    fn move_with_tags_deserializes() {
        let json = r#"{
          "input": "5L",
          "name": "Light",
          "tags": ["normal", "light"],
          "startup": 5,
          "active": 2,
          "recovery": 10,
          "damage": 500,
          "hitstun": 15,
          "blockstun": 10,
          "hitstop": 10,
          "guard": "mid",
          "hitboxes": [],
          "hurtboxes": [],
          "pushback": { "hit": 5, "block": 8 },
          "meter_gain": { "hit": 100, "whiff": 20 },
          "animation": "5L"
        }"#;

        let mv: Move = serde_json::from_str(json).expect("move should parse");
        assert_eq!(mv.tags.len(), 2);
        assert_eq!(mv.tags[0].as_str(), "normal");
        assert_eq!(mv.tags[1].as_str(), "light");
    }

    #[test]
    fn move_without_tags_deserializes_empty() {
        let json = r#"{
          "input": "5L",
          "name": "Light",
          "startup": 5,
          "active": 2,
          "recovery": 10,
          "damage": 500,
          "hitstun": 15,
          "blockstun": 10,
          "hitstop": 10,
          "guard": "mid",
          "hitboxes": [],
          "hurtboxes": [],
          "pushback": { "hit": 5, "block": 8 },
          "meter_gain": { "hit": 100, "whiff": 20 },
          "animation": "5L"
        }"#;

        let mv: Move = serde_json::from_str(json).expect("move should parse");
        assert!(mv.tags.is_empty());
    }
}
