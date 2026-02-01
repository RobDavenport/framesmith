#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod bytes;
pub mod error;
pub mod view;

pub use error::Error;
pub use view::{
    CancelFlags,
    CancelsView,
    EventArgView,
    EventArgsView,
    EventEmitView,
    EventEmitsView,
    HitWindowView,
    HitWindowsView,
    HurtWindowView,
    HurtWindowsView,
    KeyframesKeysView,
    MeshKeysView,
    MoveNotifiesView,
    MoveNotifyView,
    MoveResourceCostView,
    MoveResourceCostsView,
    MoveResourceDeltaView,
    MoveResourceDeltasView,
    MoveResourcePreconditionView,
    MoveResourcePreconditionsView,
    PackView,
    PushWindowView,
    PushWindowsView,
    ResourceDefView,
    ResourceDefsView,
    ShapeView,
    ShapesView,
    StateExtrasRecordView,
    StateExtrasView,
    StateView,
    StatesView,
    // Constants
    EVENT_ARG_SIZE,
    EVENT_ARG_TAG_BOOL,
    EVENT_ARG_TAG_F32,
    EVENT_ARG_TAG_I64,
    EVENT_ARG_TAG_STRING,
    EVENT_EMIT_SIZE,
    HEADER_SIZE,
    HIT_WINDOW_SIZE,
    HURT_WINDOW_SIZE,
    KEY_NONE,
    MAGIC,
    MAX_SECTIONS,
    MOVE_NOTIFY_SIZE,
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
    SECTION_MOVE_NOTIFIES,
    SECTION_MOVE_RESOURCE_COSTS,
    SECTION_MOVE_RESOURCE_DELTAS,
    SECTION_MOVE_RESOURCE_PRECONDITIONS,
    SECTION_RESOURCE_DEFS,
    SECTION_SHAPES,
    SECTION_STATES,
    SECTION_STATE_EXTRAS,
    SECTION_STRING_TABLE,
    SHAPE_KIND_AABB,
    SHAPE_KIND_CAPSULE,
    SHAPE_KIND_CIRCLE,
    SHAPE_KIND_RECT,
    SHAPE_SIZE,
    STATE_EXTRAS_SIZE,
    STATE_RECORD_SIZE,
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
