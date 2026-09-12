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

fn resource_index(pack: &framesmith_fspack::PackView, off: u32, len: u16) -> Option<usize> {
    let name = pack.string(off, len)?;
    let defs = pack.resource_defs()?;
    (0..defs.len().min(MAX_RESOURCES)).find(|&i| {
        defs.get(i)
            .and_then(|def| pack.string(def.name_off(), def.name_len()))
            == Some(name)
    })
}

/// Pay all costs atomically. Unknown resources, malformed records and insufficient
/// balances return false without changing state; repeated costs accumulate.
pub fn apply_resource_costs(
    state: &mut CharacterState,
    pack: &framesmith_fspack::PackView,
    move_index: u16,
) -> bool {
    if pack
        .states()
        .and_then(|states| states.get(move_index as usize))
        .is_none()
    {
        return false;
    }
    let Some(extras) = pack.state_extras() else {
        return true;
    };
    let Some(extra) = extras.get(move_index as usize) else {
        return false;
    };
    let (off, len) = extra.resource_costs();
    if len == 0 {
        return true;
    }
    let Some(costs) = pack.move_resource_costs() else {
        return false;
    };
    let mut balances = state.resources;
    for i in 0..usize::from(len) {
        let Some(cost) = costs.get_at(off, i) else {
            return false;
        };
        let Some(index) = resource_index(pack, cost.name_off(), cost.name_len()) else {
            return false;
        };
        let Some(value) = balances[index].checked_sub(cost.amount()) else {
            return false;
        };
        balances[index] = value;
    }
    state.resources = balances;
    true
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
    if pack
        .states()
        .and_then(|states| states.get(move_index as usize))
        .is_none()
    {
        return false;
    }
    let Some(extras) = pack.state_extras() else {
        return true;
    };
    let Some(extra) = extras.get(move_index as usize) else {
        return false;
    };
    let (off, len) = extra.resource_preconditions();
    if len == 0 {
        return true;
    }
    let Some(preconditions) = pack.move_resource_preconditions() else {
        return false;
    };
    for i in 0..usize::from(len) {
        let Some(pre) = preconditions.get_at(off, i) else {
            return false;
        };
        let Some(index) = resource_index(pack, pre.name_off(), pre.name_len()) else {
            return false;
        };
        if !check_precondition_value(state.resources[index], pre.min(), pre.max()) {
            return false;
        }
    }
    true
}

/// Initialize resources atomically. False means the pack exceeds this optional
/// helper's capacity or contains a start value above its maximum.
pub fn init_resources(state: &mut CharacterState, pack: &framesmith_fspack::PackView) -> bool {
    let mut resources = [0; MAX_RESOURCES];
    if let Some(defs) = pack.resource_defs() {
        if defs.len() > MAX_RESOURCES {
            return false;
        }
        for (i, slot) in resources.iter_mut().enumerate().take(defs.len()) {
            let Some(def) = defs.get(i) else {
                return false;
            };
            if def.start() > def.max() {
                return false;
            }
            *slot = def.start();
        }
    }
    state.resources = resources;
    true
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
        set_resource(&mut state, 1, 50); // heat

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
