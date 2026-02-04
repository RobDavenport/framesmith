//! Property packing for CHARACTER_PROPS and STATE_PROPS sections.
//!
//! Both sections use the same fixed 12-byte record format for zero-copy reading.
//! Nested Object properties are flattened using dot notation (e.g., `movement.distance`).
//! Arrays are flattened with index notation (e.g., `effects.0`, `effects.1`).

use std::collections::BTreeMap;

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
}
