//! Variant resolution for state inheritance.
//!
//! Variants allow states to inherit from base states with targeted overrides.
//! Filename convention: `{base}~{variant}.json` (e.g., `5H~level1.json`)

use crate::schema::State;
use std::collections::HashSet;

/// Check if a JSON value is "default-like" and should be skipped during merge.
/// This prevents Rust's `Default::default()` fields from overwriting base values.
fn is_default_value(v: &serde_json::Value) -> bool {
    use serde_json::Value;
    match v {
        Value::Null => true,
        Value::String(s) => s.is_empty(),
        Value::Array(arr) => arr.is_empty(),
        Value::Object(obj) => obj.is_empty(),
        Value::Number(n) => n.as_u64() == Some(0) || n.as_i64() == Some(0) || n.as_f64() == Some(0.0),
        Value::Bool(_) => false, // bools are never considered default (false is meaningful)
    }
}

/// Deep merge two JSON values.
/// - Objects: recursively merge (overlay fields override base fields)
/// - Arrays: overlay replaces base entirely (no element merging)
/// - Scalars: overlay replaces base
/// - Null/default values in overlay are skipped (don't override base)
fn deep_merge(base: serde_json::Value, overlay: serde_json::Value) -> serde_json::Value {
    use serde_json::Value;

    match (base, overlay) {
        (Value::Object(mut base_map), Value::Object(overlay_map)) => {
            for (key, overlay_val) in overlay_map {
                if is_default_value(&overlay_val) {
                    // Skip default/null overlay values - keep base value
                    continue;
                } else if let Some(base_val) = base_map.remove(&key) {
                    base_map.insert(key, deep_merge(base_val, overlay_val));
                } else {
                    base_map.insert(key, overlay_val);
                }
            }
            Value::Object(base_map)
        }
        (_, overlay) => overlay,
    }
}

/// Resolve a variant by merging overlay onto base state.
pub fn resolve_variant(base: &State, overlay: &State, resolved_id: &str) -> State {
    let base_json = serde_json::to_value(base).expect("base should serialize");
    let overlay_json = serde_json::to_value(overlay).expect("overlay should serialize");

    let mut merged = deep_merge(base_json, overlay_json);

    if let serde_json::Value::Object(ref mut map) = merged {
        map.insert("id".to_string(), serde_json::Value::String(resolved_id.to_string()));
        map.remove("base");
        if map.get("input").map(|v| v.as_str() == Some("")).unwrap_or(true) {
            map.insert("input".to_string(), serde_json::Value::String(base.input.clone()));
        }
    }

    serde_json::from_value(merged).expect("merged state should deserialize")
}

/// Parse a state name into (base, variant) components.
///
/// Splits on the **last** tilde. If the portion after the last tilde is empty,
/// treats the whole name as a base state (e.g., `5S~` is a hold input, not a variant).
pub fn parse_variant_name(name: &str) -> (&str, Option<&str>) {
    match name.rfind('~') {
        Some(pos) => {
            let variant_part = &name[pos + 1..];
            if variant_part.is_empty() {
                (name, None)
            } else {
                (&name[..pos], Some(variant_part))
            }
        }
        None => (name, None),
    }
}

/// Check if a filename represents a variant (has non-empty variant portion).
pub fn is_variant_filename(name: &str) -> bool {
    parse_variant_name(name).1.is_some()
}

/// Validate variant states have existing bases and matching base fields.
pub fn validate_variants(
    states: &[(String, State)],
    base_names: &HashSet<String>,
) -> Vec<String> {
    let mut errors = Vec::new();

    for (name, state) in states {
        if let Some(ref declared_base) = state.base {
            let (implied_base, variant_part) = parse_variant_name(name);

            if !base_names.contains(declared_base) {
                errors.push(format!(
                    "Variant '{}': Base state '{}' not found",
                    name, declared_base
                ));
            }

            if variant_part.is_some() && declared_base != implied_base {
                errors.push(format!(
                    "Variant '{}': Base field '{}' doesn't match filename implied base '{}'",
                    name, declared_base, implied_base
                ));
            }
        }
    }

    errors
}

/// Validate that variants don't inherit from other variants (single-level only).
pub fn validate_variants_no_chain(
    states: &[(String, State)],
    base_names: &HashSet<String>,
    variant_names: &HashSet<String>,
) -> Vec<String> {
    let mut errors = Vec::new();

    for (name, state) in states {
        if let Some(ref declared_base) = state.base {
            let (implied_base, variant_part) = parse_variant_name(name);

            // Check for chaining first - if base is a variant, that's the error
            if variant_names.contains(declared_base) {
                errors.push(format!(
                    "Variant '{}': Variants cannot inherit from another variant ('{}')",
                    name, declared_base
                ));
                continue;
            }

            // Only check base existence if not chaining
            if !base_names.contains(declared_base) {
                errors.push(format!(
                    "Variant '{}': Base state '{}' not found",
                    name, declared_base
                ));
            }

            if variant_part.is_some() && declared_base != implied_base {
                errors.push(format!(
                    "Variant '{}': Base field '{}' doesn't match filename implied base '{}'",
                    name, declared_base, implied_base
                ));
            }
        }
    }

    errors
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_base_state_no_tilde() {
        let (base, variant) = parse_variant_name("5H");
        assert_eq!(base, "5H");
        assert_eq!(variant, None);
    }

    #[test]
    fn parse_simple_variant() {
        let (base, variant) = parse_variant_name("5H~level1");
        assert_eq!(base, "5H");
        assert_eq!(variant, Some("level1"));
    }

    #[test]
    fn parse_hold_notation_as_base() {
        let (base, variant) = parse_variant_name("5S~");
        assert_eq!(base, "5S~");
        assert_eq!(variant, None);
    }

    #[test]
    fn parse_hold_variant() {
        let (base, variant) = parse_variant_name("5S~~installed");
        assert_eq!(base, "5S~");
        assert_eq!(variant, Some("installed"));
    }

    #[test]
    fn parse_rekka_notation() {
        let (base, variant) = parse_variant_name("236K~K");
        assert_eq!(base, "236K");
        assert_eq!(variant, Some("K"));
    }

    #[test]
    fn is_variant_checks_correctly() {
        assert!(!is_variant_filename("5H"));
        assert!(is_variant_filename("5H~level1"));
        assert!(!is_variant_filename("5S~"));
        assert!(is_variant_filename("5S~~installed"));
    }

    use crate::schema::{FrameHitbox, OnHit, Rect, State};

    #[test]
    fn merge_scalars_override() {
        let base = State {
            input: "5H".to_string(),
            name: "Standing Heavy".to_string(),
            damage: 50,
            hitstun: 20,
            ..Default::default()
        };
        let overlay = State {
            damage: 80,
            ..Default::default()
        };

        let resolved = resolve_variant(&base, &overlay, "5H~level1");

        assert_eq!(resolved.id.as_deref(), Some("5H~level1"));
        assert_eq!(resolved.input, "5H");
        assert_eq!(resolved.name, "Standing Heavy");
        assert_eq!(resolved.damage, 80);
        assert_eq!(resolved.hitstun, 20);
    }

    #[test]
    fn merge_objects_deep() {
        let base = State {
            on_hit: Some(OnHit {
                gain_meter: Some(10),
                ground_bounce: Some(false),
                ..Default::default()
            }),
            ..Default::default()
        };
        let overlay = State {
            on_hit: Some(OnHit {
                ground_bounce: Some(true),
                wall_bounce: Some(true),
                ..Default::default()
            }),
            ..Default::default()
        };

        let resolved = resolve_variant(&base, &overlay, "5H~level1");

        let on_hit = resolved.on_hit.unwrap();
        assert_eq!(on_hit.gain_meter, Some(10));
        assert_eq!(on_hit.ground_bounce, Some(true));
        assert_eq!(on_hit.wall_bounce, Some(true));
    }

    #[test]
    fn merge_arrays_replace() {
        let base = State {
            hitboxes: vec![FrameHitbox {
                frames: (8, 12),
                r#box: Rect { x: 0, y: -50, w: 40, h: 20 },
            }],
            ..Default::default()
        };
        let overlay = State {
            hitboxes: vec![FrameHitbox {
                frames: (8, 14),
                r#box: Rect { x: 0, y: -55, w: 50, h: 25 },
            }],
            ..Default::default()
        };

        let resolved = resolve_variant(&base, &overlay, "5H~level1");

        assert_eq!(resolved.hitboxes.len(), 1);
        assert_eq!(resolved.hitboxes[0].frames, (8, 14));
    }

    #[test]
    fn merge_inherits_input_from_base() {
        let base = State {
            input: "5H".to_string(),
            ..Default::default()
        };
        let overlay = State {
            damage: 80,
            ..Default::default()
        };

        let resolved = resolve_variant(&base, &overlay, "5H~level1");

        assert_eq!(resolved.input, "5H");
    }

    #[test]
    fn validate_base_exists() {
        let states = vec![
            ("5H~level1".to_string(), State { base: Some("5H".to_string()), ..Default::default() }),
        ];
        let base_names: std::collections::HashSet<_> = std::iter::empty().collect();

        let errors = validate_variants(&states, &base_names);

        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("Base state '5H' not found"));
    }

    #[test]
    fn validate_base_field_matches_filename() {
        let states = vec![
            ("5H~level1".to_string(), State { base: Some("2H".to_string()), ..Default::default() }),
        ];
        let base_names: std::collections::HashSet<_> = ["5H".to_string(), "2H".to_string()].into_iter().collect();

        let errors = validate_variants(&states, &base_names);

        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("doesn't match filename"));
    }

    #[test]
    fn validate_no_chained_inheritance() {
        let states = vec![
            ("5H~level1".to_string(), State { base: Some("5H".to_string()), ..Default::default() }),
            ("5H~level1~enhanced".to_string(), State { base: Some("5H~level1".to_string()), ..Default::default() }),
        ];
        let base_names: std::collections::HashSet<_> = ["5H".to_string()].into_iter().collect();
        let variant_names: std::collections::HashSet<_> = ["5H~level1".to_string()].into_iter().collect();

        let errors = validate_variants_no_chain(&states, &base_names, &variant_names);

        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("cannot inherit from another variant"));
    }

    #[test]
    fn validate_passes_for_valid_variant() {
        let states = vec![
            ("5H~level1".to_string(), State { base: Some("5H".to_string()), ..Default::default() }),
        ];
        let base_names: std::collections::HashSet<_> = ["5H".to_string()].into_iter().collect();

        let errors = validate_variants(&states, &base_names);

        assert!(errors.is_empty());
    }
}
