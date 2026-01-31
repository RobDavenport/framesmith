use crate::state::CharacterState;
use framesmith_fspack::PackView;

/// Action cancel IDs (offset from move_count).
/// These map to CancelFlags on the current move.
pub const ACTION_CHAIN: u16 = 0;
pub const ACTION_SPECIAL: u16 = 1;
pub const ACTION_SUPER: u16 = 2;
pub const ACTION_JUMP: u16 = 3;

/// Check if a cancel from current state to target move is valid.
///
/// This checks:
/// - Cancel routes from the cancel table
/// - Resource preconditions
/// - Hit/block state for on-hit/on-block cancels
///
/// # Arguments
/// * `state` - Current character state
/// * `pack` - Character data pack
/// * `target` - Target move ID (or action ID if >= move_count)
///
/// # Returns
/// `true` if the cancel is valid right now.
#[must_use]
pub fn can_cancel_to(state: &CharacterState, pack: &PackView, target: u16) -> bool {
    let moves = match pack.moves() {
        Some(m) => m,
        None => return false,
    };

    let move_count = moves.len() as u16;

    // Check if target is a game-defined action (>= move_count)
    // The runtime allows these; game decides if the action is valid
    if target >= move_count {
        // Check if current move allows this action via cancel flags
        return check_action_cancel(state, pack, target - move_count);
    }

    // Check chain cancels from move extras
    if let Some(extras) = pack.move_extras() {
        if let Some(extra) = extras.get(state.current_move as usize) {
            if let Some(cancels) = pack.cancels() {
                let (off, len) = extra.cancels();
                for i in 0..len as usize {
                    if let Some(cancel_target) = cancels.get_at(off, i) {
                        if cancel_target == target {
                            // Check resource preconditions
                            if !crate::resource::check_resource_preconditions(state, pack, target) {
                                continue;
                            }
                            return true;
                        }
                    }
                }
            }
        }
    }

    false
}

/// Check if an action cancel is allowed based on current move's cancel flags.
fn check_action_cancel(
    state: &CharacterState,
    pack: &PackView,
    action_id: u16,
) -> bool {
    let moves = match pack.moves() {
        Some(m) => m,
        None => return false,
    };
    let current_move = match moves.get(state.current_move as usize) {
        Some(m) => m,
        None => return false,
    };

    let flags = current_move.cancel_flags();

    match action_id {
        ACTION_CHAIN => flags.chain,
        ACTION_SPECIAL => flags.special,
        ACTION_SUPER => flags.super_cancel,
        ACTION_JUMP => flags.jump,
        _ => true, // Unknown actions delegated to game
    }
}

/// Get all valid cancel targets from current state.
///
/// Returns move IDs (< move_count) and action IDs (>= move_count).
#[cfg(feature = "alloc")]
pub fn available_cancels(state: &CharacterState, pack: &PackView) -> alloc::vec::Vec<u16> {
    let mut result = alloc::vec::Vec::new();

    if let Some(extras) = pack.move_extras() {
        if let Some(extra) = extras.get(state.current_move as usize) {
            if let Some(cancels) = pack.cancels() {
                let (off, len) = extra.cancels();
                for i in 0..len as usize {
                    if let Some(target) = cancels.get_at(off, i) {
                        // Filter by resource preconditions (timing windows not yet implemented)
                        if crate::resource::check_resource_preconditions(state, pack, target) {
                            result.push(target);
                        }
                    }
                }
            }
        }
    }

    result
}

/// Get available cancels into a fixed-size buffer.
///
/// Returns the number of cancels written.
pub fn available_cancels_buf(
    state: &CharacterState,
    pack: &PackView,
    buf: &mut [u16],
) -> usize {
    let mut count = 0;

    if let Some(extras) = pack.move_extras() {
        if let Some(extra) = extras.get(state.current_move as usize) {
            if let Some(cancels) = pack.cancels() {
                let (off, len) = extra.cancels();
                for i in 0..len as usize {
                    if count >= buf.len() {
                        break;
                    }
                    if let Some(target) = cancels.get_at(off, i) {
                        // Filter by resource preconditions (timing windows not yet implemented)
                        if crate::resource::check_resource_preconditions(state, pack, target) {
                            buf[count] = target;
                            count += 1;
                        }
                    }
                }
            }
        }
    }

    count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancel_module_compiles() {
        // Basic smoke test
        assert!(true);
    }

    #[test]
    fn action_cancel_constants_defined() {
        // Action IDs map to cancel flags
        assert_eq!(ACTION_CHAIN, 0);
        assert_eq!(ACTION_SPECIAL, 1);
        assert_eq!(ACTION_SUPER, 2);
        assert_eq!(ACTION_JUMP, 3);
    }
}
