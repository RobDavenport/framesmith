#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod bytes;
pub mod error;
pub mod view;

pub use error::Error;
pub use view::{
    CancelFlags,
    EventArgView,
    EventArgsView,
    EventEmitView,
    EventEmitsView,
    KeyframesKeysView,
    MeshKeysView,
    MoveExtrasRecordView,
    MoveExtrasView,
    MoveNotifiesView,
    MoveNotifyView,
    MoveResourceCostView,
    MoveResourceCostsView,
    MoveResourceDeltaView,
    MoveResourceDeltasView,
    MoveResourcePreconditionView,
    MoveResourcePreconditionsView,
    MoveView,
    MovesView,
    PackView,
    ResourceDefView,
    ResourceDefsView,
    // Constants
    EVENT_ARG_SIZE,
    EVENT_ARG_TAG_BOOL,
    EVENT_ARG_TAG_F32,
    EVENT_ARG_TAG_I64,
    EVENT_ARG_TAG_STRING,
    EVENT_EMIT_SIZE,
    HEADER_SIZE,
    KEY_NONE,
    MAGIC,
    MAX_SECTIONS,
    MOVE_EXTRAS_SIZE,
    MOVE_NOTIFY_SIZE,
    MOVE_RECORD_SIZE,
    MOVE_RESOURCE_COST_SIZE,
    MOVE_RESOURCE_DELTA_SIZE,
    MOVE_RESOURCE_PRECONDITION_SIZE,
    OPT_U16_NONE,
    RESOURCE_DEF_SIZE,
    RESOURCE_DELTA_TRIGGER_ON_BLOCK,
    RESOURCE_DELTA_TRIGGER_ON_HIT,
    RESOURCE_DELTA_TRIGGER_ON_USE,
    SECTION_CANCELS_U16,
    SECTION_EVENT_ARGS,
    SECTION_EVENT_EMITS,
    SECTION_HEADER_SIZE,
    SECTION_HIT_WINDOWS,
    SECTION_HURT_WINDOWS,
    SECTION_KEYFRAMES_KEYS,
    SECTION_MESH_KEYS,
    SECTION_MOVES,
    SECTION_MOVE_EXTRAS,
    SECTION_MOVE_NOTIFIES,
    SECTION_MOVE_RESOURCE_COSTS,
    SECTION_MOVE_RESOURCE_DELTAS,
    SECTION_MOVE_RESOURCE_PRECONDITIONS,
    SECTION_RESOURCE_DEFS,
    SECTION_SHAPES,
    SECTION_STRING_TABLE,
    STRREF_SIZE,
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
