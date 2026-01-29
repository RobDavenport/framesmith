//! Zero-copy view into an FSPK pack.

use crate::bytes::{read_u8, read_u16_le, read_u32_le};
use crate::error::Error;

/// Magic bytes identifying an FSPK file.
pub const MAGIC: [u8; 4] = [b'F', b'S', b'P', b'K'];

/// Current supported version.
pub const VERSION: u16 = 1;

/// Size of the main header in bytes.
/// Layout: magic(4) + version(2) + flags(2) + total_len(4) + section_count(4)
pub const HEADER_SIZE: usize = 16;

/// Size of each section header in bytes.
/// Layout: kind(4) + offset(4) + len(4) + align(4)
pub const SECTION_HEADER_SIZE: usize = 16;

/// Maximum number of sections supported.
pub const MAX_SECTIONS: usize = 16;

// =============================================================================
// Section Kind Constants
// =============================================================================

/// Raw UTF-8 string data, referenced by (off, len) pairs
pub const SECTION_STRING_TABLE: u32 = 1;

/// Array of StrRef pointing to mesh asset keys
pub const SECTION_MESH_KEYS: u32 = 2;

/// Array of StrRef pointing to keyframes asset keys
pub const SECTION_KEYFRAMES_KEYS: u32 = 3;

/// Array of MoveRecord structs
pub const SECTION_MOVES: u32 = 4;

/// Array of HitWindow24 structs
pub const SECTION_HIT_WINDOWS: u32 = 5;

/// Array of HurtWindow12 structs
pub const SECTION_HURT_WINDOWS: u32 = 6;

/// Array of Shape12 structs (hitboxes/hurtboxes geometry)
pub const SECTION_SHAPES: u32 = 7;

/// Array of u16 move IDs for cancel targets
pub const SECTION_CANCELS_U16: u32 = 8;

// =============================================================================
// Structure Sizes
// =============================================================================

/// String reference size: off(4) + len(2) + pad(2)
pub const STRREF_SIZE: usize = 8;

/// Move record size (see MoveRecord in module docs)
pub const MOVE_RECORD_SIZE: usize = 32;

// =============================================================================
// Sentinel Values
// =============================================================================

/// Sentinel value for "no mesh" or "no keyframes" (u16::MAX)
pub const KEY_NONE: u16 = 0xFFFF;

/// Information about a single section in the pack.
#[derive(Debug, Clone, Copy, Default)]
struct SectionInfo {
    kind: u32,
    offset: u32,
    len: u32,
    #[allow(dead_code)]
    align: u32,
}

/// A zero-copy view into an FSPK binary pack.
///
/// This struct provides read-only access to the pack's contents without
/// allocating memory. All data is read directly from the underlying byte slice.
pub struct PackView<'a> {
    data: &'a [u8],
    sections: [SectionInfo; MAX_SECTIONS],
    section_count: usize,
}

impl<'a> PackView<'a> {
    /// Parse the given bytes as an FSPK pack.
    ///
    /// Returns a `PackView` that provides zero-copy access to the pack contents.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The data is too short to contain a valid header (`TooShort`)
    /// - The magic bytes are incorrect (`InvalidMagic`)
    /// - The version is not supported (`UnsupportedVersion`)
    /// - Section headers or data are out of bounds (`OutOfBounds`)
    pub fn parse(bytes: &'a [u8]) -> Result<Self, Error> {
        // Check minimum length for header
        if bytes.len() < HEADER_SIZE {
            return Err(Error::TooShort);
        }

        // Validate magic bytes
        if bytes[0..4] != MAGIC {
            return Err(Error::InvalidMagic);
        }

        // Check version
        let version = read_u16_le(bytes, 4).ok_or(Error::TooShort)?;
        if version != VERSION {
            return Err(Error::UnsupportedVersion);
        }

        // Read header fields
        // flags at offset 6 (2 bytes) - currently unused
        let total_len = read_u32_le(bytes, 8).ok_or(Error::TooShort)? as usize;
        let section_count = read_u32_le(bytes, 12).ok_or(Error::TooShort)? as usize;

        // Validate total_len matches actual data length
        if total_len > bytes.len() {
            return Err(Error::OutOfBounds);
        }

        // Validate section count
        if section_count > MAX_SECTIONS {
            return Err(Error::OutOfBounds);
        }

        // Calculate where section headers end
        let section_table_size = section_count
            .checked_mul(SECTION_HEADER_SIZE)
            .ok_or(Error::OutOfBounds)?;
        let section_table_end = HEADER_SIZE
            .checked_add(section_table_size)
            .ok_or(Error::OutOfBounds)?;

        // Validate section table fits within data
        if section_table_end > bytes.len() {
            return Err(Error::OutOfBounds);
        }

        // Parse section headers
        let mut sections = [SectionInfo::default(); MAX_SECTIONS];
        for i in 0..section_count {
            let header_offset = HEADER_SIZE + i * SECTION_HEADER_SIZE;

            let kind = read_u32_le(bytes, header_offset).ok_or(Error::OutOfBounds)?;
            let offset = read_u32_le(bytes, header_offset + 4).ok_or(Error::OutOfBounds)?;
            let len = read_u32_le(bytes, header_offset + 8).ok_or(Error::OutOfBounds)?;
            let align = read_u32_le(bytes, header_offset + 12).ok_or(Error::OutOfBounds)?;

            // Validate section data fits within total_len
            let section_end = (offset as usize)
                .checked_add(len as usize)
                .ok_or(Error::OutOfBounds)?;
            if section_end > total_len {
                return Err(Error::OutOfBounds);
            }

            sections[i] = SectionInfo {
                kind,
                offset,
                len,
                align,
            };
        }

        Ok(Self {
            data: bytes,
            sections,
            section_count,
        })
    }

    /// Get the data for a section with the given kind.
    ///
    /// Returns `None` if no section with that kind exists.
    pub fn get_section(&self, kind: u32) -> Option<&'a [u8]> {
        for i in 0..self.section_count {
            if self.sections[i].kind == kind {
                let offset = self.sections[i].offset as usize;
                let len = self.sections[i].len as usize;
                return Some(&self.data[offset..offset + len]);
            }
        }
        None
    }

    /// Returns the number of sections in the pack.
    pub fn section_count(&self) -> usize {
        self.section_count
    }

    /// Get a string from the string table by offset and length.
    ///
    /// Returns `None` if:
    /// - No string table section exists
    /// - The offset/length is out of bounds
    /// - The bytes are not valid UTF-8
    pub fn string(&self, off: u32, len: u16) -> Option<&'a str> {
        let table = self.get_section(SECTION_STRING_TABLE)?;
        let start = off as usize;
        let end = start.checked_add(len as usize)?;
        if end > table.len() {
            return None;
        }
        core::str::from_utf8(&table[start..end]).ok()
    }

    /// Get mesh keys section as a typed view.
    ///
    /// Returns `None` if no mesh keys section exists.
    pub fn mesh_keys(&self) -> Option<MeshKeysView<'a>> {
        let data = self.get_section(SECTION_MESH_KEYS)?;
        Some(MeshKeysView { data })
    }

    /// Get keyframes keys section as a typed view.
    ///
    /// Returns `None` if no keyframes keys section exists.
    pub fn keyframes_keys(&self) -> Option<KeyframesKeysView<'a>> {
        let data = self.get_section(SECTION_KEYFRAMES_KEYS)?;
        Some(KeyframesKeysView { data })
    }

    /// Get moves section as a typed view.
    ///
    /// Returns `None` if no moves section exists.
    pub fn moves(&self) -> Option<MovesView<'a>> {
        let data = self.get_section(SECTION_MOVES)?;
        Some(MovesView { data })
    }
}

// =============================================================================
// Typed Views
// =============================================================================

/// Zero-copy view over the mesh keys section.
///
/// Each entry is a StrRef (8 bytes): off(4) + len(2) + pad(2)
#[derive(Clone, Copy)]
pub struct MeshKeysView<'a> {
    data: &'a [u8],
}

impl<'a> MeshKeysView<'a> {
    /// Returns the number of mesh keys in this section.
    pub fn len(&self) -> usize {
        self.data.len() / STRREF_SIZE
    }

    /// Returns true if there are no mesh keys.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get the offset and length for a mesh key at the given index.
    ///
    /// Returns `None` if the index is out of bounds.
    pub fn get(&self, index: usize) -> Option<(u32, u16)> {
        let base = index.checked_mul(STRREF_SIZE)?;
        if base + STRREF_SIZE > self.data.len() {
            return None;
        }
        let off = read_u32_le(self.data, base)?;
        let len = read_u16_le(self.data, base + 4)?;
        Some((off, len))
    }
}

/// Zero-copy view over the keyframes keys section.
///
/// Each entry is a StrRef (8 bytes): off(4) + len(2) + pad(2)
#[derive(Clone, Copy)]
pub struct KeyframesKeysView<'a> {
    data: &'a [u8],
}

impl<'a> KeyframesKeysView<'a> {
    /// Returns the number of keyframes keys in this section.
    pub fn len(&self) -> usize {
        self.data.len() / STRREF_SIZE
    }

    /// Returns true if there are no keyframes keys.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get the offset and length for a keyframes key at the given index.
    ///
    /// Returns `None` if the index is out of bounds.
    pub fn get(&self, index: usize) -> Option<(u32, u16)> {
        let base = index.checked_mul(STRREF_SIZE)?;
        if base + STRREF_SIZE > self.data.len() {
            return None;
        }
        let off = read_u32_le(self.data, base)?;
        let len = read_u16_le(self.data, base + 4)?;
        Some((off, len))
    }
}

/// Zero-copy view over the moves section.
///
/// Each entry is a MoveRecord (32 bytes).
#[derive(Clone, Copy)]
pub struct MovesView<'a> {
    data: &'a [u8],
}

impl<'a> MovesView<'a> {
    /// Returns the number of moves in this section.
    pub fn len(&self) -> usize {
        self.data.len() / MOVE_RECORD_SIZE
    }

    /// Returns true if there are no moves.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get a view of the move record at the given index.
    ///
    /// Returns `None` if the index is out of bounds.
    pub fn get(&self, index: usize) -> Option<MoveView<'a>> {
        let base = index.checked_mul(MOVE_RECORD_SIZE)?;
        let end = base.checked_add(MOVE_RECORD_SIZE)?;
        if end > self.data.len() {
            return None;
        }
        Some(MoveView {
            data: &self.data[base..end],
        })
    }
}

/// Zero-copy view over a single move record (32 bytes).
///
/// Layout:
/// - 0-1: move_id (u16)
/// - 2-3: mesh_key (u16)
/// - 4-5: keyframes_key (u16)
/// - 6: move_type (u8)
/// - 7: trigger (u8)
/// - 8: guard (u8)
/// - 9: flags (u8)
/// - 10: startup (u8)
/// - 11: active (u8)
/// - 12: recovery (u8)
/// - 13: reserved (u8)
/// - 14-15: total (u16)
/// - 16-17: damage (u16)
/// - 18: hitstun (u8)
/// - 19: blockstun (u8)
/// - 20: hitstop (u8)
/// - 21: reserved (u8)
/// - 22-25: hit_windows_off (u32)
/// - 26-27: hit_windows_len (u16)
/// - 28-29: hurt_windows_off (u16) - note: compressed to fit 32 bytes
/// - 30-31: hurt_windows_len (u16)
#[derive(Clone, Copy)]
pub struct MoveView<'a> {
    data: &'a [u8],
}

impl<'a> MoveView<'a> {
    /// Returns the move ID (index in the moves array).
    pub fn move_id(&self) -> u16 {
        read_u16_le(self.data, 0).unwrap_or(0)
    }

    /// Returns the mesh key index, or KEY_NONE (0xFFFF) if no mesh.
    pub fn mesh_key(&self) -> u16 {
        read_u16_le(self.data, 2).unwrap_or(KEY_NONE)
    }

    /// Returns the keyframes key index, or KEY_NONE (0xFFFF) if no keyframes.
    pub fn keyframes_key(&self) -> u16 {
        read_u16_le(self.data, 4).unwrap_or(KEY_NONE)
    }

    /// Returns the move type.
    pub fn move_type(&self) -> u8 {
        read_u8(self.data, 6).unwrap_or(0)
    }

    /// Returns the trigger type.
    pub fn trigger(&self) -> u8 {
        read_u8(self.data, 7).unwrap_or(0)
    }

    /// Returns the guard type.
    pub fn guard(&self) -> u8 {
        read_u8(self.data, 8).unwrap_or(0)
    }

    /// Returns the move flags.
    pub fn flags(&self) -> u8 {
        read_u8(self.data, 9).unwrap_or(0)
    }

    /// Returns the startup frames.
    pub fn startup(&self) -> u8 {
        read_u8(self.data, 10).unwrap_or(0)
    }

    /// Returns the active frames.
    pub fn active(&self) -> u8 {
        read_u8(self.data, 11).unwrap_or(0)
    }

    /// Returns the recovery frames.
    pub fn recovery(&self) -> u8 {
        read_u8(self.data, 12).unwrap_or(0)
    }

    /// Returns the total frame count.
    pub fn total(&self) -> u16 {
        read_u16_le(self.data, 14).unwrap_or(0)
    }

    /// Returns the damage value.
    pub fn damage(&self) -> u16 {
        read_u16_le(self.data, 16).unwrap_or(0)
    }

    /// Returns the hitstun frames.
    pub fn hitstun(&self) -> u8 {
        read_u8(self.data, 18).unwrap_or(0)
    }

    /// Returns the blockstun frames.
    pub fn blockstun(&self) -> u8 {
        read_u8(self.data, 19).unwrap_or(0)
    }

    /// Returns the hitstop frames.
    pub fn hitstop(&self) -> u8 {
        read_u8(self.data, 20).unwrap_or(0)
    }

    /// Returns the hit windows offset into the HIT_WINDOWS section.
    pub fn hit_windows_off(&self) -> u32 {
        read_u32_le(self.data, 22).unwrap_or(0)
    }

    /// Returns the hit windows count.
    pub fn hit_windows_len(&self) -> u16 {
        read_u16_le(self.data, 26).unwrap_or(0)
    }

    /// Returns the hurt windows offset (as u16 for compact layout).
    pub fn hurt_windows_off(&self) -> u16 {
        read_u16_le(self.data, 28).unwrap_or(0)
    }

    /// Returns the hurt windows count.
    pub fn hurt_windows_len(&self) -> u16 {
        read_u16_le(self.data, 30).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to build a valid FSPK header.
    fn build_header(version: u16, flags: u16, total_len: u32, section_count: u32) -> [u8; 16] {
        let mut header = [0u8; 16];
        header[0..4].copy_from_slice(&MAGIC);
        header[4..6].copy_from_slice(&version.to_le_bytes());
        header[6..8].copy_from_slice(&flags.to_le_bytes());
        header[8..12].copy_from_slice(&total_len.to_le_bytes());
        header[12..16].copy_from_slice(&section_count.to_le_bytes());
        header
    }

    /// Helper to build a section header.
    fn build_section_header(kind: u32, offset: u32, len: u32, align: u32) -> [u8; 16] {
        let mut section = [0u8; 16];
        section[0..4].copy_from_slice(&kind.to_le_bytes());
        section[4..8].copy_from_slice(&offset.to_le_bytes());
        section[8..12].copy_from_slice(&len.to_le_bytes());
        section[12..16].copy_from_slice(&align.to_le_bytes());
        section
    }

    #[test]
    fn parse_too_short() {
        let result = PackView::parse(&[0u8; 15]);
        assert!(matches!(result, Err(Error::TooShort)));
    }

    #[test]
    fn parse_wrong_magic() {
        let mut data = [0u8; 16];
        data[0..4].copy_from_slice(b"XXXX");
        data[4..6].copy_from_slice(&1u16.to_le_bytes());
        data[8..12].copy_from_slice(&16u32.to_le_bytes());
        data[12..16].copy_from_slice(&0u32.to_le_bytes());

        let result = PackView::parse(&data);
        assert!(matches!(result, Err(Error::InvalidMagic)));
    }

    #[test]
    fn parse_unsupported_version() {
        let header = build_header(99, 0, 16, 0);
        let result = PackView::parse(&header);
        assert!(matches!(result, Err(Error::UnsupportedVersion)));
    }

    #[test]
    fn parse_section_table_out_of_bounds() {
        // Header claims 2 sections but data only has room for header
        let header = build_header(1, 0, 16, 2);
        let result = PackView::parse(&header);
        assert!(matches!(result, Err(Error::OutOfBounds)));
    }

    #[test]
    fn parse_section_data_out_of_bounds() {
        // Header + 1 section header
        let mut data = std::vec::Vec::new();
        data.extend_from_slice(&build_header(1, 0, 32, 1));
        // Section at offset 32 with len 100 - way beyond total_len
        data.extend_from_slice(&build_section_header(1, 32, 100, 1));

        let result = PackView::parse(&data);
        assert!(matches!(result, Err(Error::OutOfBounds)));
    }

    #[test]
    fn parse_section_offset_overflow() {
        // Header + 1 section header
        let mut data = std::vec::Vec::new();
        data.extend_from_slice(&build_header(1, 0, 48, 1));
        // Section with offset + len that would overflow
        data.extend_from_slice(&build_section_header(1, u32::MAX, 1, 1));
        // Add some padding to reach total_len
        data.resize(48, 0);

        let result = PackView::parse(&data);
        assert!(matches!(result, Err(Error::OutOfBounds)));
    }

    #[test]
    fn parse_total_len_exceeds_data() {
        // Header claims total_len is 1000 but we only have 16 bytes
        let header = build_header(1, 0, 1000, 0);
        let result = PackView::parse(&header);
        assert!(matches!(result, Err(Error::OutOfBounds)));
    }

    #[test]
    fn parse_too_many_sections() {
        // Claim more than MAX_SECTIONS
        let header = build_header(1, 0, 16, 100);
        let result = PackView::parse(&header);
        assert!(matches!(result, Err(Error::OutOfBounds)));
    }

    #[test]
    fn parse_valid_empty_pack() {
        let header = build_header(1, 0, 16, 0);
        let view = PackView::parse(&header).expect("should parse valid empty pack");
        assert_eq!(view.section_count(), 0);
    }

    #[test]
    fn parse_valid_with_one_section() {
        let mut data = std::vec::Vec::new();
        // Header: total_len = 16 (header) + 16 (section header) + 8 (section data) = 40
        data.extend_from_slice(&build_header(1, 0, 40, 1));
        // Section header: kind=0x1234, offset=32, len=8, align=4
        data.extend_from_slice(&build_section_header(0x1234, 32, 8, 4));
        // Section data
        data.extend_from_slice(&[0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE, 0xBA, 0xBE]);

        let view = PackView::parse(&data).expect("should parse valid pack");
        assert_eq!(view.section_count(), 1);

        let section = view.get_section(0x1234).expect("should find section");
        assert_eq!(section, &[0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE, 0xBA, 0xBE]);
    }

    #[test]
    fn parse_valid_with_multiple_sections() {
        let mut data = std::vec::Vec::new();
        // Header: total_len = 16 + 32 (2 section headers) + 12 (section data) = 60
        data.extend_from_slice(&build_header(1, 0, 60, 2));
        // Section 1: kind=1, offset=48, len=4, align=4
        data.extend_from_slice(&build_section_header(1, 48, 4, 4));
        // Section 2: kind=2, offset=52, len=8, align=4
        data.extend_from_slice(&build_section_header(2, 52, 8, 4));
        // Section 1 data
        data.extend_from_slice(&[0x11, 0x22, 0x33, 0x44]);
        // Section 2 data
        data.extend_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x00, 0x11]);

        let view = PackView::parse(&data).expect("should parse valid pack");
        assert_eq!(view.section_count(), 2);

        let section1 = view.get_section(1).expect("should find section 1");
        assert_eq!(section1, &[0x11, 0x22, 0x33, 0x44]);

        let section2 = view.get_section(2).expect("should find section 2");
        assert_eq!(section2, &[0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x00, 0x11]);
    }

    #[test]
    fn get_section_not_found() {
        let header = build_header(1, 0, 16, 0);
        let view = PackView::parse(&header).expect("should parse valid empty pack");
        assert!(view.get_section(0x9999).is_none());
    }

    // =========================================================================
    // Typed View Tests
    // =========================================================================

    /// Helper to build a StrRef (8 bytes): off(4) + len(2) + pad(2)
    fn build_strref(off: u32, len: u16) -> [u8; 8] {
        let mut data = [0u8; 8];
        data[0..4].copy_from_slice(&off.to_le_bytes());
        data[4..6].copy_from_slice(&len.to_le_bytes());
        // pad bytes are 0
        data
    }

    /// Helper to build a MoveRecord (32 bytes).
    fn build_move_record(
        move_id: u16,
        mesh_key: u16,
        keyframes_key: u16,
        move_type: u8,
        trigger: u8,
        guard: u8,
        flags: u8,
        startup: u8,
        active: u8,
        recovery: u8,
        total: u16,
        damage: u16,
        hitstun: u8,
        blockstun: u8,
        hitstop: u8,
        hit_windows_off: u32,
        hit_windows_len: u16,
        hurt_windows_off: u16,
        hurt_windows_len: u16,
    ) -> [u8; 32] {
        let mut data = [0u8; 32];
        data[0..2].copy_from_slice(&move_id.to_le_bytes());
        data[2..4].copy_from_slice(&mesh_key.to_le_bytes());
        data[4..6].copy_from_slice(&keyframes_key.to_le_bytes());
        data[6] = move_type;
        data[7] = trigger;
        data[8] = guard;
        data[9] = flags;
        data[10] = startup;
        data[11] = active;
        data[12] = recovery;
        data[13] = 0; // reserved
        data[14..16].copy_from_slice(&total.to_le_bytes());
        data[16..18].copy_from_slice(&damage.to_le_bytes());
        data[18] = hitstun;
        data[19] = blockstun;
        data[20] = hitstop;
        data[21] = 0; // reserved
        data[22..26].copy_from_slice(&hit_windows_off.to_le_bytes());
        data[26..28].copy_from_slice(&hit_windows_len.to_le_bytes());
        data[28..30].copy_from_slice(&hurt_windows_off.to_le_bytes());
        data[30..32].copy_from_slice(&hurt_windows_len.to_le_bytes());
        data
    }

    /// Build a minimal FSPK pack containing:
    /// - STRING_TABLE: "glitch.5h" (9 bytes)
    /// - MESH_KEYS: one StrRef pointing to "glitch.5h"
    /// - MOVES: one MoveRecord with mesh_key=0
    fn build_minimal_typed_pack() -> std::vec::Vec<u8> {
        let mut data = std::vec::Vec::new();

        // String data: "glitch.5h" (9 bytes)
        let string_data = b"glitch.5h";
        let string_len = string_data.len() as u32; // 9

        // Calculate offsets:
        // Header: 16 bytes
        // 3 section headers: 48 bytes (16 * 3)
        // Section data starts at: 64

        // STRING_TABLE at offset 64, len 9
        // MESH_KEYS at offset 73 (64 + 9), but align to 4 -> 76, len 8
        // MOVES at offset 84 (76 + 8), len 32

        // Actually let's align properly:
        // offset 64: STRING_TABLE (9 bytes) -> ends at 73
        // offset 76 (aligned to 4): MESH_KEYS (8 bytes) -> ends at 84
        // offset 84 (aligned to 4): MOVES (32 bytes) -> ends at 116

        let string_table_off: u32 = 64;
        let mesh_keys_off: u32 = 76; // 64 + 9 = 73, round up to 76 (4-byte aligned)
        let moves_off: u32 = 84; // 76 + 8 = 84 (already aligned)

        let total_len: u32 = 116; // 84 + 32

        // Build header: 3 sections
        data.extend_from_slice(&build_header(1, 0, total_len, 3));

        // Section headers
        data.extend_from_slice(&build_section_header(
            SECTION_STRING_TABLE,
            string_table_off,
            string_len,
            1,
        ));
        data.extend_from_slice(&build_section_header(SECTION_MESH_KEYS, mesh_keys_off, 8, 4));
        data.extend_from_slice(&build_section_header(SECTION_MOVES, moves_off, 32, 4));

        // String table data
        data.extend_from_slice(string_data);

        // Padding to align mesh_keys (73 -> 76 = 3 bytes padding)
        data.extend_from_slice(&[0, 0, 0]);

        // Mesh keys: one StrRef pointing to "glitch.5h" at offset 0, len 9
        data.extend_from_slice(&build_strref(0, 9));

        // Move record: mesh_key=0, keyframes_key=0xFFFF (none)
        data.extend_from_slice(&build_move_record(
            0,      // move_id
            0,      // mesh_key (index 0 in MESH_KEYS)
            KEY_NONE, // keyframes_key (none)
            1,      // move_type
            2,      // trigger
            3,      // guard
            4,      // flags
            5,      // startup
            3,      // active
            7,      // recovery
            15,     // total
            100,    // damage
            12,     // hitstun
            8,      // blockstun
            6,      // hitstop
            0,      // hit_windows_off
            0,      // hit_windows_len
            0,      // hurt_windows_off
            0,      // hurt_windows_len
        ));

        assert_eq!(data.len(), total_len as usize);
        data
    }

    #[test]
    fn typed_views_string_from_strref() {
        let pack_data = build_minimal_typed_pack();
        let pack = PackView::parse(&pack_data).expect("should parse typed pack");

        // Get mesh keys view
        let mesh_keys = pack.mesh_keys().expect("should have mesh keys section");
        assert_eq!(mesh_keys.len(), 1);

        // Get the first mesh key strref
        let (off, len) = mesh_keys.get(0).expect("should get mesh key 0");
        assert_eq!(off, 0);
        assert_eq!(len, 9);

        // Resolve to string
        let key_str = pack.string(off, len).expect("should resolve string");
        assert_eq!(key_str, "glitch.5h");
    }

    #[test]
    fn typed_views_move_record_fields() {
        let pack_data = build_minimal_typed_pack();
        let pack = PackView::parse(&pack_data).expect("should parse typed pack");

        let moves = pack.moves().expect("should have moves section");
        assert_eq!(moves.len(), 1);

        let mv = moves.get(0).expect("should get move 0");
        assert_eq!(mv.move_id(), 0);
        assert_eq!(mv.mesh_key(), 0);
        assert_eq!(mv.keyframes_key(), KEY_NONE);
        assert_eq!(mv.move_type(), 1);
        assert_eq!(mv.trigger(), 2);
        assert_eq!(mv.guard(), 3);
        assert_eq!(mv.flags(), 4);
        assert_eq!(mv.startup(), 5);
        assert_eq!(mv.active(), 3);
        assert_eq!(mv.recovery(), 7);
        assert_eq!(mv.total(), 15);
        assert_eq!(mv.damage(), 100);
        assert_eq!(mv.hitstun(), 12);
        assert_eq!(mv.blockstun(), 8);
        assert_eq!(mv.hitstop(), 6);
    }

    #[test]
    fn typed_views_mesh_key_lookup() {
        let pack_data = build_minimal_typed_pack();
        let pack = PackView::parse(&pack_data).expect("should parse typed pack");

        let moves = pack.moves().expect("should have moves section");
        let mv = moves.get(0).expect("should get move 0");
        let mesh_key_idx = mv.mesh_key();

        // Look up the mesh key string
        let mesh_keys = pack.mesh_keys().expect("should have mesh keys section");
        let (off, len) = mesh_keys.get(mesh_key_idx as usize).expect("should get mesh key");
        let mesh_key_str = pack.string(off, len).expect("should resolve mesh key string");
        assert_eq!(mesh_key_str, "glitch.5h");
    }

    #[test]
    fn typed_views_out_of_bounds() {
        let pack_data = build_minimal_typed_pack();
        let pack = PackView::parse(&pack_data).expect("should parse typed pack");

        let mesh_keys = pack.mesh_keys().expect("should have mesh keys section");
        assert!(mesh_keys.get(1).is_none()); // Only 1 key at index 0

        let moves = pack.moves().expect("should have moves section");
        assert!(moves.get(1).is_none()); // Only 1 move at index 0

        // Invalid string offset
        assert!(pack.string(1000, 10).is_none());
    }

    #[test]
    fn typed_views_empty_sections() {
        // Build a pack with empty mesh_keys and moves sections
        let mut data = std::vec::Vec::new();

        // String table with some data
        let string_data = b"test";
        let string_table_off: u32 = 64;
        let total_len: u32 = 68; // 64 + 4

        data.extend_from_slice(&build_header(1, 0, total_len, 3));
        data.extend_from_slice(&build_section_header(
            SECTION_STRING_TABLE,
            string_table_off,
            4,
            1,
        ));
        data.extend_from_slice(&build_section_header(SECTION_MESH_KEYS, 68, 0, 4)); // empty
        data.extend_from_slice(&build_section_header(SECTION_MOVES, 68, 0, 4)); // empty
        data.extend_from_slice(string_data);

        let pack = PackView::parse(&data).expect("should parse pack with empty sections");

        let mesh_keys = pack.mesh_keys().expect("should have mesh keys section");
        assert_eq!(mesh_keys.len(), 0);
        assert!(mesh_keys.is_empty());
        assert!(mesh_keys.get(0).is_none());

        let moves = pack.moves().expect("should have moves section");
        assert_eq!(moves.len(), 0);
        assert!(moves.is_empty());
        assert!(moves.get(0).is_none());
    }

    #[test]
    fn typed_views_keyframes_keys() {
        // Build a pack with keyframes keys
        let mut data = std::vec::Vec::new();

        let string_data = b"anim.idle";
        let string_table_off: u32 = 48; // 16 + 32 (header + 2 section headers)
        let keyframes_keys_off: u32 = 60; // 48 + 9 = 57, round to 60
        let total_len: u32 = 68; // 60 + 8

        data.extend_from_slice(&build_header(1, 0, total_len, 2));
        data.extend_from_slice(&build_section_header(
            SECTION_STRING_TABLE,
            string_table_off,
            9,
            1,
        ));
        data.extend_from_slice(&build_section_header(SECTION_KEYFRAMES_KEYS, keyframes_keys_off, 8, 4));
        data.extend_from_slice(string_data);
        data.extend_from_slice(&[0, 0, 0]); // padding
        data.extend_from_slice(&build_strref(0, 9));

        let pack = PackView::parse(&data).expect("should parse pack with keyframes keys");

        let keyframes_keys = pack.keyframes_keys().expect("should have keyframes keys section");
        assert_eq!(keyframes_keys.len(), 1);

        let (off, len) = keyframes_keys.get(0).expect("should get keyframes key 0");
        let key_str = pack.string(off, len).expect("should resolve string");
        assert_eq!(key_str, "anim.idle");
    }
}
