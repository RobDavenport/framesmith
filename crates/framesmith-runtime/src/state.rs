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
}
