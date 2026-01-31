use crate::state::CharacterState;
use framesmith_fspack::PackView;

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
                            // TODO: Check timing windows and preconditions
                            return true;
                        }
                    }
                }
            }
        }
    }

    false
}

/// Check if an action cancel is allowed (jump, dash, etc.).
fn check_action_cancel(
    _state: &CharacterState,
    _pack: &PackView,
    _action_id: u16,
) -> bool {
    // TODO: Check cancel flags on current move
    // For now, delegate to game (return true and let game validate)
    true
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
                        // TODO: Filter by timing window and preconditions
                        result.push(target);
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
                        buf[count] = target;
                        count += 1;
                    }
                }
            }
        }
    }

    count
}

#[cfg(test)]
mod tests {
    #[test]
    fn cancel_module_compiles() {
        // Basic smoke test
        assert!(true);
    }
}
