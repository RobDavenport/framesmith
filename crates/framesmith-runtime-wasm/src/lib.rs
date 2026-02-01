//! WebAssembly bindings for framesmith-runtime.
//!
//! This crate provides a high-level `TrainingSession` API for running
//! character simulations in the browser.

use framesmith_fspack::PackView;
use framesmith_runtime::{
    available_cancels, check_hits, init_resources, next_frame,
    CharacterState as RtCharacterState, FrameInput, HitResult as RtHitResult,
};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

/// Dummy behavior states for training mode.
#[wasm_bindgen]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DummyState {
    #[default]
    Stand,
    Crouch,
    Jump,
    BlockStand,
    BlockCrouch,
    BlockAuto,
}

/// Character state exposed to JavaScript.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CharacterState {
    pub current_state: u32,
    pub frame: u32,
    pub hit_confirmed: bool,
    pub block_confirmed: bool,
    pub resources: Vec<u32>,
}

impl From<&RtCharacterState> for CharacterState {
    fn from(state: &RtCharacterState) -> Self {
        CharacterState {
            current_state: state.current_state as u32,
            frame: state.frame as u32,
            hit_confirmed: state.hit_confirmed,
            block_confirmed: state.block_confirmed,
            resources: state.resources.iter().map(|&r| r as u32).collect(),
        }
    }
}

/// Hit result exposed to JavaScript.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HitResult {
    pub attacker_move: u32,
    pub window_index: u32,
    pub damage: u32,
    pub chip_damage: u32,
    pub hitstun: u32,
    pub blockstun: u32,
    pub hitstop: u32,
    pub guard: u32,
    pub hit_pushback: i32,
    pub block_pushback: i32,
}

impl From<&RtHitResult> for HitResult {
    fn from(hit: &RtHitResult) -> Self {
        HitResult {
            attacker_move: hit.attacker_move as u32,
            window_index: hit.window_index as u32,
            damage: hit.damage as u32,
            chip_damage: hit.chip_damage as u32,
            hitstun: hit.hitstun as u32,
            blockstun: hit.blockstun as u32,
            hitstop: hit.hitstop as u32,
            guard: hit.guard as u32,
            hit_pushback: hit.hit_pushback,
            block_pushback: hit.block_pushback,
        }
    }
}

/// Result of a single frame tick.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FrameResult {
    pub player: CharacterState,
    pub dummy: CharacterState,
    pub hits: Vec<HitResult>,
}

/// Training session for simulating a player character against a dummy.
///
/// Holds the FSPK data and character states for both player and dummy.
#[wasm_bindgen]
pub struct TrainingSession {
    // Owned copies of the pack data
    player_pack_data: Vec<u8>,
    dummy_pack_data: Vec<u8>,
    // Current character states
    player_state: RtCharacterState,
    dummy_state: RtCharacterState,
    // Character positions (in pixels)
    player_pos: (i32, i32),
    dummy_pos: (i32, i32),
    // Last hit results (cached for hit_results() call)
    last_hits: Vec<RtHitResult>,
}

#[wasm_bindgen]
impl TrainingSession {
    /// Create a new training session with the given FSPK data.
    ///
    /// # Arguments
    /// * `player_fspk` - FSPK binary data for the player character
    /// * `dummy_fspk` - FSPK binary data for the dummy character
    ///
    /// # Errors
    /// Returns an error if the FSPK data is invalid.
    #[wasm_bindgen(constructor)]
    pub fn new(player_fspk: &[u8], dummy_fspk: &[u8]) -> Result<TrainingSession, JsError> {
        // Validate the pack data by trying to parse it
        let player_pack = PackView::parse(player_fspk)
            .map_err(|e| JsError::new(&format!("Invalid player FSPK: {:?}", e)))?;
        let dummy_pack = PackView::parse(dummy_fspk)
            .map_err(|e| JsError::new(&format!("Invalid dummy FSPK: {:?}", e)))?;

        // Initialize character states
        let mut player_state = RtCharacterState::default();
        let mut dummy_state = RtCharacterState::default();

        // Initialize resources from pack definitions
        init_resources(&mut player_state, &player_pack);
        init_resources(&mut dummy_state, &dummy_pack);

        Ok(TrainingSession {
            player_pack_data: player_fspk.to_vec(),
            dummy_pack_data: dummy_fspk.to_vec(),
            player_state,
            dummy_state,
            player_pos: (-100, 0), // Player starts on the left
            dummy_pos: (100, 0),   // Dummy starts on the right
            last_hits: Vec::new(),
        })
    }

    /// Advance the simulation by one frame.
    ///
    /// # Arguments
    /// * `player_input` - State index the player wants to transition to (0xFFFF = no input)
    /// * `dummy_behavior` - How the dummy should behave this frame
    ///
    /// # Returns
    /// A FrameResult containing the new states and any hits that occurred.
    pub fn tick(&mut self, player_input: u32, dummy_behavior: DummyState) -> Result<JsValue, JsError> {
        // PackView::parse is zero-copy: it just validates the header and stores
        // offsets into the existing byte slice. Re-parsing each frame is cheap
        // (~100ns) and avoids lifetime complexity from caching the view.
        let player_pack = PackView::parse(&self.player_pack_data)
            .map_err(|e| JsError::new(&format!("Invalid player FSPK: {:?}", e)))?;
        let dummy_pack = PackView::parse(&self.dummy_pack_data)
            .map_err(|e| JsError::new(&format!("Invalid dummy FSPK: {:?}", e)))?;

        // Build player input
        let player_frame_input = FrameInput {
            requested_state: if player_input == 0xFFFF {
                None
            } else {
                Some(player_input as u16)
            },
        };

        // Build dummy input based on behavior
        let dummy_state = self.compute_dummy_state(dummy_behavior, &dummy_pack);
        let dummy_frame_input = FrameInput {
            requested_state: dummy_state,
        };

        // Advance player state
        let player_result = next_frame(&self.player_state, &player_pack, &player_frame_input);
        self.player_state = player_result.state;

        // Handle move completion for player
        if player_result.move_ended {
            Self::handle_move_ended(&mut self.player_state, &player_pack);
        }

        // Advance dummy state
        let dummy_result = next_frame(&self.dummy_state, &dummy_pack, &dummy_frame_input);
        self.dummy_state = dummy_result.state;

        // Handle move completion for dummy
        if dummy_result.move_ended {
            Self::handle_move_ended(&mut self.dummy_state, &dummy_pack);
        }

        // Check for hits (player attacking dummy)
        let hits_result = check_hits(
            &self.player_state,
            &player_pack,
            self.player_pos,
            &self.dummy_state,
            &dummy_pack,
            self.dummy_pos,
        );

        // Debug: Log hit detection info
        #[cfg(debug_assertions)]
        {
            // Get move info for debugging
            if let Some(moves) = player_pack.states() {
                if let Some(mv) = moves.get(self.player_state.current_state as usize) {
                    let hit_count = mv.hit_windows_len();
                    if hit_count > 0 {
                        // Get hit window details
                        let mut hw_info = String::new();
                        if let Some(hit_windows) = player_pack.hit_windows() {
                            for i in 0..hit_count as usize {
                                if let Some(hw) = hit_windows.get_at(mv.hit_windows_off(), i) {
                                    hw_info.push_str(&format!(
                                        " hw[{}]: frames={}-{}, damage={}, shapes_off={}, shapes_len={}",
                                        i, hw.start_frame(), hw.end_frame(), hw.damage(),
                                        hw.shapes_off(), hw.shapes_len()
                                    ));

                                    // Get shape details
                                    if let Some(shapes) = player_pack.shapes() {
                                        for j in 0..hw.shapes_len() as usize {
                                            if let Some(shape) = shapes.get_at(hw.shapes_off(), j) {
                                                hw_info.push_str(&format!(
                                                    " shape[{}]: kind={}, x={}, y={}, w={}, h={}",
                                                    j, shape.kind(), shape.x_px(), shape.y_px(),
                                                    shape.width_px(), shape.height_px()
                                                ));
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Get dummy hurtbox info
                        let mut hrt_info = String::new();
                        if let Some(dummy_moves) = dummy_pack.states() {
                            if let Some(dummy_mv) = dummy_moves.get(self.dummy_state.current_state as usize) {
                                if let Some(hurt_windows) = dummy_pack.hurt_windows() {
                                    for i in 0..dummy_mv.hurt_windows_len() as usize {
                                        if let Some(hrt) = hurt_windows.get_at(dummy_mv.hurt_windows_off(), i) {
                                            hrt_info.push_str(&format!(
                                                " hrt[{}]: frames={}-{}, shapes_off={}, shapes_len={}",
                                                i, hrt.start_frame(), hrt.end_frame(),
                                                hrt.shapes_off(), hrt.shapes_len()
                                            ));

                                            if let Some(shapes) = dummy_pack.shapes() {
                                                for j in 0..hrt.shapes_len() as usize {
                                                    if let Some(shape) = shapes.get_at(hrt.shapes_off(), j) {
                                                        hrt_info.push_str(&format!(
                                                            " shape[{}]: x={}, y={}, w={}, h={}",
                                                            j, shape.x_px(), shape.y_px(),
                                                            shape.width_px(), shape.height_px()
                                                        ));
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        web_sys::console::log_1(&format!(
                            "[WASM] state={}, frame={}, player_pos={:?}, dummy_pos={:?}, hits={}{}{}",
                            self.player_state.current_state,
                            self.player_state.frame,
                            self.player_pos,
                            self.dummy_pos,
                            hits_result.len(),
                            hw_info,
                            hrt_info
                        ).into());
                    }
                }
            }
        }

        // Store hits for later retrieval
        self.last_hits.clear();
        for hit in hits_result.iter() {
            self.last_hits.push(*hit);
            // Report hit on player state
            framesmith_runtime::report_hit(&mut self.player_state);
        }

        // Also check dummy attacking player (for reversals, etc.)
        let dummy_hits_result = check_hits(
            &self.dummy_state,
            &dummy_pack,
            self.dummy_pos,
            &self.player_state,
            &player_pack,
            self.player_pos,
        );

        for hit in dummy_hits_result.iter() {
            self.last_hits.push(*hit);
            framesmith_runtime::report_hit(&mut self.dummy_state);
        }

        // Build result
        let result = FrameResult {
            player: CharacterState::from(&self.player_state),
            dummy: CharacterState::from(&self.dummy_state),
            hits: self.last_hits.iter().map(HitResult::from).collect(),
        };

        serde_wasm_bindgen::to_value(&result)
            .map_err(|e| JsError::new(&format!("Serialization error: {:?}", e)))
    }

    /// Get the current player state.
    pub fn player_state(&self) -> Result<JsValue, JsError> {
        let state = CharacterState::from(&self.player_state);
        serde_wasm_bindgen::to_value(&state)
            .map_err(|e| JsError::new(&format!("Serialization error: {:?}", e)))
    }

    /// Get the current dummy state.
    pub fn dummy_state(&self) -> Result<JsValue, JsError> {
        let state = CharacterState::from(&self.dummy_state);
        serde_wasm_bindgen::to_value(&state)
            .map_err(|e| JsError::new(&format!("Serialization error: {:?}", e)))
    }

    /// Get available cancel targets for the player's current state.
    pub fn available_cancels(&self) -> Result<JsValue, JsError> {
        // Zero-copy parse; see comment in tick() for rationale.
        let player_pack = PackView::parse(&self.player_pack_data)
            .map_err(|e| JsError::new(&format!("Invalid player FSPK: {:?}", e)))?;

        let cancels = available_cancels(&self.player_state, &player_pack);
        let cancels_u32: Vec<u32> = cancels.iter().map(|&c| c as u32).collect();

        serde_wasm_bindgen::to_value(&cancels_u32)
            .map_err(|e| JsError::new(&format!("Serialization error: {:?}", e)))
    }

    /// Get the hit results from the last tick.
    pub fn hit_results(&self) -> Result<JsValue, JsError> {
        let hits: Vec<HitResult> = self.last_hits.iter().map(HitResult::from).collect();
        serde_wasm_bindgen::to_value(&hits)
            .map_err(|e| JsError::new(&format!("Serialization error: {:?}", e)))
    }

    /// Reset the session to initial state.
    pub fn reset(&mut self) -> Result<(), JsError> {
        // Zero-copy parse; see comment in tick() for rationale.
        let player_pack = PackView::parse(&self.player_pack_data)
            .map_err(|e| JsError::new(&format!("Invalid player FSPK: {:?}", e)))?;
        let dummy_pack = PackView::parse(&self.dummy_pack_data)
            .map_err(|e| JsError::new(&format!("Invalid dummy FSPK: {:?}", e)))?;

        self.player_state = RtCharacterState::default();
        self.dummy_state = RtCharacterState::default();

        init_resources(&mut self.player_state, &player_pack);
        init_resources(&mut self.dummy_state, &dummy_pack);

        self.player_pos = (-100, 0);
        self.dummy_pos = (100, 0);
        self.last_hits.clear();

        Ok(())
    }

    /// Set character positions (for collision checking).
    pub fn set_positions(&mut self, player_x: i32, player_y: i32, dummy_x: i32, dummy_y: i32) {
        self.player_pos = (player_x, player_y);
        self.dummy_pos = (dummy_x, dummy_y);
    }
}

impl TrainingSession {
    /// Compute what state the dummy should transition to based on its behavior.
    fn compute_dummy_state(&self, behavior: DummyState, _pack: &PackView) -> Option<u16> {
        // For now, dummy just stays in its current state
        // Future: map behavior to specific states (crouch, block, etc.)
        match behavior {
            DummyState::Stand => None,      // Stay idle
            DummyState::Crouch => Some(1),  // Assume state 1 is crouch (game-specific)
            DummyState::Jump => Some(2),    // Assume state 2 is jump
            DummyState::BlockStand => None, // Block is handled by game logic
            DummyState::BlockCrouch => Some(1), // Crouching block
            DummyState::BlockAuto => None,  // Auto-block handled by game logic
        }
    }

    /// Handle move completion - either loop system states or return to idle.
    fn handle_move_ended(state: &mut RtCharacterState, pack: &PackView) {
        // Check if current state is a system state (state 0 = idle, state 1 = crouch)
        // System states loop back to frame 0 instead of transitioning
        const IDLE_STATE: u16 = 0;
        const MAX_SYSTEM_STATE: u16 = 1; // States 0-1 are system states that loop

        if state.current_state <= MAX_SYSTEM_STATE {
            // System state - loop back to frame 0
            state.frame = 0;
        } else {
            // Attack/action state ended - return to idle
            state.current_state = IDLE_STATE;
            state.frame = 0;
            state.hit_confirmed = false;
            state.block_confirmed = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dummy_state_default() {
        assert_eq!(DummyState::default(), DummyState::Stand);
    }

    #[test]
    fn character_state_conversion() {
        let rt_state = RtCharacterState {
            current_state: 5,
            frame: 10,
            instance_duration: 0,
            hit_confirmed: true,
            block_confirmed: false,
            resources: [100, 50, 0, 0, 0, 0, 0, 0],
        };

        let js_state = CharacterState::from(&rt_state);

        assert_eq!(js_state.current_state, 5);
        assert_eq!(js_state.frame, 10);
        assert!(js_state.hit_confirmed);
        assert!(!js_state.block_confirmed);
        assert_eq!(js_state.resources.len(), 8);
        assert_eq!(js_state.resources[0], 100);
        assert_eq!(js_state.resources[1], 50);
    }

    #[test]
    fn hit_result_conversion() {
        let rt_hit = RtHitResult {
            attacker_move: 3,
            window_index: 0,
            damage: 50,
            chip_damage: 5,
            hitstun: 15,
            blockstun: 10,
            hitstop: 8,
            guard: 1,
            hit_pushback: 20,
            block_pushback: 15,
        };

        let js_hit = HitResult::from(&rt_hit);

        assert_eq!(js_hit.attacker_move, 3);
        assert_eq!(js_hit.damage, 50);
        assert_eq!(js_hit.hitstun, 15);
        assert_eq!(js_hit.hit_pushback, 20);
    }
}
