//! ZX FSPK (Framesmith Pack) Binary Format Constants
//!
//! This module defines the binary format for exporting character data to the ZX runtime.
//! All multi-byte integers are little-endian.
//!
//! # Container Header (16 bytes)
//!
//! | Offset | Size | Field          | Description                        |
//! |--------|------|----------------|------------------------------------|
//! | 0      | 4    | magic          | "FSPK" (0x4B505346 little-endian)  |
//! | 4      | 4    | flags          | Reserved, must be 0                |
//! | 8      | 4    | total_len      | Total file size in bytes           |
//! | 12     | 4    | section_count  | Number of sections                 |
//!
//! Immediately following the header are `section_count` section headers (16 bytes each).
//!
//! # Section Header (16 bytes)
//!
//! | Offset | Size | Field | Description                          |
//! |--------|------|-------|--------------------------------------|
//! | 0      | 4    | kind  | Section type (see SECTION_* consts) |
//! | 4      | 4    | off   | Offset from file start               |
//! | 8      | 4    | len   | Section length in bytes              |
//! | 12     | 4    | align | Required alignment (power of 2)      |
//!
//! # Numeric Encoding
//!
//! - Most coordinate/dimension values use Q12.4 fixed point (1/16 pixel precision)
//! - Angles use Q8.8 fixed point (degrees)
//! - String references are (offset: u32, length: u16) pairs into STRING_TABLE

// =============================================================================
// Magic and Header Fields
// =============================================================================

/// Magic bytes "FSPK" as a 4-byte array
pub const MAGIC: [u8; 4] = [b'F', b'S', b'P', b'K'];

/// Reserved flags value (must be 0 for v1)
pub const FLAGS_RESERVED: u32 = 0;

// =============================================================================
// Section Kinds (stable numeric IDs - do not reuse deleted values)
// =============================================================================

/// Raw UTF-8 string data, referenced by (off, len) pairs
pub const SECTION_STRING_TABLE: u32 = 1;

/// Array of StrRef pointing to mesh asset keys
pub const SECTION_MESH_KEYS: u32 = 2;

/// Array of StrRef pointing to keyframes asset keys
pub const SECTION_KEYFRAMES_KEYS: u32 = 3;

/// Array of StateRecord structs
pub const SECTION_STATES: u32 = 4;

/// Array of HitWindow24 structs
pub const SECTION_HIT_WINDOWS: u32 = 5;

/// Array of HurtWindow12 structs
pub const SECTION_HURT_WINDOWS: u32 = 6;

/// Array of Shape12 structs (hitboxes/hurtboxes geometry)
pub const SECTION_SHAPES: u32 = 7;

/// Array of u16 move IDs for cancel targets
pub const SECTION_CANCELS_U16: u32 = 8;

/// Array of ResourceDef12 structs
pub const SECTION_RESOURCE_DEFS: u32 = 9;

/// Array of StateExtras56 structs (parallel to STATES)
pub const SECTION_STATE_EXTRAS: u32 = 10;

/// Array of EventEmit16 structs
pub const SECTION_EVENT_EMITS: u32 = 11;

/// Array of EventArg20 structs
pub const SECTION_EVENT_ARGS: u32 = 12;

/// Array of MoveNotify12 structs
pub const SECTION_MOVE_NOTIFIES: u32 = 13;

/// Array of MoveResourceCost12 structs
pub const SECTION_MOVE_RESOURCE_COSTS: u32 = 14;

/// Array of MoveResourcePrecondition12 structs
pub const SECTION_MOVE_RESOURCE_PRECONDITIONS: u32 = 15;

/// Array of MoveResourceDelta16 structs
pub const SECTION_MOVE_RESOURCE_DELTAS: u32 = 16;

/// Array of StateTagRange8 structs (one per move): offset(4) + count(2) + padding(2)
pub const SECTION_STATE_TAG_RANGES: u32 = 17;

/// Array of StrRef for state tags (indexed by ranges)
pub const SECTION_STATE_TAGS: u32 = 18;

/// Array of CancelTagRule24 structs (tag-based cancel rules)
pub const SECTION_CANCEL_TAG_RULES: u32 = 19;

/// Array of CancelDeny4 structs (explicit deny pairs as from_idx:u16, to_idx:u16)
pub const SECTION_CANCEL_DENIES: u32 = 20;

// =============================================================================
// Sentinel Values
// =============================================================================

/// Sentinel value for "no mesh" or "no keyframes" (u16::MAX)
pub const KEY_NONE: u16 = 0xFFFF;

/// Sentinel value for invalid/unset u32 offsets
pub const OFFSET_NONE: u32 = 0xFFFF_FFFF;

// =============================================================================
// Cancel Flag Constants (MoveRecord.flags byte)
// =============================================================================

/// Move has chain cancel routes in CANCELS_U16
pub const CANCEL_FLAG_CHAIN: u8 = 0x01;

/// Move can cancel into special moves
pub const CANCEL_FLAG_SPECIAL: u8 = 0x02;

/// Move can cancel into super moves
pub const CANCEL_FLAG_SUPER: u8 = 0x04;

/// Move can cancel into jump
pub const CANCEL_FLAG_JUMP: u8 = 0x08;

// =============================================================================
// Shape Kind Constants
// =============================================================================

/// Axis-aligned bounding box: x=a, y=b, w=u16(c), h=u16(d)
pub const SHAPE_KIND_AABB: u8 = 0;

/// Rotated rectangle: x=a, y=b, w=u16(c), h=u16(d), angle=e (Q8.8 degrees)
pub const SHAPE_KIND_RECT: u8 = 1;

/// Circle: x=a, y=b, r=u16(c)
pub const SHAPE_KIND_CIRCLE: u8 = 2;

/// Capsule: x1=a, y1=b, x2=c, y2=d, r=u16(e)
pub const SHAPE_KIND_CAPSULE: u8 = 3;

// =============================================================================
// Structure Sizes (bytes)
// =============================================================================

/// Container header size: magic(4) + flags(4) + total_len(4) + section_count(4)
pub const HEADER_SIZE: usize = 16;

/// Section header size: kind(4) + off(4) + len(4) + align(4)
pub const SECTION_HEADER_SIZE: usize = 16;

/// String reference size: off(4) + len(2) + pad(2)
pub const STRREF_SIZE: usize = 8;

/// Shape encoding size: kind(1) + flags(1) + a(2) + b(2) + c(2) + d(2) + e(2)
pub const SHAPE12_SIZE: usize = 12;

/// Hit window size (see HitWindow24 struct in module docs)
pub const HIT_WINDOW24_SIZE: usize = 24;

/// Hurt window size (see HurtWindow12 struct in module docs)
pub const HURT_WINDOW12_SIZE: usize = 12;

/// State record size (see StateRecord struct in module docs)
pub const STATE_RECORD_SIZE: usize = 32;

/// ResourceDef record size
pub const RESOURCE_DEF12_SIZE: usize = 12;

/// StateExtras record size (expanded from 64 to 72 for cancel offset/length)
pub const STATE_EXTRAS72_SIZE: usize = 72;

/// EventEmit record size
pub const EVENT_EMIT16_SIZE: usize = 16;

/// EventArg record size
pub const EVENT_ARG20_SIZE: usize = 20;

/// MoveNotify record size
pub const MOVE_NOTIFY12_SIZE: usize = 12;

/// MoveResourceCost record size
pub const MOVE_RESOURCE_COST12_SIZE: usize = 12;

/// MoveResourcePrecondition record size
pub const MOVE_RESOURCE_PRECONDITION12_SIZE: usize = 12;

/// MoveResourceDelta record size
pub const MOVE_RESOURCE_DELTA16_SIZE: usize = 16;

// =============================================================================
// Fixed Point Conversion Helpers
// =============================================================================

/// Convert a floating-point value to Q12.4 fixed point (1/16 pixel precision)
#[inline]
pub fn to_q12_4(value: f32) -> i16 {
    (value * 16.0).round() as i16
}

/// Convert a floating-point value to unsigned Q12.4 fixed point
#[inline]
pub fn to_q12_4_unsigned(value: f32) -> u16 {
    (value * 16.0).round() as u16
}

/// Convert a floating-point angle (degrees) to Q8.8 fixed point
#[inline]
pub fn to_q8_8_degrees(degrees: f32) -> i16 {
    (degrees * 256.0).round() as i16
}

// =============================================================================
// Little-Endian Write Helpers
// =============================================================================

/// Write a u16 in little-endian format to the buffer
#[inline]
pub fn write_u16_le(buf: &mut Vec<u8>, value: u16) {
    buf.extend_from_slice(&value.to_le_bytes());
}

/// Write a u32 in little-endian format to the buffer
#[inline]
pub fn write_u32_le(buf: &mut Vec<u8>, value: u32) {
    buf.extend_from_slice(&value.to_le_bytes());
}

/// Write an i16 in little-endian format to the buffer
#[inline]
pub fn write_i16_le(buf: &mut Vec<u8>, value: i16) {
    buf.extend_from_slice(&value.to_le_bytes());
}

/// Write a u8 to the buffer
#[inline]
pub fn write_u8(buf: &mut Vec<u8>, value: u8) {
    buf.push(value);
}

/// Write padding bytes (zeros) to align to the specified boundary
///
/// Returns the number of padding bytes written.
#[inline]
pub fn write_padding(buf: &mut Vec<u8>, alignment: usize) -> usize {
    let current = buf.len();
    let aligned = (current + alignment - 1) & !(alignment - 1);
    let padding = aligned - current;
    buf.resize(aligned, 0);
    padding
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_magic_bytes() {
        assert_eq!(&MAGIC, b"FSPK");
    }

    #[test]
    fn test_flags_reserved_is_zero() {
        assert_eq!(FLAGS_RESERVED as u32, 0);
    }

    #[test]
    fn test_section_kinds_are_unique() {
        let kinds = [
            SECTION_STRING_TABLE,
            SECTION_MESH_KEYS,
            SECTION_KEYFRAMES_KEYS,
            SECTION_STATES,
            SECTION_HIT_WINDOWS,
            SECTION_HURT_WINDOWS,
            SECTION_SHAPES,
            SECTION_CANCELS_U16,
            SECTION_RESOURCE_DEFS,
            SECTION_STATE_EXTRAS,
            SECTION_EVENT_EMITS,
            SECTION_EVENT_ARGS,
            SECTION_MOVE_NOTIFIES,
            SECTION_MOVE_RESOURCE_COSTS,
            SECTION_MOVE_RESOURCE_PRECONDITIONS,
            SECTION_MOVE_RESOURCE_DELTAS,
            SECTION_STATE_TAG_RANGES,
            SECTION_STATE_TAGS,
            SECTION_CANCEL_TAG_RULES,
            SECTION_CANCEL_DENIES,
        ];
        let mut sorted = kinds;
        sorted.sort();
        for i in 1..sorted.len() {
            assert_ne!(sorted[i - 1], sorted[i], "Section kinds must be unique");
        }
    }

    #[test]
    fn test_structure_sizes() {
        // These sizes are part of the binary format contract and must not change
        assert_eq!(HEADER_SIZE, 16, "Header size must be 16 bytes");
        assert_eq!(
            SECTION_HEADER_SIZE, 16,
            "Section header size must be 16 bytes"
        );
        assert_eq!(STRREF_SIZE, 8, "StrRef size must be 8 bytes");
        assert_eq!(SHAPE12_SIZE, 12, "Shape12 size must be 12 bytes");
        assert_eq!(HIT_WINDOW24_SIZE, 24, "HitWindow24 size must be 24 bytes");
        assert_eq!(HURT_WINDOW12_SIZE, 12, "HurtWindow12 size must be 12 bytes");
        assert_eq!(STATE_RECORD_SIZE, 32, "StateRecord size must be 32 bytes");
    }

    #[test]
    fn test_write_u16_le() {
        let mut buf = Vec::new();
        write_u16_le(&mut buf, 0x1234);
        assert_eq!(buf, vec![0x34, 0x12]);
    }

    #[test]
    fn test_write_u32_le() {
        let mut buf = Vec::new();
        write_u32_le(&mut buf, 0x12345678);
        assert_eq!(buf, vec![0x78, 0x56, 0x34, 0x12]);
    }

    #[test]
    fn test_write_i16_le() {
        let mut buf = Vec::new();
        write_i16_le(&mut buf, -1);
        assert_eq!(buf, vec![0xFF, 0xFF]);

        let mut buf2 = Vec::new();
        write_i16_le(&mut buf2, 0x1234);
        assert_eq!(buf2, vec![0x34, 0x12]);
    }

    #[test]
    fn test_write_padding() {
        let mut buf = vec![0u8; 5];
        let padding = write_padding(&mut buf, 4);
        assert_eq!(buf.len(), 8);
        assert_eq!(padding, 3);

        let mut buf2 = vec![0u8; 8];
        let padding2 = write_padding(&mut buf2, 4);
        assert_eq!(buf2.len(), 8);
        assert_eq!(padding2, 0);
    }

    #[test]
    fn test_q12_4_conversion() {
        // 1.0 -> 16
        assert_eq!(to_q12_4(1.0), 16);
        // 0.5 -> 8
        assert_eq!(to_q12_4(0.5), 8);
        // -1.0 -> -16
        assert_eq!(to_q12_4(-1.0), -16);
        // 0.0625 (1/16) -> 1
        assert_eq!(to_q12_4(0.0625), 1);
    }

    #[test]
    fn test_q8_8_degrees_conversion() {
        // 1.0 degree -> 256
        assert_eq!(to_q8_8_degrees(1.0), 256);
        // 90.0 degrees -> 23040
        assert_eq!(to_q8_8_degrees(90.0), 23040);
        // -45.0 degrees -> -11520
        assert_eq!(to_q8_8_degrees(-45.0), -11520);
    }

    #[test]
    fn test_sentinels() {
        assert_eq!(KEY_NONE, u16::MAX);
        assert_eq!(OFFSET_NONE, u32::MAX);
    }
}
