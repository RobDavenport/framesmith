//! Property packing for CHARACTER_PROPS and STATE_PROPS sections.
//!
//! Both sections use the same fixed 12-byte record format for zero-copy reading.
//! Nested Object properties are flattened using dot notation (e.g., `movement.distance`).
//! Arrays are flattened with index notation (e.g., `effects.0`, `effects.1`).
//!
//! When a schema is provided, an alternative 8-byte format is used where property
//! names are replaced with schema IDs, reducing per-property overhead.

use std::collections::{BTreeMap, HashMap};

use crate::codegen::fspk_format::{
    to_q24_8, write_u16_le, write_u32_le, write_u8, PROP_TYPE_BOOL, PROP_TYPE_Q24_8, PROP_TYPE_STR,
};
use crate::schema::{Character, PropertyValue};

use super::types::StringTable;
use super::utils::checked_u16;

/// Flatten a PropertyValue map into dot-notation keys.
///
/// - `{"movement": {"distance": 80}}` becomes `{"movement.distance": 80}`
/// - `{"effects": [1, 2]}` becomes `{"effects.0": 1, "effects.1": 2}`
fn flatten_properties(
    props: &BTreeMap<String, PropertyValue>,
) -> BTreeMap<String, PropertyValue> {
    let mut flat = BTreeMap::new();
    flatten_into("", props, &mut flat);
    flat
}

fn flatten_into(
    prefix: &str,
    props: &BTreeMap<String, PropertyValue>,
    out: &mut BTreeMap<String, PropertyValue>,
) {
    for (key, value) in props {
        let full_key = if prefix.is_empty() {
            key.clone()
        } else {
            format!("{}.{}", prefix, key)
        };

        match value {
            PropertyValue::Object(nested) => {
                flatten_into(&full_key, nested, out);
            }
            PropertyValue::Array(items) => {
                for (i, item) in items.iter().enumerate() {
                    let indexed_key = format!("{}.{}", full_key, i);
                    flatten_value(&indexed_key, item, out);
                }
            }
            // Scalar values go directly
            _ => {
                out.insert(full_key, value.clone());
            }
        }
    }
}

fn flatten_value(key: &str, value: &PropertyValue, out: &mut BTreeMap<String, PropertyValue>) {
    match value {
        PropertyValue::Object(nested) => {
            flatten_into(key, nested, out);
        }
        PropertyValue::Array(items) => {
            for (i, item) in items.iter().enumerate() {
                let indexed_key = format!("{}.{}", key, i);
                flatten_value(&indexed_key, item, out);
            }
        }
        _ => {
            out.insert(key.to_string(), value.clone());
        }
    }
}

/// Pack a flattened property map into fixed 12-byte records.
///
/// Each property record is 12 bytes:
/// - bytes 0-3: name offset (u32) into string pool
/// - bytes 4-5: name length (u16)
/// - byte 6: value type (0=Q24.8, 1=bool, 2=string ref)
/// - byte 7: reserved
/// - bytes 8-11: value (i32 for Q24.8, u32 for bool, packed strref for string)
fn pack_flat_properties(
    props: &BTreeMap<String, PropertyValue>,
    strings: &mut StringTable,
) -> Result<Vec<u8>, String> {
    let mut data = Vec::with_capacity(props.len() * 12);

    for (name, value) in props {
        let (name_off, name_len) = strings.intern(name)?;
        write_u32_le(&mut data, name_off);
        write_u16_le(&mut data, name_len);

        match value {
            PropertyValue::Number(n) => {
                write_u8(&mut data, PROP_TYPE_Q24_8);
                write_u8(&mut data, 0);
                let q24_8 = to_q24_8(*n);
                data.extend_from_slice(&q24_8.to_le_bytes());
            }
            PropertyValue::Bool(b) => {
                write_u8(&mut data, PROP_TYPE_BOOL);
                write_u8(&mut data, 0);
                let val: u32 = if *b { 1 } else { 0 };
                write_u32_le(&mut data, val);
            }
            PropertyValue::String(s) => {
                write_u8(&mut data, PROP_TYPE_STR);
                write_u8(&mut data, 0);
                let (str_off, str_len) = strings.intern(s)?;
                let str_off_u16 = checked_u16(str_off as usize, "string property offset")?;
                write_u16_le(&mut data, str_off_u16);
                write_u16_le(&mut data, str_len);
            }
            PropertyValue::Array(_) | PropertyValue::Object(_) => {
                // Should never happen after flattening
                return Err(format!(
                    "property '{}' has nested value after flattening (bug)",
                    name
                ));
            }
        }
    }

    Ok(data)
}

/// Pack character properties into the CHARACTER_PROPS section.
///
/// Nested properties are flattened using dot notation before packing.
pub fn pack_character_props(
    character: &Character,
    strings: &mut StringTable,
) -> Result<Vec<u8>, String> {
    if character.properties.is_empty() {
        return Ok(Vec::new());
    }

    let flat = flatten_properties(&character.properties);
    pack_flat_properties(&flat, strings)
}

/// Pack state properties into fixed 12-byte records.
///
/// Nested properties are flattened using dot notation before packing.
/// Returns (data, count) where count is the number of property records.
pub fn pack_state_props(
    props: &BTreeMap<String, PropertyValue>,
    strings: &mut StringTable,
) -> Result<(Vec<u8>, u16), String> {
    if props.is_empty() {
        return Ok((Vec::new(), 0));
    }

    let flat = flatten_properties(props);
    let count = checked_u16(flat.len(), "state property count")?;
    let data = pack_flat_properties(&flat, strings)?;
    Ok((data, count))
}

// =============================================================================
// Schema-Based Property Packing (8-byte records)
// =============================================================================

/// Find similar strings using simple Levenshtein distance and substring matching.
///
/// Returns up to 3 suggestions sorted by similarity.
pub fn find_similar<'a>(needle: &str, haystack: &[&'a str]) -> Vec<&'a str> {
    let needle_lower = needle.to_lowercase();
    let mut candidates: Vec<(&str, usize)> = haystack
        .iter()
        .filter_map(|&s| {
            let s_lower = s.to_lowercase();
            // Substring match gets priority (distance 0)
            if s_lower.contains(&needle_lower) || needle_lower.contains(&s_lower) {
                return Some((s, 0));
            }
            // Compute Levenshtein distance
            let dist = levenshtein(&needle_lower, &s_lower);
            // Only suggest if distance is reasonable (less than half the string length)
            let max_dist = needle.len().max(s.len()) / 2 + 1;
            if dist <= max_dist {
                Some((s, dist))
            } else {
                None
            }
        })
        .collect();

    candidates.sort_by_key(|&(_, dist)| dist);
    candidates.into_iter().take(3).map(|(s, _)| s).collect()
}

/// Simple Levenshtein distance implementation.
fn levenshtein(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let m = a_chars.len();
    let n = b_chars.len();

    if m == 0 {
        return n;
    }
    if n == 0 {
        return m;
    }

    let mut dp = vec![vec![0usize; n + 1]; m + 1];

    for (i, row) in dp.iter_mut().enumerate().take(m + 1) {
        row[0] = i;
    }
    for (j, val) in dp[0].iter_mut().enumerate().take(n + 1) {
        *val = j;
    }

    for i in 1..=m {
        for j in 1..=n {
            let cost = if a_chars[i - 1] == b_chars[j - 1] {
                0
            } else {
                1
            };
            dp[i][j] = (dp[i - 1][j] + 1)
                .min(dp[i][j - 1] + 1)
                .min(dp[i - 1][j - 1] + cost);
        }
    }

    dp[m][n]
}

/// Pack a flattened property map into 8-byte schema-based records.
///
/// Each property record is 8 bytes:
/// - bytes 0-1: schema_id (u16) - index into the schema's property list
/// - byte 2: value type (0=Q24.8, 1=bool, 2=string ref)
/// - byte 3: reserved (0)
/// - bytes 4-7: value (i32 for Q24.8, u32 for bool, packed strref for string)
///
/// Returns an error if a property name is not found in the schema.
fn pack_flat_properties_with_schema(
    props: &BTreeMap<String, PropertyValue>,
    schema_lookup: &HashMap<String, u16>,
    schema_names: &[&str],
    strings: &mut StringTable,
) -> Result<Vec<u8>, String> {
    let mut data = Vec::with_capacity(props.len() * 8);

    for (name, value) in props {
        let schema_id = match schema_lookup.get(name) {
            Some(&id) => id,
            None => {
                let suggestions = find_similar(name, schema_names);
                let suggestion_text = if suggestions.is_empty() {
                    String::new()
                } else {
                    format!(". Did you mean: {}?", suggestions.join(", "))
                };
                return Err(format!(
                    "Property '{}' is not defined in the schema{}",
                    name, suggestion_text
                ));
            }
        };

        write_u16_le(&mut data, schema_id);

        match value {
            PropertyValue::Number(n) => {
                write_u8(&mut data, PROP_TYPE_Q24_8);
                write_u8(&mut data, 0);
                let q24_8 = to_q24_8(*n);
                data.extend_from_slice(&q24_8.to_le_bytes());
            }
            PropertyValue::Bool(b) => {
                write_u8(&mut data, PROP_TYPE_BOOL);
                write_u8(&mut data, 0);
                let val: u32 = if *b { 1 } else { 0 };
                write_u32_le(&mut data, val);
            }
            PropertyValue::String(s) => {
                write_u8(&mut data, PROP_TYPE_STR);
                write_u8(&mut data, 0);
                let (str_off, str_len) = strings.intern(s)?;
                let str_off_u16 = checked_u16(str_off as usize, "string property offset")?;
                write_u16_le(&mut data, str_off_u16);
                write_u16_le(&mut data, str_len);
            }
            PropertyValue::Array(_) | PropertyValue::Object(_) => {
                // Should never happen after flattening
                return Err(format!(
                    "property '{}' has nested value after flattening (bug)",
                    name
                ));
            }
        }
    }

    Ok(data)
}

/// Pack character properties using schema IDs (8-byte records).
///
/// Returns an error if any property name is not in the schema.
pub fn pack_character_props_with_schema(
    character: &Character,
    schema_lookup: &HashMap<String, u16>,
    schema_names: &[&str],
    strings: &mut StringTable,
) -> Result<Vec<u8>, String> {
    if character.properties.is_empty() {
        return Ok(Vec::new());
    }

    let flat = flatten_properties(&character.properties);
    pack_flat_properties_with_schema(&flat, schema_lookup, schema_names, strings)
}

/// Pack state properties using schema IDs (8-byte records).
///
/// Returns (data, count) where count is the number of property records.
/// Returns an error if any property name is not in the schema.
pub fn pack_state_props_with_schema(
    props: &BTreeMap<String, PropertyValue>,
    schema_lookup: &HashMap<String, u16>,
    schema_names: &[&str],
    strings: &mut StringTable,
) -> Result<(Vec<u8>, u16), String> {
    if props.is_empty() {
        return Ok((Vec::new(), 0));
    }

    let flat = flatten_properties(props);
    let count = checked_u16(flat.len(), "state property count")?;
    let data = pack_flat_properties_with_schema(&flat, schema_lookup, schema_names, strings)?;
    Ok((data, count))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flatten_scalar_properties() {
        let mut props = BTreeMap::new();
        props.insert("health".to_string(), PropertyValue::Number(10000.0));
        props.insert("enabled".to_string(), PropertyValue::Bool(true));

        let flat = flatten_properties(&props);
        assert_eq!(flat.len(), 2);
        assert_eq!(flat.get("health"), Some(&PropertyValue::Number(10000.0)));
        assert_eq!(flat.get("enabled"), Some(&PropertyValue::Bool(true)));
    }

    #[test]
    fn flatten_nested_object() {
        let mut movement = BTreeMap::new();
        movement.insert("distance".to_string(), PropertyValue::Number(80.0));
        movement.insert("direction".to_string(), PropertyValue::String("forward".to_string()));

        let mut props = BTreeMap::new();
        props.insert("movement".to_string(), PropertyValue::Object(movement));

        let flat = flatten_properties(&props);
        assert_eq!(flat.len(), 2);
        assert_eq!(flat.get("movement.distance"), Some(&PropertyValue::Number(80.0)));
        assert_eq!(
            flat.get("movement.direction"),
            Some(&PropertyValue::String("forward".to_string()))
        );
    }

    #[test]
    fn flatten_array() {
        let mut props = BTreeMap::new();
        props.insert(
            "effects".to_string(),
            PropertyValue::Array(vec![
                PropertyValue::String("spark".to_string()),
                PropertyValue::Number(2.0),
            ]),
        );

        let flat = flatten_properties(&props);
        assert_eq!(flat.len(), 2);
        assert_eq!(
            flat.get("effects.0"),
            Some(&PropertyValue::String("spark".to_string()))
        );
        assert_eq!(flat.get("effects.1"), Some(&PropertyValue::Number(2.0)));
    }

    #[test]
    fn flatten_deeply_nested() {
        let mut inner = BTreeMap::new();
        inner.insert("value".to_string(), PropertyValue::Number(42.0));

        let mut outer = BTreeMap::new();
        outer.insert("inner".to_string(), PropertyValue::Object(inner));

        let mut props = BTreeMap::new();
        props.insert("outer".to_string(), PropertyValue::Object(outer));

        let flat = flatten_properties(&props);
        assert_eq!(flat.len(), 1);
        assert_eq!(flat.get("outer.inner.value"), Some(&PropertyValue::Number(42.0)));
    }

    #[test]
    fn pack_character_props_roundtrip() {
        use crate::schema::Character;

        let mut props = BTreeMap::new();
        props.insert("health".to_string(), PropertyValue::Number(10000.0));
        props.insert("archetype".to_string(), PropertyValue::String("rushdown".to_string()));

        let char = Character {
            id: "test".to_string(),
            name: "Test".to_string(),
            properties: props,
            resources: vec![],
        };

        let mut strings = StringTable::new();
        let data = pack_character_props(&char, &mut strings).unwrap();

        // 2 properties * 12 bytes = 24 bytes
        assert_eq!(data.len(), 24);
    }

    #[test]
    fn pack_state_props_with_nested() {
        let mut movement = BTreeMap::new();
        movement.insert("distance".to_string(), PropertyValue::Number(80.0));

        let mut props = BTreeMap::new();
        props.insert("startup".to_string(), PropertyValue::Number(5.0));
        props.insert("movement".to_string(), PropertyValue::Object(movement));

        let mut strings = StringTable::new();
        let (data, count) = pack_state_props(&props, &mut strings).unwrap();

        // 2 flattened properties (startup, movement.distance)
        assert_eq!(count, 2);
        assert_eq!(data.len(), 24);
    }

    // =========================================================================
    // Schema-Based Property Packing Tests
    // =========================================================================

    #[test]
    fn test_find_similar_substring_match() {
        let haystack = vec!["health", "walkSpeed", "dashSpeed", "jumpHeight"];
        let suggestions = find_similar("walk", &haystack);
        assert!(!suggestions.is_empty());
        assert!(suggestions.contains(&"walkSpeed"));
    }

    #[test]
    fn test_find_similar_levenshtein() {
        let haystack = vec!["health", "walkSpeed", "dashSpeed", "jumpHeight"];
        // "healt" is close to "health"
        let suggestions = find_similar("healt", &haystack);
        assert!(!suggestions.is_empty());
        assert!(suggestions.contains(&"health"));
    }

    #[test]
    fn test_find_similar_no_match() {
        let haystack = vec!["health", "walkSpeed"];
        let suggestions = find_similar("xyzabc", &haystack);
        assert!(suggestions.is_empty());
    }

    #[test]
    fn test_levenshtein_distance() {
        assert_eq!(levenshtein("", ""), 0);
        assert_eq!(levenshtein("abc", ""), 3);
        assert_eq!(levenshtein("", "abc"), 3);
        assert_eq!(levenshtein("abc", "abc"), 0);
        assert_eq!(levenshtein("abc", "abd"), 1);
        assert_eq!(levenshtein("kitten", "sitting"), 3);
    }

    #[test]
    fn pack_character_props_with_schema_success() {
        use std::collections::HashMap;

        let mut props = BTreeMap::new();
        props.insert("health".to_string(), PropertyValue::Number(10000.0));
        props.insert("walkSpeed".to_string(), PropertyValue::Number(3.5));

        let char = Character {
            id: "test".to_string(),
            name: "Test".to_string(),
            properties: props,
            resources: vec![],
        };

        let mut schema_lookup = HashMap::new();
        schema_lookup.insert("health".to_string(), 0u16);
        schema_lookup.insert("walkSpeed".to_string(), 1u16);
        let schema_names = vec!["health", "walkSpeed"];

        let mut strings = StringTable::new();
        let data = pack_character_props_with_schema(&char, &schema_lookup, &schema_names, &mut strings).unwrap();

        // 2 properties * 8 bytes = 16 bytes
        assert_eq!(data.len(), 16);
    }

    #[test]
    fn pack_character_props_with_schema_unknown_property() {
        use std::collections::HashMap;

        let mut props = BTreeMap::new();
        props.insert("unknownProp".to_string(), PropertyValue::Number(100.0));

        let char = Character {
            id: "test".to_string(),
            name: "Test".to_string(),
            properties: props,
            resources: vec![],
        };

        let mut schema_lookup = HashMap::new();
        schema_lookup.insert("health".to_string(), 0u16);
        let schema_names = vec!["health"];

        let mut strings = StringTable::new();
        let result = pack_character_props_with_schema(&char, &schema_lookup, &schema_names, &mut strings);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("unknownProp"));
        assert!(err.contains("not defined in the schema"));
    }

    #[test]
    fn pack_state_props_with_schema_success() {
        use std::collections::HashMap;

        let mut props = BTreeMap::new();
        props.insert("startup".to_string(), PropertyValue::Number(5.0));
        props.insert("damage".to_string(), PropertyValue::Number(500.0));

        let mut schema_lookup = HashMap::new();
        schema_lookup.insert("startup".to_string(), 0u16);
        schema_lookup.insert("damage".to_string(), 1u16);
        let schema_names = vec!["startup", "damage"];

        let mut strings = StringTable::new();
        let (data, count) = pack_state_props_with_schema(&props, &schema_lookup, &schema_names, &mut strings).unwrap();

        // 2 properties * 8 bytes = 16 bytes
        assert_eq!(count, 2);
        assert_eq!(data.len(), 16);
    }

    #[test]
    fn pack_state_props_with_schema_with_nested_flattening() {
        use std::collections::HashMap;

        let mut movement = BTreeMap::new();
        movement.insert("distance".to_string(), PropertyValue::Number(80.0));

        let mut props = BTreeMap::new();
        props.insert("startup".to_string(), PropertyValue::Number(5.0));
        props.insert("movement".to_string(), PropertyValue::Object(movement));

        // Schema must include flattened names
        let mut schema_lookup = HashMap::new();
        schema_lookup.insert("startup".to_string(), 0u16);
        schema_lookup.insert("movement.distance".to_string(), 1u16);
        let schema_names = vec!["startup", "movement.distance"];

        let mut strings = StringTable::new();
        let (data, count) = pack_state_props_with_schema(&props, &schema_lookup, &schema_names, &mut strings).unwrap();

        // 2 flattened properties * 8 bytes = 16 bytes
        assert_eq!(count, 2);
        assert_eq!(data.len(), 16);
    }

    #[test]
    fn pack_state_props_with_schema_suggestion_on_typo() {
        use std::collections::HashMap;

        let mut props = BTreeMap::new();
        props.insert("startu".to_string(), PropertyValue::Number(5.0)); // typo

        let mut schema_lookup = HashMap::new();
        schema_lookup.insert("startup".to_string(), 0u16);
        let schema_names = vec!["startup"];

        let mut strings = StringTable::new();
        let result = pack_state_props_with_schema(&props, &schema_lookup, &schema_names, &mut strings);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("startu"));
        assert!(err.contains("Did you mean"));
        assert!(err.contains("startup"));
    }
}
