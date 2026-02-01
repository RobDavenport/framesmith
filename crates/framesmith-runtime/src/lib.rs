#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod cancel;
pub mod collision;
pub mod frame;
pub mod resource;
pub mod state;

// Re-export main types
pub use state::{CharacterState, FrameInput, FrameResult, MAX_RESOURCES};
pub use state::{report_block, report_hit};
pub use frame::next_frame;
pub use cancel::{available_cancels_buf, can_cancel_to, ACTION_CHAIN, ACTION_SPECIAL, ACTION_SUPER, ACTION_JUMP};
#[cfg(feature = "alloc")]
pub use cancel::available_cancels;
pub use collision::{aabb_circle_overlap, aabb_overlap, calculate_pushbox_separation, capsule_overlap, check_hits, check_pushbox, circle_overlap, shapes_overlap, Aabb, Capsule, CheckHitsResult, Circle, HitResult, PushboxResult, MAX_HIT_RESULTS};
pub use resource::{apply_resource_costs, check_resource_preconditions, init_resources, resource, set_resource};

// Re-export fspack for convenience
pub use framesmith_fspack::PackView;

#[cfg(test)]
extern crate std;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_api_is_accessible() {
        let state = CharacterState::default();
        let _input = FrameInput::default();
        let _ = resource::resource(&state, 0);
    }
}
