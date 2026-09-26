//! FSPK v2 typed payloads: fixed 32-byte nodes, breadth-first children, UTF-8 pool.
//! Runtime views allocate nothing. Objects have sorted keys; arrays retain order.

use crate::bytes::{read_u32_le, read_u64_le};
use crate::Error;

pub const NODE_SIZE: usize = 32;
pub const MAX_DEPTH: u32 = 64;
pub const SECTION_PAYLOAD_NODES: u32 = 25;
pub const SECTION_PAYLOAD_STRINGS: u32 = 26;
pub const VERSION_2: u32 = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Kind {
    Null,
    Bool,
    I64,
    U64,
    F64,
    String,
    Array,
    Object,
}

impl Kind {
    fn from_byte(value: u8) -> Result<Self, Error> {
        match value {
            0 => Ok(Self::Null),
            1 => Ok(Self::Bool),
            2 => Ok(Self::I64),
            3 => Ok(Self::U64),
            4 => Ok(Self::F64),
            5 => Ok(Self::String),
            6 => Ok(Self::Array),
            7 => Ok(Self::Object),
            _ => Err(Error::InvalidFormat),
        }
    }
}

#[derive(Clone, Copy)]
pub struct PayloadView<'a> {
    nodes: &'a [u8],
    strings: &'a [u8],
}

impl<'a> PayloadView<'a> {
    #[cfg(feature = "alloc")]
    pub(crate) fn from_validated(nodes: &'a [u8], strings: &'a [u8]) -> Self {
        Self { nodes, strings }
    }

    /// Validate a complete tree in linear time, without recursion or allocation.
    pub fn parse(nodes: &'a [u8], strings: &'a [u8]) -> Result<Self, Error> {
        if nodes.is_empty() || !nodes.len().is_multiple_of(NODE_SIZE) {
            return Err(Error::InvalidFormat);
        }
        let view = Self { nodes, strings };
        let count = nodes.len() / NODE_SIZE;
        let root = view.at(0);
        if root.word(4) != 0 || root.word(8) != 0 || root.word(20) != 0 {
            return Err(Error::InvalidFormat);
        }
        let mut next_child = 1usize;
        for index in 0..count {
            if index >= next_child {
                return Err(Error::InvalidFormat);
            }
            let value = view.at(index);
            let kind = Kind::from_byte(value.raw()[0])?;
            if value.raw()[1..4] != [0, 0, 0] || value.word(20) > MAX_DEPTH {
                return Err(Error::InvalidFormat);
            }
            value
                .text(value.word(4), value.word(8))
                .ok_or(Error::InvalidFormat)?;
            let start = value.word(12) as usize;
            let len = value.word(16) as usize;
            if matches!(kind, Kind::Array | Kind::Object) {
                if start != next_child || value.bits() != 0 {
                    return Err(Error::InvalidFormat);
                }
                next_child = start.checked_add(len).ok_or(Error::OutOfBounds)?;
                if next_child > count {
                    return Err(Error::OutOfBounds);
                }
                let mut previous = None;
                for child_index in start..next_child {
                    let child = view.at(child_index);
                    if child.word(20) != value.word(20) + 1 {
                        return Err(Error::InvalidFormat);
                    }
                    let key = child
                        .text(child.word(4), child.word(8))
                        .ok_or(Error::InvalidFormat)?;
                    if kind == Kind::Array {
                        if child.word(4) != 0 || child.word(8) != 0 {
                            return Err(Error::InvalidFormat);
                        }
                    } else {
                        if previous.is_some_and(|prev| prev >= key) {
                            return Err(Error::InvalidFormat);
                        }
                        previous = Some(key);
                    }
                }
            } else {
                if start != 0 || len != 0 {
                    return Err(Error::InvalidFormat);
                }
                match kind {
                    Kind::Null if value.bits() != 0 => return Err(Error::InvalidFormat),
                    Kind::Bool if value.bits() > 1 => return Err(Error::InvalidFormat),
                    Kind::F64 if !f64::from_bits(value.bits()).is_finite() => {
                        return Err(Error::InvalidFormat)
                    }
                    Kind::String => {
                        value
                            .text(value.word(24), value.word(28))
                            .ok_or(Error::InvalidFormat)?;
                    }
                    _ => {}
                }
            }
        }
        if next_child != count {
            return Err(Error::InvalidFormat);
        }
        Ok(view)
    }

    pub fn root(self) -> ValueView<'a> {
        self.at(0)
    }
    fn at(self, index: usize) -> ValueView<'a> {
        ValueView {
            payload: self,
            index,
        }
    }
}

#[derive(Clone, Copy)]
pub struct ValueView<'a> {
    payload: PayloadView<'a>,
    index: usize,
}

impl<'a> ValueView<'a> {
    fn raw(self) -> &'a [u8] {
        &self.payload.nodes[self.index * NODE_SIZE..(self.index + 1) * NODE_SIZE]
    }
    fn word(self, off: usize) -> u32 {
        read_u32_le(self.raw(), off).unwrap_or(0)
    }
    fn bits(self) -> u64 {
        read_u64_le(self.raw(), 24).unwrap_or(0)
    }
    fn text(self, off: u32, len: u32) -> Option<&'a str> {
        let end = (off as usize).checked_add(len as usize)?;
        core::str::from_utf8(self.payload.strings.get(off as usize..end)?).ok()
    }
    pub fn kind(self) -> Kind {
        Kind::from_byte(self.raw()[0]).unwrap_or(Kind::Null)
    }
    /// Object member key; array elements and the root have an empty key.
    pub fn key(self) -> &'a str {
        self.text(self.word(4), self.word(8)).unwrap_or("")
    }
    pub fn len(self) -> usize {
        self.word(16) as usize
    }
    pub fn is_empty(self) -> bool {
        self.len() == 0
    }
    pub fn as_bool(self) -> Option<bool> {
        (self.kind() == Kind::Bool).then(|| self.bits() != 0)
    }
    pub fn as_u64(self) -> Option<u64> {
        match self.kind() {
            Kind::U64 => Some(self.bits()),
            Kind::I64 => (self.bits() as i64).try_into().ok(),
            _ => None,
        }
    }
    pub fn as_i64(self) -> Option<i64> {
        match self.kind() {
            Kind::I64 => Some(self.bits() as i64),
            Kind::U64 => self.bits().try_into().ok(),
            _ => None,
        }
    }
    pub fn as_f64(self) -> Option<f64> {
        (self.kind() == Kind::F64).then(|| f64::from_bits(self.bits()))
    }
    pub fn as_str(self) -> Option<&'a str> {
        if self.kind() != Kind::String {
            return None;
        }
        self.text(self.word(24), self.word(28))
    }
    pub fn at(self, index: usize) -> Option<Self> {
        if self.kind() != Kind::Array || index >= self.len() {
            return None;
        }
        Some(self.payload.at(self.word(12) as usize + index))
    }
    pub fn children(self) -> impl ExactSizeIterator<Item = Self> {
        let start = self.word(12) as usize;
        (start..start + self.len()).map(move |index| self.payload.at(index))
    }
    /// Binary search over sorted object members (no hash table or allocations).
    pub fn get(self, key: &str) -> Option<Self> {
        if self.kind() != Kind::Object {
            return None;
        }
        let start = self.word(12) as usize;
        let (mut low, mut high) = (0, self.len());
        while low < high {
            let mid = low + (high - low) / 2;
            let child = self.payload.at(start + mid);
            match child.key().cmp(key) {
                core::cmp::Ordering::Less => low = mid + 1,
                core::cmp::Ordering::Greater => high = mid,
                core::cmp::Ordering::Equal => return Some(child),
            }
        }
        None
    }
}

/// The optional authoring bridge uses JSON values as an in-memory value model;
/// the emitted bytes are typed binary records, never JSON text.
#[cfg(feature = "builder")]
pub mod builder {
    use super::*;
    use alloc::{vec, vec::Vec};
    use serde_json::Value;

    fn u32_len(value: usize) -> Result<u32, Error> {
        value.try_into().map_err(|_| Error::OutOfBounds)
    }
    fn string(pool: &mut Vec<u8>, text: &str) -> Result<(u32, u32), Error> {
        let offset = u32_len(pool.len())?;
        let len = u32_len(text.len())?;
        offset.checked_add(len).ok_or(Error::OutOfBounds)?;
        pool.extend_from_slice(text.as_bytes());
        Ok((offset, len))
    }

    /// Build breadth-first node and string sections. Object order is canonical.
    pub fn encode_parts(root: &Value) -> Result<(Vec<u8>, Vec<u8>), Error> {
        let mut queue = vec![("", root, 0u32)];
        let (mut nodes, mut strings) = (Vec::new(), Vec::new());
        let mut index = 0;
        while index < queue.len() {
            let (key, value, depth) = queue[index];
            if depth > MAX_DEPTH {
                return Err(Error::InvalidFormat);
            }
            let (key_off, key_len) = if key.is_empty() {
                (0, 0)
            } else {
                string(&mut strings, key)?
            };
            let (mut start, mut count, mut bits) = (0u32, 0u32, 0u64);
            let kind = match value {
                Value::Null => Kind::Null,
                Value::Bool(value) => {
                    bits = u64::from(*value);
                    Kind::Bool
                }
                Value::Number(value) if value.is_i64() => {
                    bits = value.as_i64().ok_or(Error::InvalidFormat)? as u64;
                    Kind::I64
                }
                Value::Number(value) if value.is_u64() => {
                    bits = value.as_u64().ok_or(Error::InvalidFormat)?;
                    Kind::U64
                }
                Value::Number(value) => {
                    bits = value.as_f64().ok_or(Error::InvalidFormat)?.to_bits();
                    Kind::F64
                }
                Value::String(value) => {
                    let (off, len) = string(&mut strings, value)?;
                    bits = u64::from(off) | (u64::from(len) << 32);
                    Kind::String
                }
                Value::Array(values) => {
                    start = u32_len(queue.len())?;
                    count = u32_len(values.len())?;
                    queue.extend(values.iter().map(|value| ("", value, depth + 1)));
                    Kind::Array
                }
                Value::Object(values) => {
                    start = u32_len(queue.len())?;
                    count = u32_len(values.len())?;
                    let mut entries: Vec<_> = values.iter().collect();
                    entries.sort_unstable_by(|a, b| a.0.cmp(b.0));
                    queue.extend(
                        entries
                            .into_iter()
                            .map(|(key, value)| (key.as_str(), value, depth + 1)),
                    );
                    Kind::Object
                }
            };
            nodes.extend_from_slice(&[kind as u8, 0, 0, 0]);
            for word in [key_off, key_len, start, count, depth] {
                nodes.extend_from_slice(&word.to_le_bytes());
            }
            nodes.extend_from_slice(&bits.to_le_bytes());
            u32_len(nodes.len())?;
            index += 1;
        }
        Ok((nodes, strings))
    }

    /// Create a standalone FSPK v2 payload; no editor or combat schema required.
    pub fn encode_pack(root: &Value) -> Result<Vec<u8>, Error> {
        let (nodes, strings) = encode_parts(root)?;
        let start = 48u32;
        let string_start = start
            .checked_add(u32_len(nodes.len())?)
            .ok_or(Error::OutOfBounds)?;
        let total = string_start
            .checked_add(u32_len(strings.len())?)
            .ok_or(Error::OutOfBounds)?;
        let mut bytes = Vec::with_capacity(total as usize);
        bytes.extend_from_slice(b"FSPK");
        for word in [
            VERSION_2,
            total,
            2,
            SECTION_PAYLOAD_NODES,
            start,
            u32_len(nodes.len())?,
            4,
            SECTION_PAYLOAD_STRINGS,
            string_start,
            u32_len(strings.len())?,
            1,
        ] {
            bytes.extend_from_slice(&word.to_le_bytes());
        }
        bytes.extend_from_slice(&nodes);
        bytes.extend_from_slice(&strings);
        Ok(bytes)
    }

    impl ValueView<'_> {
        /// Optional authoring/debug conversion, not needed for runtime access.
        pub fn to_json(self) -> Value {
            match self.kind() {
                Kind::Null => Value::Null,
                Kind::Bool => self.as_bool().unwrap_or(false).into(),
                Kind::I64 => self.as_i64().unwrap_or(0).into(),
                Kind::U64 => self.as_u64().unwrap_or(0).into(),
                Kind::F64 => self.as_f64().unwrap_or(0.0).into(),
                Kind::String => self.as_str().unwrap_or("").into(),
                Kind::Array => Value::Array(self.children().map(Self::to_json).collect()),
                Kind::Object => Value::Object(
                    self.children()
                        .map(|child| (child.key().into(), child.to_json()))
                        .collect(),
                ),
            }
        }
    }
}

#[cfg(all(test, feature = "builder"))]
mod tests {
    use super::*;
    use builder::{encode_pack, encode_parts};
    use serde_json::json;

    #[test]
    fn typed_payload_roundtrip_and_malformed_tree_contract() {
        let value = json!({"empty": [], "obj": {}, "unicode": "剣", "null": null,
            "nested": [true, -12, u64::MAX, 0.12345678901234567], "a.b": 1, "a": {"b": 2}});
        let bytes = encode_pack(&value).unwrap();
        let pack = crate::PackView::parse(&bytes).unwrap();
        assert_eq!(pack.payload().unwrap().root().to_json(), value);
        let owned = crate::OwnedPack::new(bytes.clone()).unwrap();
        assert_eq!(owned.view().payload().unwrap().root().to_json(), value);
        let (nodes, strings) = encode_parts(&value).unwrap();
        for field in [0usize, 4, 8, 12, 16, 20] {
            let mut bad = nodes.clone();
            bad[field..field + 4].copy_from_slice(&u32::MAX.to_le_bytes());
            assert!(PayloadView::parse(&bad, &strings).is_err(), "field {field}");
        }
        assert!(PayloadView::parse(&nodes[..nodes.len() - 1], &strings).is_err());
        let mut bad = strings.clone();
        bad.fill(255);
        assert!(PayloadView::parse(&nodes, &bad).is_err());
        assert_eq!(bytes, encode_pack(&value).unwrap());
    }
}
