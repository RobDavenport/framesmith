//! Zero-copy view into an FSPK pack.

use crate::bytes::{read_u16_le, read_u32_le};
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
}
