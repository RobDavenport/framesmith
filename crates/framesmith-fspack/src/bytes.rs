//! Byte reading utilities for no_std environments.

/// Read a little-endian u16 from `data` at `offset`.
/// Returns `None` if `offset + 2 > data.len()`.
#[inline]
pub fn read_u16_le(data: &[u8], offset: usize) -> Option<u16> {
    if offset.checked_add(2)? > data.len() {
        return None;
    }
    Some(u16::from_le_bytes([data[offset], data[offset + 1]]))
}

/// Read a little-endian u32 from `data` at `offset`.
/// Returns `None` if `offset + 4 > data.len()`.
#[inline]
pub fn read_u32_le(data: &[u8], offset: usize) -> Option<u32> {
    if offset.checked_add(4)? > data.len() {
        return None;
    }
    Some(u32::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ]))
}

/// Read a little-endian i16 from `data` at `offset`.
/// Returns `None` if `offset + 2 > data.len()`.
#[inline]
pub fn read_i16_le(data: &[u8], offset: usize) -> Option<i16> {
    if offset.checked_add(2)? > data.len() {
        return None;
    }
    Some(i16::from_le_bytes([data[offset], data[offset + 1]]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_u16_le_valid() {
        let data = [0x34, 0x12];
        assert_eq!(read_u16_le(&data, 0), Some(0x1234));
    }

    #[test]
    fn read_u16_le_offset() {
        let data = [0x00, 0x34, 0x12];
        assert_eq!(read_u16_le(&data, 1), Some(0x1234));
    }

    #[test]
    fn read_u16_le_out_of_bounds() {
        let data = [0x34];
        assert_eq!(read_u16_le(&data, 0), None);
    }

    #[test]
    fn read_u16_le_offset_overflow() {
        let data = [0x34, 0x12];
        assert_eq!(read_u16_le(&data, usize::MAX), None);
    }

    #[test]
    fn read_u32_le_valid() {
        let data = [0x78, 0x56, 0x34, 0x12];
        assert_eq!(read_u32_le(&data, 0), Some(0x12345678));
    }

    #[test]
    fn read_u32_le_out_of_bounds() {
        let data = [0x78, 0x56, 0x34];
        assert_eq!(read_u32_le(&data, 0), None);
    }

    #[test]
    fn read_i16_le_positive() {
        let data = [0x34, 0x12];
        assert_eq!(read_i16_le(&data, 0), Some(0x1234));
    }

    #[test]
    fn read_i16_le_negative() {
        let data = [0xFE, 0xFF]; // -2 in little-endian
        assert_eq!(read_i16_le(&data, 0), Some(-2));
    }
}
