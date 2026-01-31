// TODO: Implement cancel logic (Task 6)

use crate::state::CharacterState;
use framesmith_fspack::PackView;

/// Check if the current state allows cancelling to the target move.
///
/// This is a stub that always returns false - cancel logic will be
/// implemented in Task 6.
pub fn can_cancel_to(_state: &CharacterState, _pack: &PackView, _target_move: u16) -> bool {
    // TODO: Implement in Task 6
    // For now, always return false (no cancels allowed)
    false
}

/// Return list of moves that can be cancelled to from the current state.
pub fn available_cancels(_current_move: u16) -> &'static [u16] {
    // TODO: Implement in Task 6
    &[]
}
