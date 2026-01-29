#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod bytes;
pub mod error;
pub mod view;

pub use error::Error;
pub use view::{
    KeyframesKeysView, MeshKeysView, MoveView, MovesView, PackView,
    // Constants
    HEADER_SIZE, KEY_NONE, MAGIC, MAX_SECTIONS, MOVE_RECORD_SIZE, SECTION_CANCELS_U16,
    SECTION_HEADER_SIZE, SECTION_HIT_WINDOWS, SECTION_HURT_WINDOWS, SECTION_KEYFRAMES_KEYS,
    SECTION_MESH_KEYS, SECTION_MOVES, SECTION_SHAPES, SECTION_STRING_TABLE, STRREF_SIZE, VERSION,
};

#[cfg(test)]
extern crate std;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_empty_fails_too_short() {
        let result = PackView::parse(&[]);
        assert!(matches!(result, Err(Error::TooShort)));
    }
}
