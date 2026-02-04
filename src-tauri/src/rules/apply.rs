use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{matchers::MatchSpec, RulesError, RulesFile};

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
            if base_value.is_none_or(is_unset_value) {
                set_value_at_path(resolved, path, leaf.clone());
            }
        }
    }
}

pub fn apply_rules_to_move(
    project: Option<&RulesFile>,
    character: Option<&RulesFile>,
    mv: &crate::schema::State,
) -> Result<crate::schema::State, RulesError> {
    let base = serde_json::to_value(mv)?;
    let mut resolved = base.clone();

    for rule in merged_apply_rules(project, character) {
        if !super::matchers::matches_move(&rule.match_spec, mv) {
            continue;
        }

        let mut path = Vec::new();
        apply_set_object(&rule.set, &base, &mut resolved, &mut path);
    }

    Ok(serde_json::from_value(resolved)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::matchers::StringOrVec;

    const RULES_VERSION: u32 = 1;

    fn rules_with_apply(apply: Vec<ApplyRule>) -> RulesFile {
        RulesFile {
            version: RULES_VERSION,
            registry: None,
            apply,
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

        let mut mv = crate::schema::State::default();
        mv.input = "236P".to_string();
        mv.move_type = Some("special".to_string());
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

        let mut mv = crate::schema::State::default();
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

        let mut mv = crate::schema::State::default();
        mv.input = "5L".to_string();
        mv.move_type = Some("normal".to_string());
        mv.hitstop = 0;

        let resolved = apply_rules_to_move(Some(&project), Some(&character), &mv).unwrap();
        assert_eq!(resolved.hitstop, 9);
    }
}
