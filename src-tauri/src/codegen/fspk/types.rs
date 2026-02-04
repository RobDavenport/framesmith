//! Core types for FSPK binary packing.

use std::collections::HashMap;

// Re-export StringTable and StrRef from builders module
pub use super::builders::{StrRef, StringTable};

/// Cancel lookup data for export.
///
/// Maps move input notation to move index for resolving cancel denies.
pub struct CancelLookup<'a> {
    /// Map from input notation to move index
    pub input_to_index: HashMap<&'a str, u16>,
}

/// Packed move data with backing arrays.
pub struct PackedMoveData {
    /// MOVES section: array of MoveRecord (36 bytes each)
    pub moves: Vec<u8>,
    /// SHAPES section: array of Shape12 (12 bytes each)
    pub shapes: Vec<u8>,
    /// HIT_WINDOWS section: array of HitWindow24 (24 bytes each)
    pub hit_windows: Vec<u8>,
    /// HURT_WINDOWS section: array of HurtWindow12 (12 bytes each)
    pub hurt_windows: Vec<u8>,
    /// PUSH_WINDOWS section: array of PushWindow12 (12 bytes each, same format as HurtWindow12)
    pub push_windows: Vec<u8>,
}
