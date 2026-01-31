use crate::state::{CharacterState, FrameInput, FrameResult};
use framesmith_fspack::PackView;

/// Advance frame counter by 1, saturating at u8::MAX.
#[inline]
fn advance_frame_counter(state: &CharacterState) -> CharacterState {
    CharacterState {
        frame: state.frame.saturating_add(1),
        ..*state
    }
}

/// Compute the next frame state for a character.
///
/// This is a pure function - it does not mutate the input state.
/// The game decides whether to apply the returned state.
///
/// # Arguments
/// * `state` - Current character state
/// * `pack` - Character data pack (moves, cancels, etc.)
/// * `input` - Frame input (requested state, etc.)
///
/// # Returns
/// New state and whether the move ended this frame.
#[must_use]
pub fn next_frame(
    state: &CharacterState,
    pack: &PackView,
    input: &FrameInput,
) -> FrameResult {
    // Try to transition if a state was requested
    if let Some(target) = input.requested_state {
        if crate::cancel::can_cancel_to(state, pack, target) {
            let mut new_state = *state;
            new_state.current_state = target;
            new_state.frame = 0;
            new_state.hit_confirmed = false;
            new_state.block_confirmed = false;
            // Apply resource costs for the target move
            crate::resource::apply_resource_costs(&mut new_state, pack, target);
            return FrameResult {
                state: new_state,
                move_ended: false,
            };
        }
    }

    // Advance frame
    let new_state = advance_frame_counter(state);

    // Check if state ended
    // Use instance_duration if set, otherwise use state's default total
    let move_ended = if let Some(moves) = pack.states() {
        if let Some(mv) = moves.get(state.current_state as usize) {
            let effective_duration = if state.instance_duration > 0 {
                state.instance_duration
            } else {
                mv.total() as u8
            };
            new_state.frame >= effective_duration
        } else {
            false
        }
    } else {
        false
    };

    FrameResult {
        state: new_state,
        move_ended,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_frame_advances_frame_counter() {
        let state = CharacterState {
            current_state: 0,
            frame: 5,
            ..Default::default()
        };

        let next = advance_frame_counter(&state);
        assert_eq!(next.frame, 6);
        assert_eq!(next.current_state, 0);
    }

    #[test]
    fn frame_counter_saturates_at_max() {
        let state = CharacterState {
            frame: 255,
            ..Default::default()
        };
        let next = advance_frame_counter(&state);
        assert_eq!(next.frame, 255);
    }

    #[test]
    fn instance_duration_field_behavior() {
        // This test documents the expected behavior:
        // - When instance_duration = 0, use move's total()
        // - When instance_duration > 0, use instance_duration as effective duration

        // Verify the field exists and defaults to 0
        let state = CharacterState::default();
        assert_eq!(state.instance_duration, 0);

        // With a non-zero instance_duration, the effective duration changes
        let state_with_override = CharacterState {
            instance_duration: 10,
            ..Default::default()
        };
        assert_eq!(state_with_override.instance_duration, 10);

        // The actual integration testing of move_ended behavior with
        // instance_duration is done with real packs in roundtrip tests
    }
}
