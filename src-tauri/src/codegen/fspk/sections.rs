//! Section building helpers for FSPK export.

use crate::codegen::fspk_format::{write_u16_le, write_u32_le, write_u8};
use crate::schema::{EventArgValue, EventEmit};

use super::types::StringTable;
use super::utils::{checked_u16, checked_u32, write_i64_le, write_range, write_strref, write_u64_le};

// Event argument type tags
pub const EVENT_ARG_TAG_BOOL: u8 = 0;
pub const EVENT_ARG_TAG_I64: u8 = 1;
pub const EVENT_ARG_TAG_F32: u8 = 2;
pub const EVENT_ARG_TAG_STRING: u8 = 3;

// Resource delta trigger types
pub const RESOURCE_DELTA_TRIGGER_ON_USE: u8 = 0;
pub const RESOURCE_DELTA_TRIGGER_ON_HIT: u8 = 1;
pub const RESOURCE_DELTA_TRIGGER_ON_BLOCK: u8 = 2;

/// Sentinel value for optional u16 fields
pub const OPT_U16_NONE: u16 = u16::MAX;

/// Pack event arguments into the event_args buffer.
///
/// Returns (args_off, args_len) for the packed arguments.
pub fn pack_event_args(
    args: &std::collections::BTreeMap<String, EventArgValue>,
    event_args_data: &mut Vec<u8>,
    strings: &mut StringTable,
) -> Result<(u32, u16), String> {
    let args_off = checked_u32(event_args_data.len(), "event_args_off")?;
    let args_len = checked_u16(args.len(), "event_args_len")?;

    for (k, v) in args {
        let key = strings.intern(k)?;
        write_strref(event_args_data, key);

        match v {
            EventArgValue::Bool(b) => {
                write_u8(event_args_data, EVENT_ARG_TAG_BOOL);
                write_u8(event_args_data, 0);
                write_u16_le(event_args_data, 0);
                write_u64_le(event_args_data, if *b { 1 } else { 0 });
            }
            EventArgValue::I64(i) => {
                write_u8(event_args_data, EVENT_ARG_TAG_I64);
                write_u8(event_args_data, 0);
                write_u16_le(event_args_data, 0);
                write_i64_le(event_args_data, *i);
            }
            EventArgValue::F32(f) => {
                write_u8(event_args_data, EVENT_ARG_TAG_F32);
                write_u8(event_args_data, 0);
                write_u16_le(event_args_data, 0);
                write_u64_le(event_args_data, f.to_bits() as u64);
            }
            EventArgValue::String(s) => {
                write_u8(event_args_data, EVENT_ARG_TAG_STRING);
                write_u8(event_args_data, 0);
                write_u16_le(event_args_data, 0);
                let vref = strings.intern(s)?;
                write_u32_le(event_args_data, vref.0);
                write_u16_le(event_args_data, vref.1);
                write_u16_le(event_args_data, 0);
            }
        }
    }

    Ok((args_off, args_len))
}

/// Pack event emits into the event_emits buffer.
///
/// Returns (emits_off, emits_len) for the packed emits.
pub fn pack_event_emits(
    events: &[EventEmit],
    event_emits_data: &mut Vec<u8>,
    event_args_data: &mut Vec<u8>,
    strings: &mut StringTable,
) -> Result<(u32, u16), String> {
    let emits_off = checked_u32(event_emits_data.len(), "event_emits_off")?;
    let emits_len = checked_u16(events.len(), "event_emits_len")?;

    for emit in events {
        let (args_off, args_len) = pack_event_args(&emit.args, event_args_data, strings)?;

        let id = strings.intern(&emit.id)?;
        write_strref(event_emits_data, id);
        write_range(event_emits_data, args_off, args_len);
    }

    Ok((emits_off, emits_len))
}

/// Pack resource definitions into the RESOURCE_DEFS section.
///
/// Returns the packed binary data.
pub fn pack_resource_defs(
    resources: &[crate::schema::CharacterResource],
    strings: &mut StringTable,
) -> Result<Vec<u8>, String> {
    let mut data = Vec::new();
    for res in resources {
        let name = strings.intern(&res.name)?;
        write_strref(&mut data, name);
        write_u16_le(&mut data, res.start);
        write_u16_le(&mut data, res.max);
    }
    Ok(data)
}
