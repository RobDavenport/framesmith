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
}
