#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod state;
pub mod frame;
pub mod cancel;
pub mod collision;
pub mod resource;

pub use state::{CharacterState, FrameInput, FrameResult};
pub use frame::next_frame;
pub use cancel::{can_cancel_to, available_cancels};
pub use collision::{check_hits, shapes_overlap, HitResult};
pub use resource::{resource, set_resource};

#[cfg(test)]
extern crate std;
