/// Maximum number of resource pools per character.
pub const MAX_RESOURCES: usize = 8;

/// Character simulation state.
///
/// This struct is intentionally small, `Copy`, and deterministic for:
/// - Cheap cloning (rollback netcode)
/// - No heap allocations (no_std compatible)
/// - Predictable simulation (no floats, no randomness)
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct CharacterState {
    /// Current move index (0 = idle by convention).
    pub current_move: u16,
    /// Current frame within the move (0-indexed).
    pub frame: u8,
    /// Move connected with a hit (opens on-hit cancel windows).
    pub hit_confirmed: bool,
    /// Move was blocked (opens on-block cancel windows).
    pub block_confirmed: bool,
    /// Resource pool values (meter, heat, ammo, etc.).
    pub resources: [u16; MAX_RESOURCES],
}

/// Input for a single frame of simulation.
#[derive(Clone, Copy, Debug, Default)]
pub struct FrameInput {
    /// Move to transition to, if cancel is valid.
    /// `None` means continue current move.
    pub requested_move: Option<u16>,
}

/// Result of simulating one frame.
#[derive(Clone, Copy, Debug)]
pub struct FrameResult {
    /// The new character state after this frame.
    pub state: CharacterState,
    /// True if the move reached its final frame.
    /// Game decides whether to loop or transition.
    pub move_ended: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn character_state_is_copy_and_default() {
        let state = CharacterState::default();
        let copy = state; // Copy
        assert_eq!(state.current_move, copy.current_move);
        assert_eq!(state.frame, 0);
        assert!(!state.hit_confirmed);
        assert!(!state.block_confirmed);
    }

    #[test]
    fn character_state_size_is_small() {
        // State should be small enough for cheap copies (rollback netcode)
        assert!(core::mem::size_of::<CharacterState>() <= 32);
    }

    #[test]
    fn frame_input_default_has_no_requested_move() {
        let input = FrameInput::default();
        assert!(input.requested_move.is_none());
    }

    #[test]
    fn frame_result_can_hold_events() {
        let result = FrameResult {
            state: CharacterState::default(),
            move_ended: false,
        };
        assert!(!result.move_ended);
    }
}
