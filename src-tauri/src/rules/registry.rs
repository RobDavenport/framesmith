use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{property_schema::PropertySchema, RulesFile, Severity, ValidationIssue};

/// Registry of resource IDs and event definitions.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct RulesRegistry {
    /// Known resource IDs (e.g. "heat", "ammo").
    #[serde(default)]
    pub resources: Vec<String>,
    /// Known event definitions keyed by event ID.
    #[serde(default)]
    pub events: std::collections::BTreeMap<String, EventDefinition>,
    /// Move type configuration for filtering and categorization.
    #[serde(default)]
    pub move_types: Option<MoveTypesConfig>,
    /// Chain order for deriving chain cancel edges from tags (e.g., ["L", "M", "H"]).
    #[serde(default)]
    pub chain_order: Option<Vec<String>>,
}

/// Configuration for move types and their filter groupings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct MoveTypesConfig {
    /// List of valid move type strings (e.g., "normal", "special", "super", "ex").
    #[serde(default)]
    pub types: Vec<String>,
    /// Filter groups mapping group names to lists of types.
    /// E.g., {"normals": ["normal", "command_normal"], "specials": ["special", "super", "ex"]}
    #[serde(default)]
    pub filter_groups: std::collections::BTreeMap<String, Vec<String>>,
}

/// Definition for a registered event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct EventDefinition {
    /// Contexts this event may appear in.
    pub contexts: Vec<EventContext>,
    /// Flat argument list (no nested objects/arrays).
    #[serde(default)]
    pub args: std::collections::BTreeMap<String, EventArgSpec>,
}

/// Allowed contexts for event usage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EventContext {
    OnUse,
    OnHit,
    OnBlock,
    Notify,
}

impl EventContext {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::OnUse => "on_use",
            Self::OnHit => "on_hit",
            Self::OnBlock => "on_block",
            Self::Notify => "notify",
        }
    }
}

/// Schema for a single event argument.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type")]
pub enum EventArgSpec {
    #[serde(rename = "bool")]
    Bool,
    #[serde(rename = "i64")]
    I64,
    #[serde(rename = "f32")]
    F32 { min: Option<f32>, max: Option<f32> },
    #[serde(rename = "string")]
    String,
    #[serde(rename = "enum")]
    Enum { values: Vec<String> },
}

/// Merge project + character registries.
///
/// Semantics:
/// - `resources`: union + dedup (project then character)
/// - `events`: merge by key; character overrides same id
/// - `move_types`: character overrides project if present
/// - `chain_order`: character overrides project if present
pub fn merged_registry(
    project: Option<&RulesFile>,
    character: Option<&RulesFile>,
) -> RulesRegistry {
    let mut resources = Vec::new();
    let mut seen = std::collections::HashSet::<String>::new();

    let mut events = std::collections::BTreeMap::new();
    let mut move_types: Option<MoveTypesConfig> = None;
    let mut chain_order: Option<Vec<String>> = None;

    if let Some(reg) = project.and_then(|r| r.registry.as_ref()) {
        for r in &reg.resources {
            if seen.insert(r.clone()) {
                resources.push(r.clone());
            }
        }
        events.extend(reg.events.iter().map(|(k, v)| (k.clone(), v.clone())));
        if reg.move_types.is_some() {
            move_types = reg.move_types.clone();
        }
        if reg.chain_order.is_some() {
            chain_order = reg.chain_order.clone();
        }
    }

    if let Some(reg) = character.and_then(|r| r.registry.as_ref()) {
        for r in &reg.resources {
            if seen.insert(r.clone()) {
                resources.push(r.clone());
            }
        }
        for (k, v) in &reg.events {
            events.insert(k.clone(), v.clone());
        }
        // Character overrides project for move_types and chain_order
        if reg.move_types.is_some() {
            move_types = reg.move_types.clone();
        }
        if reg.chain_order.is_some() {
            chain_order = reg.chain_order.clone();
        }
    }

    RulesRegistry {
        resources,
        events,
        move_types,
        chain_order,
    }
}

/// A fully merged rules configuration combining project and character rules.
/// This struct is used by the FSPK exporter to validate and encode data.
#[derive(Debug, Clone, Default)]
pub struct MergedRules {
    /// Merged registry (resources, events, move_types, chain_order).
    pub registry: RulesRegistry,
    /// Merged property schema. None means no strict validation.
    pub properties: Option<PropertySchema>,
    /// Merged tag schema. None means no strict validation.
    pub tags: Option<Vec<String>>,
}

impl MergedRules {
    /// Create merged rules from project and character rules files.
    pub fn merge(project: Option<&RulesFile>, character: Option<&RulesFile>) -> Self {
        Self {
            registry: merged_registry(project, character),
            properties: super::property_schema::merged_property_schema(project, character),
            tags: super::property_schema::merged_tag_schema(project, character),
        }
    }

    /// Returns true if property schema validation is enabled.
    pub fn has_property_schema(&self) -> bool {
        self.properties.is_some()
    }

    /// Returns true if tag schema validation is enabled.
    pub fn has_tag_schema(&self) -> bool {
        self.tags.is_some()
    }
}

pub fn validate_character_resources_with_registry(
    character: &crate::schema::Character,
    registry: &RulesRegistry,
) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    let mut seen = std::collections::HashSet::<String>::new();
    let allowed = registry
        .resources
        .iter()
        .map(|s| s.as_str())
        .collect::<std::collections::HashSet<_>>();

    for (i, res) in character.resources.iter().enumerate() {
        let base = format!("resources[{i}]");

        if !seen.insert(res.name.clone()) {
            issues.push(ValidationIssue {
                field: format!("{base}.name"),
                message: format!("Duplicate resource name '{}'", res.name),
                severity: Severity::Error,
            });
        }

        if !allowed.contains(res.name.as_str()) {
            issues.push(ValidationIssue {
                field: format!("{base}.name"),
                message: format!("Unknown resource '{}' (not registered)", res.name),
                severity: Severity::Error,
            });
        }

        if res.start > res.max {
            issues.push(ValidationIssue {
                field: format!("{base}.start"),
                message: "start cannot be greater than max".to_string(),
                severity: Severity::Error,
            });
        }
    }

    issues
}

pub(super) fn validate_move_registry(
    mv: &crate::schema::State,
    registry: &RulesRegistry,
    issues: &mut Vec<ValidationIssue>,
) {
    let allowed_resources = registry
        .resources
        .iter()
        .map(|s| s.as_str())
        .collect::<std::collections::HashSet<_>>();

    let total_frames = (mv.startup as u16)
        .checked_add(mv.active as u16)
        .and_then(|x| x.checked_add(mv.recovery as u16))
        .unwrap_or(u16::MAX);

    for (i, notify) in mv.notifies.iter().enumerate() {
        if notify.frame > total_frames {
            issues.push(ValidationIssue {
                field: format!("notifies[{i}].frame"),
                message: format!(
                    "notify frame {} exceeds total frames {}",
                    notify.frame, total_frames
                ),
                severity: Severity::Error,
            });
        }
    }

    let mut check_resource = |field: String, name: &str| {
        if !allowed_resources.contains(name) {
            issues.push(ValidationIssue {
                field,
                message: format!("Unknown resource '{}' (not registered)", name),
                severity: Severity::Error,
            });
        }
    };

    if let Some(preconditions) = &mv.preconditions {
        for (i, p) in preconditions.iter().enumerate() {
            if let crate::schema::Precondition::Resource { name, .. } = p {
                check_resource(format!("preconditions[{i}].name"), name);
            }
        }
    }

    if let Some(costs) = &mv.costs {
        for (i, c) in costs.iter().enumerate() {
            if let crate::schema::Cost::Resource { name, .. } = c {
                check_resource(format!("costs[{i}].name"), name);
            }
        }
    }

    if let Some(on_use) = &mv.on_use {
        for (i, d) in on_use.resource_deltas.iter().enumerate() {
            check_resource(format!("on_use.resource_deltas[{i}].name"), &d.name);
        }
    }

    if let Some(on_hit) = &mv.on_hit {
        for (i, d) in on_hit.resource_deltas.iter().enumerate() {
            check_resource(format!("on_hit.resource_deltas[{i}].name"), &d.name);
        }
    }

    if let Some(on_block) = &mv.on_block {
        for (i, d) in on_block.resource_deltas.iter().enumerate() {
            check_resource(format!("on_block.resource_deltas[{i}].name"), &d.name);
        }
    }

    let validate_emit = |issues: &mut Vec<ValidationIssue>,
                         context: EventContext,
                         base: &str,
                         emit: &crate::schema::EventEmit,
                         registry: &RulesRegistry| {
        let def = match registry.events.get(&emit.id) {
            Some(d) => d,
            None => {
                issues.push(ValidationIssue {
                    field: format!("{base}.id"),
                    message: format!("Unknown event '{}' (not registered)", emit.id),
                    severity: Severity::Error,
                });
                return;
            }
        };

        if !def.contexts.contains(&context) {
            issues.push(ValidationIssue {
                field: format!("{base}.id"),
                message: format!(
                    "Event '{}' not allowed in context '{}'",
                    emit.id,
                    context.as_str()
                ),
                severity: Severity::Error,
            });
        }

        for (k, v) in &emit.args {
            let Some(spec) = def.args.get(k) else {
                issues.push(ValidationIssue {
                    field: format!("{base}.args.{k}"),
                    message: format!("Unknown arg key '{k}' for event '{}'", emit.id),
                    severity: Severity::Error,
                });
                continue;
            };

            let mut mismatch = |expected: &str| {
                issues.push(ValidationIssue {
                    field: format!("{base}.args.{k}"),
                    message: format!(
                        "Type mismatch for arg '{k}' on event '{}': expected {expected}",
                        emit.id
                    ),
                    severity: Severity::Error,
                });
            };

            match (spec, v) {
                (EventArgSpec::Bool, crate::schema::EventArgValue::Bool(_)) => {}
                (EventArgSpec::I64, crate::schema::EventArgValue::I64(_)) => {}
                (EventArgSpec::String, crate::schema::EventArgValue::String(_)) => {}
                (EventArgSpec::F32 { min, max }, crate::schema::EventArgValue::F32(x)) => {
                    if let Some(min) = min {
                        if *x < *min {
                            issues.push(ValidationIssue {
                                field: format!("{base}.args.{k}"),
                                message: format!("Value for arg '{k}' must be >= {min}"),
                                severity: Severity::Error,
                            });
                        }
                    }
                    if let Some(max) = max {
                        if *x > *max {
                            issues.push(ValidationIssue {
                                field: format!("{base}.args.{k}"),
                                message: format!("Value for arg '{k}' must be <= {max}"),
                                severity: Severity::Error,
                            });
                        }
                    }
                }
                (EventArgSpec::F32 { min, max }, crate::schema::EventArgValue::I64(x)) => {
                    let x = *x as f32;
                    if let Some(min) = min {
                        if x < *min {
                            issues.push(ValidationIssue {
                                field: format!("{base}.args.{k}"),
                                message: format!("Value for arg '{k}' must be >= {min}"),
                                severity: Severity::Error,
                            });
                        }
                    }
                    if let Some(max) = max {
                        if x > *max {
                            issues.push(ValidationIssue {
                                field: format!("{base}.args.{k}"),
                                message: format!("Value for arg '{k}' must be <= {max}"),
                                severity: Severity::Error,
                            });
                        }
                    }
                }
                (EventArgSpec::Enum { values }, crate::schema::EventArgValue::String(s)) => {
                    if !values.iter().any(|v| v == s) {
                        issues.push(ValidationIssue {
                            field: format!("{base}.args.{k}"),
                            message: format!(
                                "Invalid enum value '{}' for arg '{k}' on event '{}'",
                                s, emit.id
                            ),
                            severity: Severity::Error,
                        });
                    }
                }
                (EventArgSpec::Bool, _) => mismatch("bool"),
                (EventArgSpec::I64, _) => mismatch("i64"),
                (EventArgSpec::F32 { .. }, _) => mismatch("f32"),
                (EventArgSpec::String, _) => mismatch("string"),
                (EventArgSpec::Enum { .. }, _) => mismatch("enum (string)"),
            }
        }
    };

    if let Some(on_use) = &mv.on_use {
        for (i, emit) in on_use.events.iter().enumerate() {
            validate_emit(
                issues,
                EventContext::OnUse,
                &format!("on_use.events[{i}]"),
                emit,
                registry,
            );
        }
    }
    if let Some(on_hit) = &mv.on_hit {
        for (i, emit) in on_hit.events.iter().enumerate() {
            validate_emit(
                issues,
                EventContext::OnHit,
                &format!("on_hit.events[{i}]"),
                emit,
                registry,
            );
        }
    }
    if let Some(on_block) = &mv.on_block {
        for (i, emit) in on_block.events.iter().enumerate() {
            validate_emit(
                issues,
                EventContext::OnBlock,
                &format!("on_block.events[{i}]"),
                emit,
                registry,
            );
        }
    }

    for (ni, notify) in mv.notifies.iter().enumerate() {
        for (ei, emit) in notify.events.iter().enumerate() {
            validate_emit(
                issues,
                EventContext::Notify,
                &format!("notifies[{ni}].events[{ei}]"),
                emit,
                registry,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RULES_VERSION: u32 = 1;

    fn rules_with_registry(registry: RulesRegistry) -> RulesFile {
        RulesFile {
            version: RULES_VERSION,
            registry: Some(registry),
            apply: Vec::new(),
            validate: Vec::new(),
            properties: None,
            tags: None,
        }
    }

    fn make_valid_move() -> crate::schema::State {
        let mut mv = crate::schema::State::default();
        mv.input = "5L".to_string();
        mv.startup = 1;
        mv.active = 1;
        mv
    }

    #[test]
    fn test_rules_registry_parses_and_schema_includes_registry() {
        let rules: RulesFile = serde_json::from_str(
            r#"{
  "version": 1,
  "registry": {
    "resources": ["heat", "ammo"],
    "events": {
      "gain_heat": {
        "contexts": ["on_hit", "notify"],
        "args": {
          "amount": { "type": "i64" }
        }
      }
    }
  }
}"#,
        )
        .unwrap();

        assert_eq!(rules.version, 1);

        let registry = rules.registry.unwrap();
        assert_eq!(
            registry.resources,
            vec!["heat".to_string(), "ammo".to_string()]
        );
        let ev = registry.events.get("gain_heat").unwrap();
        assert_eq!(ev.contexts, vec![EventContext::OnHit, EventContext::Notify]);
        assert_eq!(
            ev.args.get("amount"),
            Some(&EventArgSpec::I64),
            "amount arg should deserialize as i64 spec"
        );

        let schema = super::super::generate_rules_schema();
        let schema_json = serde_json::to_value(&schema).unwrap();
        assert!(schema_json
            .get("properties")
            .and_then(|p| p.get("registry"))
            .is_some());

        assert!(schema_json
            .get("$defs")
            .and_then(|d| d.get("RulesRegistry"))
            .is_some());
    }

    #[test]
    fn test_merged_registry_union_dedup_and_character_overrides_events() {
        let mut project_events = std::collections::BTreeMap::new();
        project_events.insert(
            "vfx.hit_sparks".to_string(),
            EventDefinition {
                contexts: vec![EventContext::OnHit],
                args: std::collections::BTreeMap::new(),
            },
        );
        project_events.insert(
            "vfx.swing_trail".to_string(),
            EventDefinition {
                contexts: vec![EventContext::OnHit],
                args: std::collections::BTreeMap::new(),
            },
        );

        let project = RulesFile {
            version: RULES_VERSION,
            registry: Some(RulesRegistry {
                resources: vec!["heat".to_string(), "ammo".to_string()],
                events: project_events,
                ..Default::default()
            }),
            apply: vec![],
            validate: vec![],
            properties: None,
            tags: None,
        };

        let mut character_events = std::collections::BTreeMap::new();
        // Override same id with different contexts.
        character_events.insert(
            "vfx.swing_trail".to_string(),
            EventDefinition {
                contexts: vec![EventContext::Notify],
                args: std::collections::BTreeMap::new(),
            },
        );

        let character = RulesFile {
            version: RULES_VERSION,
            registry: Some(RulesRegistry {
                resources: vec!["heat".to_string(), "stamina".to_string()],
                events: character_events,
                ..Default::default()
            }),
            apply: vec![],
            validate: vec![],
            properties: None,
            tags: None,
        };

        let merged = merged_registry(Some(&project), Some(&character));
        assert_eq!(
            merged.resources,
            vec![
                "heat".to_string(),
                "ammo".to_string(),
                "stamina".to_string()
            ]
        );

        let swing = merged.events.get("vfx.swing_trail").unwrap();
        assert_eq!(swing.contexts, vec![EventContext::Notify]);
    }

    #[test]
    fn test_validate_move_events_unknown_id_is_error() {
        let rules = rules_with_registry(RulesRegistry {
            resources: vec![],
            events: std::collections::BTreeMap::new(),
            ..Default::default()
        });

        let mut mv = make_valid_move();
        mv.on_hit = Some(crate::schema::OnHit {
            events: vec![crate::schema::EventEmit {
                id: "vfx.hit_sparks".to_string(),
                args: std::collections::BTreeMap::new(),
            }],
            ..Default::default()
        });

        let issues = super::super::validate_move_with_rules(Some(&rules), None, &mv).unwrap();
        assert!(issues
            .iter()
            .any(|i| i.severity == Severity::Error && i.field == "on_hit.events[0].id"));
    }

    #[test]
    fn test_validate_move_events_unknown_arg_key_is_error() {
        let mut args = std::collections::BTreeMap::new();
        args.insert(
            "scale".to_string(),
            EventArgSpec::F32 {
                min: None,
                max: None,
            },
        );

        let mut events = std::collections::BTreeMap::new();
        events.insert(
            "vfx.hit_sparks".to_string(),
            EventDefinition {
                contexts: vec![EventContext::OnHit],
                args,
            },
        );

        let rules = rules_with_registry(RulesRegistry {
            resources: vec![],
            events,
            ..Default::default()
        });

        let mut mv = make_valid_move();
        let mut emit_args = std::collections::BTreeMap::new();
        emit_args.insert(
            "unknown".to_string(),
            crate::schema::EventArgValue::F32(1.0),
        );
        mv.on_hit = Some(crate::schema::OnHit {
            events: vec![crate::schema::EventEmit {
                id: "vfx.hit_sparks".to_string(),
                args: emit_args,
            }],
            ..Default::default()
        });

        let issues = super::super::validate_move_with_rules(Some(&rules), None, &mv).unwrap();
        assert!(issues.iter().any(|i| {
            i.severity == Severity::Error && i.field == "on_hit.events[0].args.unknown"
        }));
    }

    #[test]
    fn test_validate_move_events_type_mismatch_is_error() {
        let mut args = std::collections::BTreeMap::new();
        args.insert(
            "scale".to_string(),
            EventArgSpec::F32 {
                min: Some(0.0),
                max: Some(10.0),
            },
        );

        let mut events = std::collections::BTreeMap::new();
        events.insert(
            "vfx.hit_sparks".to_string(),
            EventDefinition {
                contexts: vec![EventContext::OnHit],
                args,
            },
        );

        let rules = rules_with_registry(RulesRegistry {
            resources: vec![],
            events,
            ..Default::default()
        });

        let mut mv = make_valid_move();
        let mut emit_args = std::collections::BTreeMap::new();
        emit_args.insert(
            "scale".to_string(),
            crate::schema::EventArgValue::String("nope".to_string()),
        );
        mv.on_hit = Some(crate::schema::OnHit {
            events: vec![crate::schema::EventEmit {
                id: "vfx.hit_sparks".to_string(),
                args: emit_args,
            }],
            ..Default::default()
        });

        let issues = super::super::validate_move_with_rules(Some(&rules), None, &mv).unwrap();
        assert!(issues
            .iter()
            .any(|i| i.severity == Severity::Error && i.field == "on_hit.events[0].args.scale"));
    }

    #[test]
    fn test_validate_move_events_enum_invalid_value_is_error() {
        let mut args = std::collections::BTreeMap::new();
        args.insert(
            "strength".to_string(),
            EventArgSpec::Enum {
                values: vec!["light".to_string(), "med".to_string(), "heavy".to_string()],
            },
        );

        let mut events = std::collections::BTreeMap::new();
        events.insert(
            "vfx.hit_sparks".to_string(),
            EventDefinition {
                contexts: vec![EventContext::OnHit],
                args,
            },
        );

        let rules = rules_with_registry(RulesRegistry {
            resources: vec![],
            events,
            ..Default::default()
        });

        let mut mv = make_valid_move();
        let mut emit_args = std::collections::BTreeMap::new();
        emit_args.insert(
            "strength".to_string(),
            crate::schema::EventArgValue::String("wrong".to_string()),
        );
        mv.on_hit = Some(crate::schema::OnHit {
            events: vec![crate::schema::EventEmit {
                id: "vfx.hit_sparks".to_string(),
                args: emit_args,
            }],
            ..Default::default()
        });

        let issues = super::super::validate_move_with_rules(Some(&rules), None, &mv).unwrap();
        assert!(issues.iter().any(|i| {
            i.severity == Severity::Error && i.field == "on_hit.events[0].args.strength"
        }));
    }

    #[test]
    fn test_validate_move_notify_frame_out_of_bounds_is_error() {
        let rules = rules_with_registry(RulesRegistry {
            resources: vec![],
            events: std::collections::BTreeMap::new(),
            ..Default::default()
        });

        let mut mv = make_valid_move();
        mv.recovery = 1;
        mv.notifies = vec![crate::schema::StateNotify {
            frame: 99,
            events: vec![],
        }];

        let issues = super::super::validate_move_with_rules(Some(&rules), None, &mv).unwrap();
        assert!(issues
            .iter()
            .any(|i| i.severity == Severity::Error && i.field == "notifies[0].frame"));
    }

    #[test]
    fn test_validate_move_resource_reference_not_in_registry_is_error() {
        let rules = rules_with_registry(RulesRegistry {
            resources: vec!["stamina".to_string()],
            events: std::collections::BTreeMap::new(),
            ..Default::default()
        });

        let mut mv = make_valid_move();
        mv.preconditions = Some(vec![crate::schema::Precondition::Resource {
            name: "heat".to_string(),
            min: Some(1),
            max: None,
        }]);

        let issues = super::super::validate_move_with_rules(Some(&rules), None, &mv).unwrap();
        assert!(issues
            .iter()
            .any(|i| { i.severity == Severity::Error && i.field == "preconditions[0].name" }));
    }

    #[test]
    fn test_merged_rules_struct() {
        let project = RulesFile {
            version: RULES_VERSION,
            registry: Some(RulesRegistry {
                resources: vec!["heat".to_string()],
                ..Default::default()
            }),
            apply: vec![],
            validate: vec![],
            properties: Some(PropertySchema {
                character: vec!["health".to_string()],
                state: vec!["startup".to_string()],
            }),
            tags: Some(vec!["normal".to_string()]),
        };

        let character = RulesFile {
            version: RULES_VERSION,
            registry: Some(RulesRegistry {
                resources: vec!["ammo".to_string()],
                ..Default::default()
            }),
            apply: vec![],
            validate: vec![],
            properties: Some(PropertySchema {
                character: vec!["walkSpeed".to_string()],
                state: vec!["damage".to_string()],
            }),
            tags: Some(vec!["special".to_string()]),
        };

        let merged = MergedRules::merge(Some(&project), Some(&character));

        // Registry merged
        assert_eq!(merged.registry.resources, vec!["heat", "ammo"]);

        // Property schema merged
        assert!(merged.has_property_schema());
        let props = merged.properties.as_ref().unwrap();
        assert_eq!(props.character, vec!["health", "walkSpeed"]);
        assert_eq!(props.state, vec!["startup", "damage"]);

        // Tag schema merged
        assert!(merged.has_tag_schema());
        assert_eq!(merged.tags.as_ref().unwrap(), &vec!["normal", "special"]);
    }
}
