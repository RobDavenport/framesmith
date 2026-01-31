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
/// * `input` - Frame input (requested move, etc.)
///
/// # Returns
/// New state and whether the move ended this frame.
#[must_use]
pub fn next_frame(
    state: &CharacterState,
    pack: &PackView,
    input: &FrameInput,
) -> FrameResult {
    // Try to transition if a move was requested
    if let Some(target) = input.requested_move {
        if crate::cancel::can_cancel_to(state, pack, target) {
            let mut new_state = *state;
            new_state.current_move = target;
            new_state.frame = 0;
            new_state.hit_confirmed = false;
            new_state.block_confirmed = false;
            // Note: Resource costs are not deducted here. The game is responsible
            // for deducting resources (meter, etc.) when executing moves.
            return FrameResult {
                state: new_state,
                move_ended: false,
            };
        }
    }

    // Advance frame
    let new_state = advance_frame_counter(state);

    // Check if move ended
    let move_ended = if let Some(moves) = pack.moves() {
        if let Some(mv) = moves.get(state.current_move as usize) {
            new_state.frame >= mv.total() as u8
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
            current_move: 0,
            frame: 5,
            ..Default::default()
        };

        let next = advance_frame_counter(&state);
        assert_eq!(next.frame, 6);
        assert_eq!(next.current_move, 0);
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
}
