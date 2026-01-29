use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

const RULES_VERSION: u32 = 1;

/// Root structure for a Framesmith rules file.
/// Rules files define default values (apply) and validation constraints (validate) for moves.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct RulesFile {
    /// Schema version. Must be 1.
    pub version: u32,
    /// Rules that set default values on matching moves.
    #[serde(default)]
    pub apply: Vec<ApplyRule>,
    /// Rules that enforce constraints on matching moves.
    #[serde(default)]
    pub validate: Vec<ValidateRule>,
}

/// A rule that sets default values on moves matching certain criteria.
/// Only fills in values that are unset (null, empty, or zero).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ApplyRule {
    /// Criteria for which moves this rule applies to.
    #[serde(rename = "match")]
    pub match_spec: MatchSpec,
    /// Key-value pairs to set on matching moves. Nested paths supported.
    #[serde(deserialize_with = "deserialize_object_value")]
    #[schemars(with = "std::collections::HashMap<String, serde_json::Value>")]
    pub set: serde_json::Value,
}

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

/// Specifies which moves a rule applies to. All specified fields must match (AND logic).
/// Within a single field, multiple values use OR logic.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct MatchSpec {
    /// Move type: normal, command_normal, special, super, movement, throw.
    #[serde(rename = "type")]
    pub r#type: Option<StringOrVec>,
    /// Button extracted from input (e.g., "236P" -> "P").
    pub button: Option<StringOrVec>,
    /// Guard type: high, mid, low, unblockable.
    pub guard: Option<StringOrVec>,
    /// Tags that must ALL be present on the move (AND logic).
    pub tags: Option<Vec<String>>,
    /// Input notation with glob pattern support (* matches any, ? matches one char).
    pub input: Option<StringOrVec>,
}

/// A value that can be either a single string or an array of strings.
/// Used for match criteria where OR logic is needed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum StringOrVec {
    /// A single value to match.
    One(String),
    /// Multiple values where any match satisfies the condition (OR logic).
    Many(Vec<String>),
}

/// Severity level for validation rule violations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Errors indicate invalid data that must be fixed.
    Error,
    /// Warnings indicate potential issues but don't block saving.
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationIssue {
    pub field: String,
    pub message: String,
    pub severity: Severity,
}

#[derive(Debug)]
pub enum RulesError {
    Io(std::io::Error),
    Json(serde_json::Error),
    UnsupportedVersion(u32),
}

impl std::fmt::Display for RulesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "io error: {err}"),
            Self::Json(err) => write!(f, "json error: {err}"),
            Self::UnsupportedVersion(version) => {
                write!(f, "unsupported rules version: {version}")
            }
        }
    }
}

impl std::error::Error for RulesError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            Self::Json(err) => Some(err),
            Self::UnsupportedVersion(_) => None,
        }
    }
}

impl From<std::io::Error> for RulesError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for RulesError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

pub fn load_rules_file(path: &std::path::Path) -> Result<Option<RulesFile>, RulesError> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(err.into()),
    };
    let rules = serde_json::from_slice::<RulesFile>(&bytes)?;

    if rules.version != RULES_VERSION {
        return Err(RulesError::UnsupportedVersion(rules.version));
    }

    Ok(Some(rules))
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

fn glob_match(pattern: &str, text: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let t: Vec<char> = text.chars().collect();

    let mut dp = vec![vec![false; t.len() + 1]; p.len() + 1];
    dp[0][0] = true;

    for i in 1..=p.len() {
        if p[i - 1] == '*' {
            dp[i][0] = dp[i - 1][0];
        } else {
            break;
        }
    }

    for i in 1..=p.len() {
        for j in 1..=t.len() {
            dp[i][j] = match p[i - 1] {
                '*' => dp[i - 1][j] || dp[i][j - 1],
                '?' => dp[i - 1][j - 1],
                c => dp[i - 1][j - 1] && c == t[j - 1],
            };
        }
    }

    dp[p.len()][t.len()]
}

fn match_any<P>(patterns: &StringOrVec, value: &str, pred: P) -> bool
where
    P: Fn(&str, &str) -> bool,
{
    match patterns {
        StringOrVec::One(p) => pred(p, value),
        StringOrVec::Many(ps) => ps.iter().any(|p| pred(p, value)),
    }
}

fn button_from_input(input: &str) -> Option<&str> {
    let bytes = input.as_bytes();
    let mut i = bytes.len();
    while i > 0 && bytes[i - 1].is_ascii_alphabetic() {
        i -= 1;
    }
    if i == bytes.len() {
        None
    } else {
        Some(&input[i..])
    }
}

pub fn matches_move(spec: &MatchSpec, mv: &crate::schema::Move) -> bool {
    if let Some(ty) = &spec.r#type {
        let mv_type = match mv.move_type {
            Some(crate::schema::MoveType::Normal) => "normal",
            Some(crate::schema::MoveType::CommandNormal) => "command_normal",
            Some(crate::schema::MoveType::Special) => "special",
            Some(crate::schema::MoveType::Super) => "super",
            Some(crate::schema::MoveType::Movement) => "movement",
            Some(crate::schema::MoveType::Throw) => "throw",
            None => return false,
        };

        if !match_any(ty, mv_type, |p, v| p == v) {
            return false;
        }
    }

    if let Some(guard) = &spec.guard {
        let mv_guard = match mv.guard {
            crate::schema::GuardType::High => "high",
            crate::schema::GuardType::Mid => "mid",
            crate::schema::GuardType::Low => "low",
            crate::schema::GuardType::Unblockable => "unblockable",
        };

        if !match_any(guard, mv_guard, |p, v| p == v) {
            return false;
        }
    }

    if let Some(input) = &spec.input {
        if !match_any(input, &mv.input, glob_match) {
            return false;
        }
    }

    if let Some(button) = &spec.button {
        let mv_button = match button_from_input(&mv.input) {
            Some(b) => b,
            None => return false,
        };

        if !match_any(button, mv_button, |p, v| p == v) {
            return false;
        }
    }

    if let Some(tags) = &spec.tags {
        if !tags.iter().all(|t| mv.tags.contains(t)) {
            return false;
        }
    }

    true
}

pub fn merged_apply_rules(
    project: Option<&RulesFile>,
    character: Option<&RulesFile>,
) -> Vec<ApplyRule> {
    let mut merged = project.map(|rules| rules.apply.clone()).unwrap_or_default();

    let character_apply = match character {
        Some(rules) => &rules.apply,
        None => return merged,
    };

    for rule in character_apply {
        merged.retain(|existing| existing.match_spec != rule.match_spec);
    }

    merged.extend(character_apply.iter().cloned());
    merged
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

fn set_value_at_path(root: &mut serde_json::Value, path: &[String], value: serde_json::Value) {
    if path.is_empty() {
        return;
    }

    let mut cur = root;
    for key in &path[..path.len() - 1] {
        if !matches!(cur, serde_json::Value::Object(_)) {
            *cur = serde_json::Value::Object(serde_json::Map::new());
        }

        let serde_json::Value::Object(map) = cur else {
            unreachable!("cur forced to object");
        };

        cur = map
            .entry(key.clone())
            .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));

        if !matches!(cur, serde_json::Value::Object(_)) {
            *cur = serde_json::Value::Object(serde_json::Map::new());
        }
    }

    let leaf_key = &path[path.len() - 1];
    if !matches!(cur, serde_json::Value::Object(_)) {
        *cur = serde_json::Value::Object(serde_json::Map::new());
    }

    let serde_json::Value::Object(map) = cur else {
        unreachable!("cur forced to object");
    };
    map.insert(leaf_key.clone(), value);
}

fn apply_set_object(
    set: &serde_json::Value,
    base: &serde_json::Value,
    resolved: &mut serde_json::Value,
    path: &mut Vec<String>,
) {
    match set {
        serde_json::Value::Object(map) => {
            for (key, value) in map {
                path.push(key.clone());
                apply_set_object(value, base, resolved, path);
                path.pop();
            }
        }
        leaf => {
            let base_value = get_value_at_path(base, path);
            if base_value.map_or(true, is_unset_value) {
                set_value_at_path(resolved, path, leaf.clone());
            }
        }
    }
}

pub fn apply_rules_to_move(
    project: Option<&RulesFile>,
    character: Option<&RulesFile>,
    mv: &crate::schema::Move,
) -> Result<crate::schema::Move, RulesError> {
    let base = serde_json::to_value(mv)?;
    let mut resolved = base.clone();

    for rule in merged_apply_rules(project, character) {
        if !matches_move(&rule.match_spec, mv) {
            continue;
        }

        let mut path = Vec::new();
        apply_set_object(&rule.set, &base, &mut resolved, &mut path);
    }

    Ok(serde_json::from_value(resolved)?)
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
    mv: &crate::schema::Move,
) -> Result<Vec<ValidationIssue>, RulesError> {
    let resolved = apply_rules_to_move(project, character, mv)?;

    let mut issues = Vec::new();

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

/// Generates the JSON Schema for RulesFile.
pub fn generate_rules_schema() -> schemars::Schema {
    schemars::schema_for!(RulesFile)
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

    #[test]
    fn test_parse_match_string_and_array() {
        let rules: RulesFile = serde_json::from_str(
            r#"{
  "version": 1,
  "apply": [
    {
      "match": { "type": ["normal", "special"], "button": "L", "input": ["5L", "2L"] },
      "set": { "hitstop": 3 }
    }
  ],
  "validate": [
    {
      "match": { "guard": "mid", "tags": ["starter", "reversal"] },
      "require": { "animation": { "exists": true } },
      "severity": "warning",
      "message": "needs animation"
    }
  ]
}"#,
        )
        .unwrap();

        assert_eq!(rules.apply.len(), 1);
        assert_eq!(rules.validate.len(), 1);

        let m = &rules.apply[0].match_spec;
        assert_eq!(
            m.r#type,
            Some(StringOrVec::Many(vec![
                "normal".to_string(),
                "special".to_string()
            ]))
        );
        assert_eq!(m.button, Some(StringOrVec::One("L".to_string())));
        assert_eq!(
            m.input,
            Some(StringOrVec::Many(vec!["5L".to_string(), "2L".to_string()]))
        );

        assert!(matches!(rules.apply[0].set, serde_json::Value::Object(_)));
        assert!(matches!(
            rules.validate[0].require,
            serde_json::Value::Object(_)
        ));

        let vm = &rules.validate[0].match_spec;
        assert_eq!(vm.guard, Some(StringOrVec::One("mid".to_string())));
        assert_eq!(
            vm.tags,
            Some(vec!["starter".to_string(), "reversal".to_string()])
        );
    }

    #[test]
    fn test_missing_sections_default_empty() {
        let rules: RulesFile = serde_json::from_str(r#"{"version": 1}"#).unwrap();
        assert!(rules.apply.is_empty());
        assert!(rules.validate.is_empty());
    }

    #[test]
    fn test_load_rules_file_unsupported_version() {
        let mut path = std::env::temp_dir();
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        path.push(format!(
            "framesmith_rules_unsupported_version_{unique}.json"
        ));

        std::fs::write(&path, r#"{"version": 2}"#).unwrap();

        let res = load_rules_file(&path);
        assert!(matches!(res, Err(RulesError::UnsupportedVersion(2))));

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_apply_set_must_be_object() {
        let err = serde_json::from_str::<RulesFile>(
            r#"{
  "version": 1,
  "apply": [
    { "match": {}, "set": 3 }
  ]
}"#,
        )
        .unwrap_err();

        assert!(err.to_string().contains("expected object"));
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
    fn test_glob_match() {
        assert!(glob_match("236*", "236P"));
        assert!(!glob_match("236*", "623P"));

        assert!(glob_match("*P", "5P"));
        assert!(glob_match("*P", "623P"));
        assert!(!glob_match("*P", "5K"));

        assert!(glob_match("5?", "5L"));
        assert!(glob_match("5?", "5M"));
        assert!(!glob_match("5?", "5LL"));

        assert!(glob_match("236236*", "236236K"));
        assert!(!glob_match("236236*", "236K"));

        assert!(glob_match("[*]*", "[4]6P"));
    }

    #[test]
    fn test_matches_move_or_within_field_and_across_fields() {
        let mut mv = crate::schema::Move::default();
        mv.input = "2L".to_string();
        mv.move_type = Some(crate::schema::MoveType::CommandNormal);
        mv.guard = crate::schema::GuardType::Unblockable;

        // OR within a field
        let spec = MatchSpec {
            r#type: None,
            button: None,
            guard: None,
            tags: None,
            input: Some(StringOrVec::Many(vec!["5L".to_string(), "2L".to_string()])),
        };
        assert!(matches_move(&spec, &mv));

        // AND across fields
        let spec = MatchSpec {
            r#type: Some(StringOrVec::One("command_normal".to_string())),
            button: None,
            guard: Some(StringOrVec::One("unblockable".to_string())),
            tags: None,
            input: Some(StringOrVec::One("2L".to_string())),
        };
        assert!(matches_move(&spec, &mv));

        // AND across fields - guard mismatch
        let spec = MatchSpec {
            guard: Some(StringOrVec::One("mid".to_string())),
            ..spec
        };
        assert!(!matches_move(&spec, &mv));
    }

    #[test]
    fn test_matches_move_button_extraction() {
        let mut mv = crate::schema::Move::default();
        mv.input = "j.H".to_string();

        let spec = MatchSpec {
            r#type: None,
            button: Some(StringOrVec::One("H".to_string())),
            guard: None,
            tags: None,
            input: None,
        };

        assert!(matches_move(&spec, &mv));

        mv.input = "632146PP".to_string();
        let spec = MatchSpec {
            r#type: None,
            button: Some(StringOrVec::One("PP".to_string())),
            guard: None,
            tags: None,
            input: None,
        };

        assert!(matches_move(&spec, &mv));
    }

    #[test]
    fn test_matches_move_tags_and() {
        let mut mv = crate::schema::Move::default();
        mv.input = "5L".to_string();
        mv.tags = vec!["starter".to_string(), "reversal".to_string()];

        let spec = MatchSpec {
            r#type: None,
            button: None,
            guard: None,
            tags: Some(vec!["starter".to_string(), "reversal".to_string()]),
            input: None,
        };
        assert!(matches_move(&spec, &mv));

        let spec = MatchSpec {
            r#type: None,
            button: None,
            guard: None,
            tags: Some(vec!["starter".to_string(), "missing".to_string()]),
            input: None,
        };
        assert!(!matches_move(&spec, &mv));
    }

    fn rules_with_apply(apply: Vec<ApplyRule>) -> RulesFile {
        RulesFile {
            version: RULES_VERSION,
            apply,
            validate: Vec::new(),
        }
    }

    fn rules_with_validate(validate: Vec<ValidateRule>) -> RulesFile {
        RulesFile {
            version: RULES_VERSION,
            apply: Vec::new(),
            validate,
        }
    }

    fn make_valid_move() -> crate::schema::Move {
        let mut mv = crate::schema::Move::default();
        mv.input = "5L".to_string();
        mv.startup = 1;
        mv.active = 1;
        mv
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
        let project = RulesFile {
            version: RULES_VERSION,
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
        let project = RulesFile {
            version: RULES_VERSION,
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
    fn test_apply_rules_stacking_overrides_earlier_rules() {
        let project = rules_with_apply(vec![
            ApplyRule {
                match_spec: MatchSpec {
                    r#type: None,
                    button: None,
                    guard: None,
                    tags: None,
                    input: None,
                },
                set: serde_json::json!({ "hitstop": 8 }),
            },
            ApplyRule {
                match_spec: MatchSpec {
                    r#type: Some(StringOrVec::One("special".to_string())),
                    button: None,
                    guard: None,
                    tags: None,
                    input: None,
                },
                set: serde_json::json!({ "hitstop": 10 }),
            },
        ]);

        let mut mv = crate::schema::Move::default();
        mv.input = "236P".to_string();
        mv.move_type = Some(crate::schema::MoveType::Special);
        mv.hitstop = 0;

        let resolved = apply_rules_to_move(Some(&project), None, &mv).unwrap();
        assert_eq!(resolved.hitstop, 10);
    }

    #[test]
    fn test_apply_rules_does_not_override_explicit_move_values() {
        let project = rules_with_apply(vec![ApplyRule {
            match_spec: MatchSpec {
                r#type: None,
                button: None,
                guard: None,
                tags: None,
                input: None,
            },
            set: serde_json::json!({ "hitstop": 8 }),
        }]);

        let mut mv = crate::schema::Move::default();
        mv.input = "5L".to_string();
        mv.hitstop = 6;

        let resolved = apply_rules_to_move(Some(&project), None, &mv).unwrap();
        assert_eq!(resolved.hitstop, 6);
    }

    #[test]
    fn test_character_apply_rule_replaces_project_rule_with_same_match() {
        let match_normal = MatchSpec {
            r#type: Some(StringOrVec::One("normal".to_string())),
            button: None,
            guard: None,
            tags: None,
            input: None,
        };

        let project = rules_with_apply(vec![ApplyRule {
            match_spec: match_normal.clone(),
            set: serde_json::json!({ "hitstop": 8 }),
        }]);
        let character = rules_with_apply(vec![ApplyRule {
            match_spec: match_normal,
            set: serde_json::json!({ "hitstop": 9 }),
        }]);

        let mut mv = crate::schema::Move::default();
        mv.input = "5L".to_string();
        mv.move_type = Some(crate::schema::MoveType::Normal);
        mv.hitstop = 0;

        let resolved = apply_rules_to_move(Some(&project), Some(&character), &mv).unwrap();
        assert_eq!(resolved.hitstop, 9);
    }

    #[test]
    fn test_generate_rules_schema() {
        let schema = generate_rules_schema();
        let json = serde_json::to_string_pretty(&schema).unwrap();
        // Schema should contain key type names
        assert!(json.contains("RulesFile"));
        assert!(json.contains("ApplyRule"));
        assert!(json.contains("ValidateRule"));
        assert!(json.contains("MatchSpec"));
        assert!(json.contains("Severity"));
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
