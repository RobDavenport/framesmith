//! Byte reading utilities for no_std environments.

/// Read a u8 from `data` at `offset`.
/// Returns `None` if `offset >= data.len()`.
#[inline]
pub fn read_u8(data: &[u8], offset: usize) -> Option<u8> {
    data.get(offset).copied()
}

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

/// Read a little-endian u64 from `data` at `offset`.
/// Returns `None` if `offset + 8 > data.len()`.
#[inline]
pub fn read_u64_le(data: &[u8], offset: usize) -> Option<u64> {
    if offset.checked_add(8)? > data.len() {
        return None;
    }
    Some(u64::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
        data[offset + 4],
        data[offset + 5],
        data[offset + 6],
        data[offset + 7],
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

/// Read a little-endian i32 from `data` at `offset`.
/// Returns `None` if `offset + 4 > data.len()`.
#[inline]
pub fn read_i32_le(data: &[u8], offset: usize) -> Option<i32> {
    if offset.checked_add(4)? > data.len() {
        return None;
    }
    Some(i32::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ]))
}

/// Read a little-endian i64 from `data` at `offset`.
/// Returns `None` if `offset + 8 > data.len()`.
#[inline]
pub fn read_i64_le(data: &[u8], offset: usize) -> Option<i64> {
    Some(read_u64_le(data, offset)? as i64)
}

/// Read a little-endian f32 from `data` at `offset`.
/// Returns `None` if `offset + 4 > data.len()`.
#[inline]
pub fn read_f32_le(data: &[u8], offset: usize) -> Option<f32> {
    let bits = read_u32_le(data, offset)?;
    Some(f32::from_bits(bits))
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
