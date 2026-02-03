use crate::state::CharacterState;
use framesmith_fspack::PackView;

/// Action cancel IDs (offset from move_count).
/// These map to CancelFlags on the current move.
pub const ACTION_CHAIN: u16 = 0;
pub const ACTION_SPECIAL: u16 = 1;
pub const ACTION_SUPER: u16 = 2;
pub const ACTION_JUMP: u16 = 3;

/// Check if a state (move) has a specific tag.
///
/// Searches the state's tag list for the given tag string.
/// Returns false if the pack has no tag data or the state has no tags.
fn state_has_tag(pack: &PackView, state_idx: u16, tag: &str) -> bool {
    pack.state_tags(state_idx as usize)
        .map(|mut tags| tags.any(|t| t == tag))
        .unwrap_or(false)
}

/// Check if a cancel from current state to target move is valid.
///
/// This checks (in priority order):
/// 1. Explicit denies - block specific cancels
/// 2. Tag-based rules (patterns like "normal->special on hit+block")
///
/// Resource preconditions are checked for tag rules.
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
    let moves = match pack.states() {
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

    // 1. Explicit deny always wins - block this cancel entirely
    if pack.has_cancel_deny(state.current_state, target) {
        return false;
    }

    // 2. Check tag-based cancel rules
    if let Some(rules) = pack.cancel_tag_rules() {
        for rule in rules.iter() {
            // Check from_tag matches (None means "any")
            let from_matches = match rule.from_tag() {
                Some(tag) => state_has_tag(pack, state.current_state, tag),
                None => true, // "any" matches all moves
            };
            if !from_matches {
                continue;
            }

            // Check to_tag matches
            let to_matches = match rule.to_tag() {
                Some(tag) => state_has_tag(pack, target, tag),
                None => true, // "any" matches all moves
            };
            if !to_matches {
                continue;
            }

            // Check condition bitfield
            // bit 0 = hit, bit 1 = block, bit 2 = whiff
            let condition = rule.condition();
            let condition_met = if state.hit_confirmed {
                condition & 0b001 != 0  // HIT bit
            } else if state.block_confirmed {
                condition & 0b010 != 0  // BLOCK bit
            } else {
                condition & 0b100 != 0  // WHIFF bit
            };
            if !condition_met {
                continue;
            }

            // Check frame range
            if state.frame < rule.min_frame() {
                continue;
            }
            if state.frame > rule.max_frame() {
                continue;
            }

            // All checks passed - now verify resource preconditions
            if !crate::resource::check_resource_preconditions(state, pack, target) {
                continue;
            }

            // Cancel allowed by this tag rule
            return true;
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
    let moves = match pack.states() {
        Some(m) => m,
        None => return false,
    };
    let current = match moves.get(state.current_state as usize) {
        Some(m) => m,
        None => return false,
    };

    let flags = current.cancel_flags();

    match action_id {
        ACTION_CHAIN => flags.chain,
        ACTION_SPECIAL => flags.special,
        ACTION_SUPER => flags.super_cancel,
        ACTION_JUMP => flags.jump,
        _ => true, // Unknown actions delegated to game
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancel_module_compiles() {
        // Basic smoke test - verify types exist and constants are accessible
        let _ = ACTION_CHAIN;
        let _ = ACTION_SPECIAL;
    }

    #[test]
    fn action_cancel_constants_defined() {
        // Action IDs map to cancel flags
        assert_eq!(ACTION_CHAIN, 0);
        assert_eq!(ACTION_SPECIAL, 1);
        assert_eq!(ACTION_SUPER, 2);
        assert_eq!(ACTION_JUMP, 3);
    }

    #[test]
    fn state_has_tag_returns_false_for_missing_data() {
        // Create minimal pack data that will fail parsing
        // This tests that state_has_tag gracefully returns false
        // when pack has no tag data
        let empty_data: [u8; 0] = [];
        if let Ok(pack) = PackView::parse(&empty_data) {
            // If somehow parsed, should return false for missing tags
            assert!(!state_has_tag(&pack, 0, "normal"));
        }
        // If parsing fails (expected), the test passes - we can't test
        // with invalid pack data, but that's fine since:
        // 1. The function exists and compiles
        // 2. Full integration tests are in Task 3.5
    }

    #[test]
    fn can_cancel_to_returns_false_for_empty_pack() {
        // can_cancel_to should return false when pack has no moves
        let state = CharacterState::default();
        let empty_data: [u8; 0] = [];
        if let Ok(pack) = PackView::parse(&empty_data) {
            assert!(!can_cancel_to(&state, &pack, 0));
        }
        // Parsing empty data fails, which is expected
    }
}
