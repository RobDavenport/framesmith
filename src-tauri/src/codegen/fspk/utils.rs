//! Utility functions for FSPK binary packing.

use crate::codegen::fspk_format::{write_u16_le, write_u32_le};

use super::types::StrRef;

pub fn checked_u16(value: usize, what: &str) -> Result<u16, String> {
    u16::try_from(value).map_err(|_| format!("{} overflows u16: {}", what, value))
}

pub fn checked_u32(value: usize, what: &str) -> Result<u32, String> {
    u32::try_from(value).map_err(|_| format!("{} overflows u32: {}", what, value))
}

pub fn align_up(value: usize, align: u32) -> Result<usize, String> {
    if align == 0 {
        return Err("alignment must be non-zero".to_string());
    }
    if !align.is_power_of_two() {
        return Err(format!("alignment must be power of two, got {}", align));
    }

    let align = align as usize;
    if align == 1 {
        return Ok(value);
    }

    let mask = align - 1;
    let v = value
        .checked_add(mask)
        .ok_or_else(|| "align_up overflow".to_string())?;
    Ok(v & !mask)
}

/// Write a string reference (StrRef) to the buffer.
///
/// StrRef layout: offset(u32) + length(u16) + padding(u16)
pub fn write_strref(buf: &mut Vec<u8>, strref: StrRef) {
    write_u32_le(buf, strref.0); // offset
    write_u16_le(buf, strref.1); // length
    write_u16_le(buf, 0); // padding
}

pub fn write_range(buf: &mut Vec<u8>, off: u32, len: u16) {
    write_u32_le(buf, off);
    write_u16_le(buf, len);
    write_u16_le(buf, 0);
}

pub fn write_i32_le(buf: &mut Vec<u8>, value: i32) {
    buf.extend_from_slice(&value.to_le_bytes());
}

pub fn write_i64_le(buf: &mut Vec<u8>, value: i64) {
    buf.extend_from_slice(&value.to_le_bytes());
}

pub fn write_u64_le(buf: &mut Vec<u8>, value: u64) {
    buf.extend_from_slice(&value.to_le_bytes());
}

/// Write a section header to the buffer.
///
/// Section header layout: kind(u32) + offset(u32) + length(u32) + alignment(u32)
pub fn write_section_header(buf: &mut Vec<u8>, kind: u32, offset: u32, length: u32, alignment: u32) {
    write_u32_le(buf, kind);
    write_u32_le(buf, offset);
    write_u32_le(buf, length);
    write_u32_le(buf, alignment);
}
