//! Character properties packing.

use crate::codegen::fspk_format::{
    to_q24_8, write_u16_le, write_u32_le, write_u8, PROP_TYPE_BOOL, PROP_TYPE_Q24_8, PROP_TYPE_STR,
};
use crate::schema::Character;

use super::types::StringTable;
use super::utils::checked_u16;

/// Pack character properties into the CHARACTER_PROPS section.
///
/// Each property record is 12 bytes (CHARACTER_PROP12_SIZE):
/// - bytes 0-3: name offset (u32) into string pool
/// - bytes 4-5: name length (u16)
/// - bytes 6: value type (u8) - 0=Q24.8 number, 1=bool, 2=string ref
/// - byte 7: reserved/padding
/// - bytes 8-11: value (u32/i32 depending on type)
///
/// For string values, the value field contains (offset: u16, len: u16) packed into u32.
pub fn pack_character_props(
    character: &Character,
    strings: &mut StringTable,
) -> Result<Vec<u8>, String> {
    use crate::schema::PropertyValue;

    // CHARACTER_PROP12_SIZE = 12 bytes per property
    let mut data = Vec::with_capacity(character.properties.len() * 12);

    // BTreeMap iterates in sorted key order, ensuring deterministic output
    for (name, value) in &character.properties {
        // Write name reference (offset + length)
        let (name_off, name_len) = strings.intern(name)?;
        write_u32_le(&mut data, name_off);
        write_u16_le(&mut data, name_len);

        // Write type and value based on PropertyValue variant
        match value {
            PropertyValue::Number(n) => {
                write_u8(&mut data, PROP_TYPE_Q24_8);
                write_u8(&mut data, 0); // reserved
                // Convert f64 to Q24.8 fixed-point and write as i32
                let q24_8 = to_q24_8(*n);
                data.extend_from_slice(&q24_8.to_le_bytes());
            }
            PropertyValue::Bool(b) => {
                write_u8(&mut data, PROP_TYPE_BOOL);
                write_u8(&mut data, 0); // reserved
                // Write boolean as u32 (0 or 1)
                let val: u32 = if *b { 1 } else { 0 };
                write_u32_le(&mut data, val);
            }
            PropertyValue::String(s) => {
                write_u8(&mut data, PROP_TYPE_STR);
                write_u8(&mut data, 0); // reserved
                // Write string reference as u16 offset + u16 length pair
                let (str_off, str_len) = strings.intern(s)?;
                let str_off_u16 = checked_u16(str_off as usize, "string property value offset")?;
                write_u16_le(&mut data, str_off_u16);
                write_u16_le(&mut data, str_len);
            }
        }
    }

    Ok(data)
}
