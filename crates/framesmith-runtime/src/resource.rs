use crate::state::{CharacterState, MAX_RESOURCES};

/// Get the current value of a resource by index.
///
/// Returns 0 if the index is out of bounds.
#[inline]
pub fn resource(state: &CharacterState, index: u8) -> u16 {
    state.resources.get(index as usize).copied().unwrap_or(0)
}

/// Set a resource value by index.
///
/// Does nothing if the index is out of bounds.
#[inline]
pub fn set_resource(state: &mut CharacterState, index: u8, value: u16) {
    if let Some(slot) = state.resources.get_mut(index as usize) {
        *slot = value;
    }
}

/// Apply resource costs for a move transition.
///
/// Deducts costs from state. Returns true if all costs were paid,
/// false if any resource was insufficient (costs still deducted).
pub fn apply_resource_costs(
    state: &mut CharacterState,
    pack: &framesmith_fspack::PackView,
    move_index: u16,
) -> bool {
    let extras = match pack.move_extras() {
        Some(e) => e,
        None => return true,
    };
    let extra = match extras.get(move_index as usize) {
        Some(e) => e,
        None => return true,
    };
    let costs_view = match pack.move_resource_costs() {
        Some(c) => c,
        None => return true,
    };
    let resource_defs = pack.resource_defs();

    let (off, len) = extra.resource_costs();
    let mut all_paid = true;

    for i in 0..len as usize {
        if let Some(cost) = costs_view.get_at(off, i) {
            // Find resource index by name
            if let Some(defs) = &resource_defs {
                for res_idx in 0..defs.len().min(MAX_RESOURCES) {
                    if let Some(def) = defs.get(res_idx) {
                        if def.name_off() == cost.name_off() && def.name_len() == cost.name_len() {
                            let current = resource(state, res_idx as u8);
                            if current < cost.amount() {
                                all_paid = false;
                            }
                            set_resource(state, res_idx as u8, current.saturating_sub(cost.amount()));
                            break;
                        }
                    }
                }
            }
        }
    }

    all_paid
}

/// Check if a resource value satisfies a precondition.
#[inline]
pub fn check_precondition_value(value: u16, min: Option<u16>, max: Option<u16>) -> bool {
    if let Some(m) = min {
        if value < m {
            return false;
        }
    }
    if let Some(m) = max {
        if value > m {
            return false;
        }
    }
    true
}

/// Check all resource preconditions for a move.
///
/// Returns true if all preconditions are satisfied.
pub fn check_resource_preconditions(
    state: &CharacterState,
    pack: &framesmith_fspack::PackView,
    move_index: u16,
) -> bool {
    let extras = match pack.move_extras() {
        Some(e) => e,
        None => return true,
    };
    let extra = match extras.get(move_index as usize) {
        Some(e) => e,
        None => return true,
    };
    let preconditions_view = match pack.move_resource_preconditions() {
        Some(p) => p,
        None => return true,
    };
    let resource_defs = match pack.resource_defs() {
        Some(d) => d,
        None => return true,
    };

    let (off, len) = extra.resource_preconditions();

    for i in 0..len as usize {
        if let Some(precond) = preconditions_view.get_at(off, i) {
            // Find resource index by name
            for res_idx in 0..resource_defs.len().min(MAX_RESOURCES) {
                if let Some(def) = resource_defs.get(res_idx) {
                    if def.name_off() == precond.name_off() && def.name_len() == precond.name_len() {
                        let current = resource(state, res_idx as u8);
                        if !check_precondition_value(current, precond.min(), precond.max()) {
                            return false;
                        }
                        break;
                    }
                }
            }
        }
    }

    true
}

/// Initialize resources from pack's resource definitions.
pub fn init_resources(state: &mut CharacterState, pack: &framesmith_fspack::PackView) {
    // Reset all to zero first
    state.resources = [0; MAX_RESOURCES];

    if let Some(defs) = pack.resource_defs() {
        for i in 0..defs.len().min(MAX_RESOURCES) {
            if let Some(def) = defs.get(i) {
                state.resources[i] = def.start();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::CharacterState;

    #[test]
    fn check_preconditions_passes_when_met() {
        // Precondition: resource must be >= 25
        assert!(check_precondition_value(50, Some(25), None));
        // Precondition: resource must be <= 100
        assert!(check_precondition_value(50, None, Some(100)));
        // Precondition: resource must be between 25 and 100
        assert!(check_precondition_value(50, Some(25), Some(100)));
        // No preconditions
        assert!(check_precondition_value(50, None, None));
    }

    #[test]
    fn check_preconditions_fails_when_not_met() {
        assert!(!check_precondition_value(20, Some(25), None)); // below min
        assert!(!check_precondition_value(150, None, Some(100))); // above max
        assert!(!check_precondition_value(20, Some(25), Some(100))); // below min
        assert!(!check_precondition_value(150, Some(25), Some(100))); // above max
    }

    #[test]
    fn get_and_set_resource() {
        let mut state = CharacterState::default();
        assert_eq!(resource(&state, 0), 0);

        set_resource(&mut state, 0, 100);
        assert_eq!(resource(&state, 0), 100);

        set_resource(&mut state, 7, 50);
        assert_eq!(resource(&state, 7), 50);
    }

    #[test]
    fn out_of_bounds_resource_returns_zero() {
        let state = CharacterState::default();
        assert_eq!(resource(&state, 8), 0);
        assert_eq!(resource(&state, 255), 0);
    }

    /// Tests that the resource primitives support the deduction pattern.
    ///
    /// Note: `apply_resource_costs` requires a full PackView with move_extras,
    /// move_resource_costs, and resource_defs sections - too complex for unit tests
    /// in a no_std crate. That function is integration-tested via frame.rs when
    /// processing real .fspk packs.
    #[test]
    fn resource_primitives_support_deduction() {
        let mut state = CharacterState::default();
        set_resource(&mut state, 0, 100); // meter
        set_resource(&mut state, 1, 50);  // heat

        // Simulate deducting 30 from resource 0, 10 from resource 1
        let costs = [(0u8, 30u16), (1u8, 10u16)];
        for (idx, amount) in costs {
            let current = resource(&state, idx);
            set_resource(&mut state, idx, current.saturating_sub(amount));
        }

        assert_eq!(resource(&state, 0), 70);
        assert_eq!(resource(&state, 1), 40);
    }
}
