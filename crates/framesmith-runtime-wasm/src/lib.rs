//! WebAssembly bindings for framesmith-runtime.
//!
//! This crate provides a high-level `TrainingSession` API for running
//! character simulations in the browser.

use framesmith_fspack::PackView;
use framesmith_runtime::{
    available_cancels, check_hits, check_pushbox, init_resources, next_frame,
    CharacterState as RtCharacterState, FrameInput, HitResult as RtHitResult,
    PushboxResult as RtPushboxResult, MAX_RESOURCES,
};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

/// Convert Q24.8 fixed-point to f64.
///
/// Q24.8 uses 8 fractional bits, so dividing by 256 converts to float.
fn from_q24_8(raw: i32) -> f64 {
    raw as f64 / 256.0
}

/// Property value type constant for Q24.8 numeric properties.
const PROP_TYPE_Q24_8: u8 = 0;

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
    pub instance_duration: u32,
    pub hit_confirmed: bool,
    pub block_confirmed: bool,
    pub resources: Vec<u32>,
}

impl From<&RtCharacterState> for CharacterState {
    fn from(state: &RtCharacterState) -> Self {
        CharacterState {
            current_state: state.current_state as u32,
            frame: state.frame as u32,
            instance_duration: state.instance_duration as u32,
            hit_confirmed: state.hit_confirmed,
            block_confirmed: state.block_confirmed,
            resources: state.resources.iter().map(|&r| r as u32).collect(),
        }
    }
}

impl CharacterState {
    fn to_runtime(&self) -> Result<RtCharacterState, String> {
        if self.current_state > u16::MAX as u32 {
            return Err("Snapshot current_state exceeds u16".to_string());
        }
        if self.frame > u8::MAX as u32 {
            return Err("Snapshot frame exceeds u8".to_string());
        }
        if self.instance_duration > u8::MAX as u32 {
            return Err("Snapshot instance_duration exceeds u8".to_string());
        }
        if self.resources.len() > MAX_RESOURCES {
            return Err("Snapshot has too many resource values".to_string());
        }

        let mut resources = [0_u16; MAX_RESOURCES];
        for (idx, value) in self.resources.iter().enumerate() {
            if *value > u16::MAX as u32 {
                return Err("Snapshot resource value exceeds u16".to_string());
            }
            resources[idx] = *value as u16;
        }

        Ok(RtCharacterState {
            current_state: self.current_state as u16,
            frame: self.frame as u8,
            instance_duration: self.instance_duration as u8,
            hit_confirmed: self.hit_confirmed,
            block_confirmed: self.block_confirmed,
            resources,
        })
    }
}

/// Hit result exposed to JavaScript.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HitResult {
    pub attacker_move: u32,
    pub window_index: u32,
    pub blocked: bool,
    pub damage: u32,
    pub chip_damage: u32,
    pub hitstun: u32,
    pub blockstun: u32,
    pub hitstop: u32,
    pub guard: u32,
    pub hit_pushback: i32,
    pub block_pushback: i32,
}

impl HitResult {
    fn from_runtime(hit: &RtHitResult, blocked: bool) -> Self {
        HitResult {
            attacker_move: hit.attacker_move as u32,
            window_index: hit.window_index as u32,
            blocked,
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

impl From<&RtHitResult> for HitResult {
    fn from(hit: &RtHitResult) -> Self {
        Self::from_runtime(hit, false)
    }
}

#[derive(Clone, Copy, Debug)]
struct TrainingHit {
    result: RtHitResult,
    blocked: bool,
}

/// Push separation result exposed to JavaScript.
/// Contains the (dx, dy) separation values if characters are overlapping.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PushSeparation {
    /// Separation for player (negative = left, positive = right)
    pub player_dx: i32,
    /// Separation for dummy (negative = left, positive = right)
    pub dummy_dx: i32,
}

impl From<&RtPushboxResult> for PushSeparation {
    fn from(result: &RtPushboxResult) -> Self {
        PushSeparation {
            player_dx: result.p1_dx,
            dummy_dx: result.p2_dx,
        }
    }
}

/// Result of a single frame tick.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FrameResult {
    pub player: CharacterState,
    pub dummy: CharacterState,
    pub hits: Vec<HitResult>,
    /// Push separation values if characters' pushboxes are overlapping.
    /// None if there is no overlap.
    pub push_separation: Option<PushSeparation>,
}

/// Serializable snapshot used by training mode for frame step-back.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrainingSnapshot {
    pub player: CharacterState,
    pub dummy: CharacterState,
    pub player_x: i32,
    pub player_y: i32,
    pub dummy_x: i32,
    pub dummy_y: i32,
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
    last_hits: Vec<TrainingHit>,
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
    pub fn tick(
        &mut self,
        player_input: u32,
        dummy_behavior: DummyState,
    ) -> Result<JsValue, JsError> {
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

        // Advance player state
        let player_result = next_frame(&self.player_state, &player_pack, &player_frame_input);
        self.player_state = player_result.state;

        // Handle move completion for player
        if player_result.move_ended {
            Self::handle_move_ended(&mut self.player_state, &player_pack);
        }

        // Apply authored dummy stance/block behavior, then let the runtime
        // advance the selected state normally.
        Self::apply_dummy_behavior(&mut self.dummy_state, dummy_behavior, &dummy_pack);
        let dummy_result = next_frame(&self.dummy_state, &dummy_pack, &FrameInput::default());
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
                                                    j,
                                                    shape.kind(),
                                                    shape.x_px(),
                                                    shape.y_px(),
                                                    shape.width_px(),
                                                    shape.height_px()
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
                            if let Some(dummy_mv) =
                                dummy_moves.get(self.dummy_state.current_state as usize)
                            {
                                if let Some(hurt_windows) = dummy_pack.hurt_windows() {
                                    for i in 0..dummy_mv.hurt_windows_len() as usize {
                                        if let Some(hrt) =
                                            hurt_windows.get_at(dummy_mv.hurt_windows_off(), i)
                                        {
                                            hrt_info.push_str(&format!(
                                                " hrt[{}]: frames={}-{}, shapes_off={}, shapes_len={}",
                                                i, hrt.start_frame(), hrt.end_frame(),
                                                hrt.shapes_off(), hrt.shapes_len()
                                            ));

                                            if let Some(shapes) = dummy_pack.shapes() {
                                                for j in 0..hrt.shapes_len() as usize {
                                                    if let Some(shape) =
                                                        shapes.get_at(hrt.shapes_off(), j)
                                                    {
                                                        hrt_info.push_str(&format!(
                                                            " shape[{}]: x={}, y={}, w={}, h={}",
                                                            j,
                                                            shape.x_px(),
                                                            shape.y_px(),
                                                            shape.width_px(),
                                                            shape.height_px()
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
        let dummy_is_blocking = Self::dummy_is_blocking(dummy_behavior);
        for hit in hits_result.iter() {
            self.last_hits.push(TrainingHit {
                result: *hit,
                blocked: dummy_is_blocking,
            });
            if dummy_is_blocking {
                framesmith_runtime::report_block(&mut self.player_state);
                Self::enter_reaction_state(
                    &mut self.dummy_state,
                    &dummy_pack,
                    &["blockstun", "block_stun", "guard_stun"],
                    &["blockstun", "block", "guard"],
                    hit.blockstun,
                );
            } else {
                framesmith_runtime::report_hit(&mut self.player_state);
                Self::enter_reaction_state(
                    &mut self.dummy_state,
                    &dummy_pack,
                    &["hitstun", "hit_stun"],
                    &["hitstun"],
                    hit.hitstun,
                );
            }
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
            self.last_hits.push(TrainingHit {
                result: *hit,
                blocked: false,
            });
            framesmith_runtime::report_hit(&mut self.dummy_state);
            Self::enter_reaction_state(
                &mut self.player_state,
                &player_pack,
                &["hitstun", "hit_stun"],
                &["hitstun"],
                hit.hitstun,
            );
        }

        // Check pushbox collision
        let push_sep = check_pushbox(
            &self.player_state,
            &player_pack,
            self.player_pos,
            &self.dummy_state,
            &dummy_pack,
            self.dummy_pos,
        );

        // Build result
        let result = FrameResult {
            player: CharacterState::from(&self.player_state),
            dummy: CharacterState::from(&self.dummy_state),
            hits: self
                .last_hits
                .iter()
                .map(|hit| HitResult::from_runtime(&hit.result, hit.blocked))
                .collect(),
            push_separation: push_sep.as_ref().map(PushSeparation::from),
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
        let hits: Vec<HitResult> = self
            .last_hits
            .iter()
            .map(|hit| HitResult::from_runtime(&hit.result, hit.blocked))
            .collect();
        serde_wasm_bindgen::to_value(&hits)
            .map_err(|e| JsError::new(&format!("Serialization error: {:?}", e)))
    }

    /// Capture the deterministic session state for training frame step-back.
    pub fn snapshot(&self) -> Result<JsValue, JsError> {
        let snapshot = TrainingSnapshot {
            player: CharacterState::from(&self.player_state),
            dummy: CharacterState::from(&self.dummy_state),
            player_x: self.player_pos.0,
            player_y: self.player_pos.1,
            dummy_x: self.dummy_pos.0,
            dummy_y: self.dummy_pos.1,
        };

        serde_wasm_bindgen::to_value(&snapshot)
            .map_err(|e| JsError::new(&format!("Serialization error: {:?}", e)))
    }

    /// Restore a snapshot previously returned by snapshot().
    pub fn restore(&mut self, snapshot: JsValue) -> Result<(), JsError> {
        let snapshot: TrainingSnapshot = serde_wasm_bindgen::from_value(snapshot)
            .map_err(|e| JsError::new(&format!("Snapshot deserialization error: {:?}", e)))?;

        self.player_state = snapshot.player.to_runtime().map_err(|e| JsError::new(&e))?;
        self.dummy_state = snapshot.dummy.to_runtime().map_err(|e| JsError::new(&e))?;
        self.player_pos = (snapshot.player_x, snapshot.player_y);
        self.dummy_pos = (snapshot.dummy_x, snapshot.dummy_y);
        self.last_hits.clear();

        Ok(())
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

    /// Get a player character property by name.
    ///
    /// Returns the property value as f64 (converted from Q24.8 fixed-point),
    /// or None if the property doesn't exist or is not a numeric (Q24.8) type.
    /// Bool and string properties are not supported by this method.
    ///
    /// # Arguments
    /// * `name` - The property name (e.g., "health", "walk_speed")
    pub fn get_property(&self, name: &str) -> Option<f64> {
        let pack = PackView::parse(&self.player_pack_data).ok()?;
        let props = pack.character_props()?;

        for i in 0..props.len() {
            let prop = props.get(i)?;
            let (off, len) = prop.name();
            let prop_name = pack.string(off, len)?;
            if prop_name == name {
                // Only return numeric (Q24.8) properties
                if prop.value_type() == PROP_TYPE_Q24_8 {
                    return Some(from_q24_8(prop.as_q24_8()));
                }
                return None;
            }
        }
        None
    }

    /// Get a dummy character property by name.
    ///
    /// Returns the property value as f64 (converted from Q24.8 fixed-point),
    /// or None if the property doesn't exist or is not a numeric (Q24.8) type.
    /// Bool and string properties are not supported by this method.
    ///
    /// # Arguments
    /// * `name` - The property name (e.g., "health", "walk_speed")
    pub fn get_dummy_property(&self, name: &str) -> Option<f64> {
        let pack = PackView::parse(&self.dummy_pack_data).ok()?;
        let props = pack.character_props()?;

        for i in 0..props.len() {
            let prop = props.get(i)?;
            let (off, len) = prop.name();
            let prop_name = pack.string(off, len)?;
            if prop_name == name {
                // Only return numeric (Q24.8) properties
                if prop.value_type() == PROP_TYPE_Q24_8 {
                    return Some(from_q24_8(prop.as_q24_8()));
                }
                return None;
            }
        }
        None
    }
}

impl TrainingSession {
    fn dummy_is_blocking(behavior: DummyState) -> bool {
        matches!(
            behavior,
            DummyState::BlockStand | DummyState::BlockCrouch | DummyState::BlockAuto
        )
    }

    fn apply_dummy_behavior(state: &mut RtCharacterState, behavior: DummyState, pack: &PackView) {
        if let Some(target) = Self::compute_dummy_state(behavior, pack) {
            if state.current_state != target {
                state.current_state = target;
                state.frame = 0;
                state.instance_duration = 0;
                state.hit_confirmed = false;
                state.block_confirmed = false;
            }
        }
    }

    /// Compute what authored state the dummy should transition to based on its behavior.
    fn compute_dummy_state(behavior: DummyState, pack: &PackView) -> Option<u16> {
        match behavior {
            DummyState::Stand => Self::find_authored_state(
                pack,
                &["0_idle", "idle", "stand", "standing"],
                &["idle", "stand"],
                &[],
            ),
            DummyState::Crouch => Self::find_authored_state(
                pack,
                &["1_crouch", "crouch", "2_crouch"],
                &["crouch"],
                &[],
            )
            .or_else(|| Self::compute_dummy_state(DummyState::Stand, pack)),
            DummyState::Jump => Self::find_authored_state(
                pack,
                &["8_jump", "jump"],
                &["jump", "airborne", "aerial"],
                &["j."],
            )
            .or_else(|| Self::compute_dummy_state(DummyState::Stand, pack)),
            DummyState::BlockStand | DummyState::BlockAuto => Self::find_authored_state(
                pack,
                &["blockstun", "block_stun", "block_stand", "stand_block"],
                &["blockstun", "block", "guard"],
                &[],
            )
            .or_else(|| Self::compute_dummy_state(DummyState::Stand, pack)),
            DummyState::BlockCrouch => Self::find_authored_state(
                pack,
                &["block_crouch", "crouch_block", "blockstun", "block_stun"],
                &["blockstun", "block", "guard"],
                &[],
            )
            .or_else(|| Self::compute_dummy_state(DummyState::Crouch, pack)),
        }
    }

    fn enter_reaction_state(
        state: &mut RtCharacterState,
        pack: &PackView,
        inputs: &[&str],
        tags: &[&str],
        duration: u8,
    ) {
        if let Some(target) = Self::find_authored_state(pack, inputs, tags, &[]) {
            state.current_state = target;
            state.frame = 0;
            state.instance_duration = duration.max(1);
            state.hit_confirmed = false;
            state.block_confirmed = false;
        }
    }

    fn find_authored_state(
        pack: &PackView,
        inputs: &[&str],
        tags: &[&str],
        input_prefixes: &[&str],
    ) -> Option<u16> {
        Self::find_state_by_input(pack, inputs)
            .or_else(|| Self::find_state_by_tags(pack, tags))
            .or_else(|| Self::find_state_by_input_prefix(pack, input_prefixes))
    }

    fn find_state_by_input(pack: &PackView, inputs: &[&str]) -> Option<u16> {
        for input in inputs {
            if let Some((idx, _)) = pack.find_state_by_input(input) {
                if idx <= u16::MAX as usize {
                    return Some(idx as u16);
                }
            }
        }
        None
    }

    fn find_state_by_tags(pack: &PackView, tags: &[&str]) -> Option<u16> {
        if tags.is_empty() {
            return None;
        }

        let states = pack.states()?;
        for idx in 0..states.len().min(u16::MAX as usize + 1) {
            let Some(mut state_tags) = pack.state_tags(idx) else {
                continue;
            };
            if state_tags.any(|tag| tags.contains(&tag)) {
                return Some(idx as u16);
            }
        }
        None
    }

    fn find_state_by_input_prefix(pack: &PackView, prefixes: &[&str]) -> Option<u16> {
        if prefixes.is_empty() {
            return None;
        }

        let states = pack.states()?;
        let extras = pack.state_extras()?;
        for idx in 0..states.len().min(u16::MAX as usize + 1) {
            let extra = extras.get(idx)?;
            let (off, len) = extra.input();
            let Some(input) = pack.string(off, len) else {
                continue;
            };
            if prefixes.iter().any(|prefix| input.starts_with(prefix)) {
                return Some(idx as u16);
            }
        }
        None
    }

    /// Handle move completion - either loop system states or return to idle.
    fn handle_move_ended(state: &mut RtCharacterState, pack: &PackView) {
        if Self::is_looping_stance_state(pack, state.current_state) {
            state.frame = 0;
        } else {
            state.current_state =
                Self::compute_dummy_state(DummyState::Stand, pack).unwrap_or_default();
            state.frame = 0;
            state.instance_duration = 0;
            state.hit_confirmed = false;
            state.block_confirmed = false;
        }
    }

    fn is_looping_stance_state(pack: &PackView, state_idx: u16) -> bool {
        let Some(extras) = pack.state_extras() else {
            return state_idx <= 1;
        };
        let Some(extra) = extras.get(state_idx as usize) else {
            return false;
        };
        let (off, len) = extra.input();
        let Some(input) = pack.string(off, len) else {
            return false;
        };

        matches!(
            input,
            "0_idle" | "idle" | "stand" | "standing" | "1_crouch" | "crouch" | "2_crouch"
        )
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
            instance_duration: 12,
            hit_confirmed: true,
            block_confirmed: false,
            resources: [100, 50, 0, 0, 0, 0, 0, 0],
        };

        let js_state = CharacterState::from(&rt_state);

        assert_eq!(js_state.current_state, 5);
        assert_eq!(js_state.frame, 10);
        assert_eq!(js_state.instance_duration, 12);
        assert!(js_state.hit_confirmed);
        assert!(!js_state.block_confirmed);
        assert_eq!(js_state.resources.len(), 8);
        assert_eq!(js_state.resources[0], 100);
        assert_eq!(js_state.resources[1], 50);
    }

    #[test]
    fn character_state_restore_conversion_validates_bounds() {
        let js_state = CharacterState {
            current_state: 5,
            frame: 10,
            instance_duration: 12,
            hit_confirmed: true,
            block_confirmed: false,
            resources: vec![100, 50],
        };

        let rt_state = js_state.to_runtime().unwrap();

        assert_eq!(rt_state.current_state, 5);
        assert_eq!(rt_state.frame, 10);
        assert_eq!(rt_state.instance_duration, 12);
        assert!(rt_state.hit_confirmed);
        assert!(!rt_state.block_confirmed);
        assert_eq!(rt_state.resources[0], 100);
        assert_eq!(rt_state.resources[1], 50);
        assert_eq!(rt_state.resources[2], 0);
    }

    #[test]
    fn character_state_restore_conversion_rejects_invalid_values() {
        let invalid_state = CharacterState {
            current_state: u16::MAX as u32 + 1,
            frame: 10,
            instance_duration: 0,
            hit_confirmed: false,
            block_confirmed: false,
            resources: vec![],
        };

        assert!(invalid_state.to_runtime().is_err());

        let invalid_resources = CharacterState {
            current_state: 0,
            frame: 0,
            instance_duration: 0,
            hit_confirmed: false,
            block_confirmed: false,
            resources: vec![0; MAX_RESOURCES + 1],
        };

        assert!(invalid_resources.to_runtime().is_err());
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
        assert!(!js_hit.blocked);
        assert_eq!(js_hit.damage, 50);
        assert_eq!(js_hit.hitstun, 15);
        assert_eq!(js_hit.hit_pushback, 20);
    }

    #[test]
    fn hit_result_can_mark_blocked_contacts() {
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

        let js_hit = HitResult::from_runtime(&rt_hit, true);

        assert!(js_hit.blocked);
        assert_eq!(js_hit.damage, 50);
        assert_eq!(js_hit.chip_damage, 5);
        assert_eq!(js_hit.blockstun, 10);
    }

    fn test_char_pack() -> PackView<'static> {
        PackView::parse(include_bytes!("../../../exports/test_char.fspk"))
            .expect("test_char.fspk should parse")
    }

    #[test]
    fn target_training_fixture_resolves_authored_reaction_states() {
        let pack = test_char_pack();
        let (hitstun_idx, _) = pack
            .find_state_by_input("hitstun")
            .expect("target fixture should export a hitstun state");
        let (blockstun_idx, _) = pack
            .find_state_by_input("blockstun")
            .expect("target fixture should export a blockstun state");

        let hitstun_tags: Vec<_> = pack
            .state_tags(hitstun_idx)
            .expect("hitstun tags should decode")
            .collect();
        let blockstun_tags: Vec<_> = pack
            .state_tags(blockstun_idx)
            .expect("blockstun tags should decode")
            .collect();
        assert!(hitstun_tags.contains(&"hitstun"));
        assert!(blockstun_tags.contains(&"blockstun"));

        assert_eq!(
            TrainingSession::compute_dummy_state(DummyState::BlockStand, &pack),
            Some(blockstun_idx as u16)
        );
        assert_eq!(
            TrainingSession::compute_dummy_state(DummyState::BlockAuto, &pack),
            Some(blockstun_idx as u16)
        );

        let mut state = RtCharacterState::default();
        TrainingSession::enter_reaction_state(
            &mut state,
            &pack,
            &["hitstun", "hit_stun"],
            &["hitstun"],
            17,
        );
        assert_eq!(state.current_state, hitstun_idx as u16);
        assert_eq!(state.frame, 0);
        assert_eq!(state.instance_duration, 17);

        TrainingSession::enter_reaction_state(
            &mut state,
            &pack,
            &["blockstun", "block_stun", "guard_stun"],
            &["blockstun", "block", "guard"],
            11,
        );
        assert_eq!(state.current_state, blockstun_idx as u16);
        assert_eq!(state.frame, 0);
        assert_eq!(state.instance_duration, 11);
    }

    #[test]
    fn target_training_fixture_preserves_resource_and_throw_policies() {
        let pack = test_char_pack();

        let resources = pack
            .resource_defs()
            .expect("target fixture should export resource definitions");
        let resource_names: Vec<_> = (0..resources.len())
            .map(|idx| {
                let resource = resources.get(idx).expect("resource record");
                pack.string(resource.name_off(), resource.name_len())
                    .expect("resource name")
            })
            .collect();
        assert!(resource_names.contains(&"heat"));
        assert!(resource_names.contains(&"ammo"));
        assert!(resource_names.contains(&"level"));
        assert!(resource_names.contains(&"install_active"));

        let (_, throw_state) = pack
            .find_state_by_input("5T")
            .expect("target fixture should export ground throw input");
        assert_eq!(throw_state.state_type(), 5, "throw state type encoding");
        assert_eq!(throw_state.guard(), 3, "unblockable guard encoding");

        let (_, heavy_state) = pack
            .find_state_by_input("5H")
            .expect("target fixture should export 5H");
        let extras = pack
            .state_extras()
            .expect("target fixture should export state extras");
        let heavy_extras = extras
            .get(heavy_state.state_id() as usize)
            .expect("5H state extras");
        let (delta_off, delta_len) = heavy_extras.resource_deltas();
        assert!(
            delta_len >= 2,
            "5H meter gain should export resource deltas"
        );

        let deltas = pack
            .move_resource_deltas()
            .expect("target fixture should export move resource deltas");
        let mut has_whiff_meter = false;
        let mut has_hit_meter = false;
        for idx in 0..delta_len as usize {
            let delta = deltas
                .get_at(delta_off, idx)
                .expect("5H resource delta should decode");
            let name = pack
                .string(delta.name_off(), delta.name_len())
                .expect("delta resource name");
            if name == "meter"
                && delta.trigger() == framesmith_fspack::RESOURCE_DELTA_TRIGGER_ON_USE
            {
                has_whiff_meter = true;
            }
            if name == "meter"
                && delta.trigger() == framesmith_fspack::RESOURCE_DELTA_TRIGGER_ON_HIT
            {
                has_hit_meter = true;
            }
        }
        assert!(has_whiff_meter, "whiff meter gain should be exported");
        assert!(has_hit_meter, "hit meter gain should be exported");
    }
}
