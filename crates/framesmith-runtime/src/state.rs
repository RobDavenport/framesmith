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
    /// Current state index (0 = idle by convention).
    pub current_state: u16,
    /// Current frame within the state (0-indexed).
    pub frame: u8,
    /// Instance-specific duration override. 0 = use state's default total().
    pub instance_duration: u8,
    /// State connected with a hit (opens on-hit cancel windows).
    pub hit_confirmed: bool,
    /// State was blocked (opens on-block cancel windows).
    pub block_confirmed: bool,
    /// Resource pool values (meter, heat, ammo, etc.).
    pub resources: [u16; MAX_RESOURCES],
}

/// Input for a single frame of simulation.
#[derive(Clone, Copy, Debug, Default)]
pub struct FrameInput {
    /// State to transition to, if cancel is valid.
    /// `None` means continue current state.
    pub requested_state: Option<u16>,
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

/// Report that the current state connected with a hit.
///
/// This opens on-hit cancel windows.
#[inline]
pub fn report_hit(state: &mut CharacterState) {
    state.hit_confirmed = true;
}

/// Report that the current state was blocked.
///
/// This opens on-block cancel windows.
#[inline]
pub fn report_block(state: &mut CharacterState) {
    state.block_confirmed = true;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn character_state_is_copy_and_default() {
        let state = CharacterState::default();
        let copy = state; // Copy
        assert_eq!(state.current_state, copy.current_state);
        assert_eq!(state.frame, 0);
        assert!(!state.hit_confirmed);
        assert!(!state.block_confirmed);
    }

    #[test]
    fn character_state_size_is_small() {
        // Exact size verification for rollback netcode efficiency
        // Still 22 bytes - instance_duration fills existing padding after frame
        assert_eq!(core::mem::size_of::<CharacterState>(), 22);
    }

    #[test]
    fn frame_input_default_has_no_requested_state() {
        let input = FrameInput::default();
        assert!(input.requested_state.is_none());
    }

    #[test]
    fn frame_result_can_hold_events() {
        let result = FrameResult {
            state: CharacterState::default(),
            move_ended: false,
        };
        assert!(!result.move_ended);
    }

    #[test]
    fn report_hit_sets_flag() {
        let mut state = CharacterState::default();
        assert!(!state.hit_confirmed);

        report_hit(&mut state);
        assert!(state.hit_confirmed);
    }

    #[test]
    fn report_block_sets_flag() {
        let mut state = CharacterState::default();
        assert!(!state.block_confirmed);

        report_block(&mut state);
        assert!(state.block_confirmed);
    }
}
