use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{
    apply::apply_rules_to_move,
    matchers::{matches_move, MatchSpec},
    registry::merged_registry,
    RulesError, RulesFile, Severity, ValidationIssue,
};

/// A rule that enforces constraints on moves, producing errors or warnings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ValidateRule {
    /// Criteria for which moves this rule applies to.
    #[serde(rename = "match")]
    pub match_spec: MatchSpec,
    /// Constraint definitions using exists, min, max, equals, or in.
    #[serde(rename = "require")]
    #[serde(deserialize_with = "deserialize_object_value")]
    #[schemars(with = "std::collections::HashMap<String, serde_json::Value>")]
    pub require: serde_json::Value,
    /// How to report violations: "error" or "warning".
    pub severity: Severity,
    /// Custom message for violations. If not provided, a default message is generated.
    pub message: Option<String>,
}

fn deserialize_object_value<'de, D>(deserializer: D) -> Result<serde_json::Value, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    match value {
        serde_json::Value::Object(_) => Ok(value),
        _ => Err(serde::de::Error::custom("expected object")),
    }
}

pub fn merged_validate_rules(
    project: Option<&RulesFile>,
    character: Option<&RulesFile>,
) -> Vec<ValidateRule> {
    let mut merged = project
        .map(|rules| rules.validate.clone())
        .unwrap_or_default();

    let character_validate = match character {
        Some(rules) => &rules.validate,
        None => return merged,
    };

    for rule in character_validate {
        merged.retain(|existing| existing.match_spec != rule.match_spec);
    }

    merged.extend(character_validate.iter().cloned());
    merged
}

fn is_unset_value(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Null => true,
        serde_json::Value::String(s) => s.is_empty(),
        serde_json::Value::Array(arr) => arr.is_empty(),
        serde_json::Value::Object(map) => map.is_empty(),
        serde_json::Value::Number(num) => {
            num.as_i64() == Some(0) || num.as_u64() == Some(0) || num.as_f64() == Some(0.0)
        }
        _ => false,
    }
}

fn get_value_at_path<'a>(
    root: &'a serde_json::Value,
    path: &[String],
) -> Option<&'a serde_json::Value> {
    let mut cur = root;
    for key in path {
        match cur {
            serde_json::Value::Object(map) => {
                cur = map.get(key)?;
            }
            _ => return None,
        }
    }
    Some(cur)
}

fn is_constraint_object(map: &serde_json::Map<String, serde_json::Value>) -> bool {
    map.contains_key("min")
        || map.contains_key("max")
        || map.contains_key("exists")
        || map.contains_key("equals")
        || map.contains_key("in")
}

fn constraints_pass(
    constraint: &serde_json::Map<String, serde_json::Value>,
    target: Option<&serde_json::Value>,
) -> bool {
    if let Some(exists_val) = constraint.get("exists") {
        let Some(exists) = exists_val.as_bool() else {
            return false;
        };

        let present_and_set = matches!(target, Some(v) if !is_unset_value(v));
        if exists != present_and_set {
            return false;
        }
    }

    if let Some(min_val) = constraint.get("min") {
        let Some(min) = min_val.as_f64() else {
            return false;
        };

        let Some(target_val) = target.filter(|v| !is_unset_value(v)) else {
            return false;
        };
        let Some(target_num) = target_val.as_f64() else {
            return false;
        };
        if target_num < min {
            return false;
        }
    }

    if let Some(max_val) = constraint.get("max") {
        let Some(max) = max_val.as_f64() else {
            return false;
        };

        let Some(target_val) = target.filter(|v| !is_unset_value(v)) else {
            return false;
        };
        let Some(target_num) = target_val.as_f64() else {
            return false;
        };
        if target_num > max {
            return false;
        }
    }

    if let Some(equals_val) = constraint.get("equals") {
        let Some(target_val) = target else {
            return false;
        };
        if target_val != equals_val {
            return false;
        }
    }

    if let Some(in_val) = constraint.get("in") {
        let serde_json::Value::Array(arr) = in_val else {
            return false;
        };
        let Some(target_val) = target else {
            return false;
        };
        if !arr.iter().any(|v| v == target_val) {
            return false;
        }
    }

    true
}

fn validate_require_object(
    rule: &ValidateRule,
    require: &serde_json::Value,
    resolved_json: &serde_json::Value,
    path: &mut Vec<String>,
    issues: &mut Vec<ValidationIssue>,
) {
    let serde_json::Value::Object(map) = require else {
        return;
    };

    for (key, val) in map {
        path.push(key.clone());

        match val {
            serde_json::Value::Object(obj) if is_constraint_object(obj) => {
                let target = get_value_at_path(resolved_json, path);
                if !constraints_pass(obj, target) {
                    let field = path.join(".");
                    let message = rule
                        .message
                        .clone()
                        .unwrap_or_else(|| format!("Rule violation: {field}"));
                    issues.push(ValidationIssue {
                        field,
                        message,
                        severity: rule.severity,
                    });
                }
            }
            serde_json::Value::Object(_) => {
                validate_require_object(rule, val, resolved_json, path, issues);
            }
            _ => {
                // Non-object leafs are ignored for now; leaf constraints are always objects.
            }
        }

        path.pop();
    }
}

pub fn validate_move_with_rules(
    project: Option<&RulesFile>,
    character: Option<&RulesFile>,
    mv: &crate::schema::State,
) -> Result<Vec<ValidationIssue>, RulesError> {
    let resolved = apply_rules_to_move(project, character, mv)?;

    let mut issues = Vec::new();

    let registry = merged_registry(project, character);
    super::registry::validate_move_registry(&resolved, &registry, &mut issues);

    if let Err(errors) = crate::mcp::validation::validate_move(&resolved) {
        issues.extend(errors.into_iter().map(|err| ValidationIssue {
            field: err.field,
            message: err.message,
            severity: Severity::Error,
        }));
    }

    let resolved_json = serde_json::to_value(&resolved)?;
    for rule in merged_validate_rules(project, character) {
        if !matches_move(&rule.match_spec, &resolved) {
            continue;
        }

        let mut path = Vec::new();
        validate_require_object(&rule, &rule.require, &resolved_json, &mut path, &mut issues);
    }

    Ok(issues)
}

/// Description of a built-in validation rule.
#[derive(Debug, Clone, Serialize)]
pub struct BuiltinValidation {
    /// The field or path being validated.
    pub field: String,
    /// Description of the constraint.
    pub constraint: String,
    /// Error message shown when validation fails.
    pub error_message: String,
}

/// Returns descriptions of all built-in validation rules.
/// These validations always run and cannot be disabled.
pub fn get_builtin_validations() -> Vec<BuiltinValidation> {
    vec![
        // Frame Data
        BuiltinValidation {
            field: "startup".to_string(),
            constraint: "must be >= 1".to_string(),
            error_message: "startup must be at least 1 frame".to_string(),
        },
        BuiltinValidation {
            field: "active".to_string(),
            constraint: "must be >= 1".to_string(),
            error_message: "active must be at least 1 frame".to_string(),
        },
        BuiltinValidation {
            field: "input".to_string(),
            constraint: "must be non-empty".to_string(),
            error_message: "input cannot be empty".to_string(),
        },
        // Hitboxes (Legacy)
        BuiltinValidation {
            field: "hitboxes[i].frames".to_string(),
            constraint: "start <= end".to_string(),
            error_message: "start frame cannot be after end frame".to_string(),
        },
        BuiltinValidation {
            field: "hitboxes[i].frames".to_string(),
            constraint: "end <= total frames".to_string(),
            error_message: "end frame exceeds total frames".to_string(),
        },
        // Hits (v2 Schema)
        BuiltinValidation {
            field: "hits[i].frames".to_string(),
            constraint: "start <= end".to_string(),
            error_message: "start frame cannot be after end frame".to_string(),
        },
        BuiltinValidation {
            field: "hits[i].hitboxes[j].w".to_string(),
            constraint: "must be > 0".to_string(),
            error_message: "width must be greater than 0".to_string(),
        },
        BuiltinValidation {
            field: "hits[i].hitboxes[j].h".to_string(),
            constraint: "must be > 0".to_string(),
            error_message: "height must be greater than 0".to_string(),
        },
        BuiltinValidation {
            field: "hits[i].hitboxes[j].r".to_string(),
            constraint: "must be > 0".to_string(),
            error_message: "radius must be greater than 0".to_string(),
        },
        // Preconditions
        BuiltinValidation {
            field: "preconditions[i] (Meter)".to_string(),
            constraint: "min <= max".to_string(),
            error_message: "meter min cannot be greater than max".to_string(),
        },
        BuiltinValidation {
            field: "preconditions[i] (Charge)".to_string(),
            constraint: "min_frames > 0".to_string(),
            error_message: "charge min_frames must be greater than 0".to_string(),
        },
        BuiltinValidation {
            field: "preconditions[i] (Health)".to_string(),
            constraint: "percent <= 100".to_string(),
            error_message: "health min/max_percent cannot exceed 100".to_string(),
        },
        BuiltinValidation {
            field: "preconditions[i] (Health)".to_string(),
            constraint: "min <= max".to_string(),
            error_message: "health min_percent cannot be greater than max_percent".to_string(),
        },
        // Costs
        BuiltinValidation {
            field: "costs[i].amount".to_string(),
            constraint: "must be > 0".to_string(),
            error_message: "cost amount must be greater than 0".to_string(),
        },
        // Movement
        BuiltinValidation {
            field: "movement".to_string(),
            constraint: "distance or velocity required".to_string(),
            error_message: "movement must have either distance or velocity defined".to_string(),
        },
        BuiltinValidation {
            field: "movement.distance".to_string(),
            constraint: "must be > 0".to_string(),
            error_message: "movement distance must be greater than 0".to_string(),
        },
        // Super Freeze
        BuiltinValidation {
            field: "super_freeze.frames".to_string(),
            constraint: "must be > 0".to_string(),
            error_message: "super_freeze frames must be greater than 0".to_string(),
        },
        BuiltinValidation {
            field: "super_freeze.zoom".to_string(),
            constraint: "must be > 0".to_string(),
            error_message: "super_freeze zoom must be greater than 0".to_string(),
        },
        BuiltinValidation {
            field: "super_freeze.darken".to_string(),
            constraint: "0.0 to 1.0".to_string(),
            error_message: "super_freeze darken must be between 0.0 and 1.0".to_string(),
        },
        // Status Effects
        BuiltinValidation {
            field: "on_hit.status[i].duration".to_string(),
            constraint: "must be > 0".to_string(),
            error_message: "duration must be greater than 0".to_string(),
        },
        BuiltinValidation {
            field: "on_hit.status[i].damage_per_frame".to_string(),
            constraint: "must be > 0".to_string(),
            error_message: "damage_per_frame must be greater than 0".to_string(),
        },
        BuiltinValidation {
            field: "on_hit.status[i].multiplier (Slow)".to_string(),
            constraint: "0.0 to 1.0".to_string(),
            error_message: "slow multiplier must be between 0.0 and 1.0".to_string(),
        },
        // Advanced Hurtboxes
        BuiltinValidation {
            field: "advanced_hurtboxes[i].frames".to_string(),
            constraint: "start <= end".to_string(),
            error_message: "start frame cannot be after end frame".to_string(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::matchers::StringOrVec;

    const RULES_VERSION: u32 = 1;

    fn rules_with_validate(validate: Vec<ValidateRule>) -> RulesFile {
        RulesFile {
            version: RULES_VERSION,
            registry: None,
            apply: Vec::new(),
            validate,
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
    fn test_validate_require_must_be_object() {
        let err = serde_json::from_str::<RulesFile>(
            r#"{
  "version": 1,
  "validate": [
    { "match": {}, "require": 3, "severity": "error" }
  ]
}"#,
        )
        .unwrap_err();

        assert!(err.to_string().contains("expected object"));
    }

    #[test]
    fn test_validate_rule_exists_warning_when_animation_unset() {
        let rules = rules_with_validate(vec![ValidateRule {
            match_spec: MatchSpec {
                r#type: None,
                button: None,
                guard: None,
                tags: None,
                input: None,
            },
            require: serde_json::json!({ "animation": { "exists": true } }),
            severity: Severity::Warning,
            message: None,
        }]);

        let mv = make_valid_move();
        let issues = validate_move_with_rules(Some(&rules), None, &mv).unwrap();

        assert!(issues
            .iter()
            .any(|i| i.field == "animation" && i.severity == Severity::Warning));
    }

    #[test]
    fn test_validate_rule_min_error_on_startup() {
        let rules = rules_with_validate(vec![ValidateRule {
            match_spec: MatchSpec {
                r#type: None,
                button: None,
                guard: None,
                tags: None,
                input: None,
            },
            require: serde_json::json!({ "startup": { "min": 3 } }),
            severity: Severity::Error,
            message: None,
        }]);

        let mut mv = make_valid_move();
        mv.startup = 1;
        let issues = validate_move_with_rules(Some(&rules), None, &mv).unwrap();

        assert!(issues
            .iter()
            .any(|i| i.field == "startup" && i.severity == Severity::Error));
    }

    #[test]
    fn test_validate_rules_run_on_resolved_values() {
        use super::super::apply::ApplyRule;

        let project = RulesFile {
            version: RULES_VERSION,
            registry: None,
            apply: vec![ApplyRule {
                match_spec: MatchSpec {
                    r#type: None,
                    button: None,
                    guard: None,
                    tags: None,
                    input: None,
                },
                set: serde_json::json!({ "hitstop": 8 }),
            }],
            validate: vec![ValidateRule {
                match_spec: MatchSpec {
                    r#type: None,
                    button: None,
                    guard: None,
                    tags: None,
                    input: None,
                },
                require: serde_json::json!({ "hitstop": { "min": 1 } }),
                severity: Severity::Error,
                message: None,
            }],
            properties: None,
            tags: None,
        };

        let mut mv = make_valid_move();
        mv.hitstop = 0;
        let issues = validate_move_with_rules(Some(&project), None, &mv).unwrap();

        assert!(!issues.iter().any(|i| i.field == "hitstop"));
    }

    #[test]
    fn test_validate_rule_max_error() {
        let rules = rules_with_validate(vec![ValidateRule {
            match_spec: MatchSpec {
                r#type: None,
                button: None,
                guard: None,
                tags: None,
                input: None,
            },
            require: serde_json::json!({ "startup": { "max": 5 } }),
            severity: Severity::Error,
            message: None,
        }]);

        let mut mv = make_valid_move();
        mv.startup = 6;
        let issues = validate_move_with_rules(Some(&rules), None, &mv).unwrap();

        assert!(issues
            .iter()
            .any(|i| i.field == "startup" && i.severity == Severity::Error));
    }

    #[test]
    fn test_validate_rule_equals_error() {
        let rules = rules_with_validate(vec![ValidateRule {
            match_spec: MatchSpec {
                r#type: None,
                button: None,
                guard: None,
                tags: None,
                input: None,
            },
            require: serde_json::json!({ "guard": { "equals": "low" } }),
            severity: Severity::Error,
            message: None,
        }]);

        let mut mv = make_valid_move();
        mv.guard = crate::schema::GuardType::Mid;
        let issues = validate_move_with_rules(Some(&rules), None, &mv).unwrap();

        assert!(issues
            .iter()
            .any(|i| i.field == "guard" && i.severity == Severity::Error));
    }

    #[test]
    fn test_validate_rule_in_error() {
        let rules = rules_with_validate(vec![ValidateRule {
            match_spec: MatchSpec {
                r#type: None,
                button: None,
                guard: None,
                tags: None,
                input: None,
            },
            require: serde_json::json!({ "guard": { "in": ["mid", "low"] } }),
            severity: Severity::Error,
            message: None,
        }]);

        let mut mv = make_valid_move();
        mv.guard = crate::schema::GuardType::High;
        let issues = validate_move_with_rules(Some(&rules), None, &mv).unwrap();

        assert!(issues
            .iter()
            .any(|i| i.field == "guard" && i.severity == Severity::Error));
    }

    #[test]
    fn test_validate_rules_character_replaces_project_same_match() {
        let project = rules_with_validate(vec![ValidateRule {
            match_spec: MatchSpec {
                r#type: None,
                button: None,
                guard: None,
                tags: None,
                input: None,
            },
            require: serde_json::json!({ "startup": { "min": 3 } }),
            severity: Severity::Warning,
            message: Some("project".to_string()),
        }]);

        let character = rules_with_validate(vec![ValidateRule {
            match_spec: MatchSpec {
                r#type: None,
                button: None,
                guard: None,
                tags: None,
                input: None,
            },
            require: serde_json::json!({ "startup": { "min": 4 } }),
            severity: Severity::Error,
            message: Some("character".to_string()),
        }]);

        let mut mv = make_valid_move();
        mv.startup = 3;

        let issues = validate_move_with_rules(Some(&project), Some(&character), &mv).unwrap();
        assert!(!issues
            .iter()
            .any(|i| i.message == "project" && i.severity == Severity::Warning));
        assert!(issues
            .iter()
            .any(|i| i.message == "character" && i.severity == Severity::Error));
    }

    #[test]
    fn test_validate_rules_include_builtin_errors_as_error_severity() {
        let mut mv = make_valid_move();
        mv.startup = 0;

        let issues = validate_move_with_rules(None, None, &mv).unwrap();
        assert!(issues
            .iter()
            .any(|i| i.field == "startup" && i.severity == Severity::Error));
    }

    #[test]
    fn test_validate_rule_matching_uses_resolved_move() {
        use super::super::apply::ApplyRule;

        let project = RulesFile {
            version: RULES_VERSION,
            registry: None,
            apply: vec![ApplyRule {
                match_spec: MatchSpec {
                    r#type: None,
                    button: None,
                    guard: None,
                    tags: None,
                    input: Some(StringOrVec::One("236*".to_string())),
                },
                set: serde_json::json!({ "type": "special" }),
            }],
            validate: vec![ValidateRule {
                match_spec: MatchSpec {
                    r#type: Some(StringOrVec::One("special".to_string())),
                    button: None,
                    guard: None,
                    tags: None,
                    input: None,
                },
                require: serde_json::json!({ "hitstop": { "min": 1 } }),
                severity: Severity::Error,
                message: None,
            }],
            properties: None,
            tags: None,
        };

        let mut mv = make_valid_move();
        mv.input = "236P".to_string();
        mv.move_type = None;
        mv.hitstop = 0;

        let issues = validate_move_with_rules(Some(&project), None, &mv).unwrap();
        assert!(issues
            .iter()
            .any(|i| i.field == "hitstop" && i.severity == Severity::Error));
    }

    #[test]
    fn test_get_builtin_validations() {
        let validations = get_builtin_validations();
        // Should have multiple validations
        assert!(validations.len() >= 10);
        // Should include key validations
        assert!(validations.iter().any(|v| v.field == "startup"));
        assert!(validations.iter().any(|v| v.field == "active"));
        assert!(validations.iter().any(|v| v.field == "input"));
    }
}
