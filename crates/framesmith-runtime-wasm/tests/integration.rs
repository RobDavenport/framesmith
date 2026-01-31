//! Integration tests for framesmith-runtime-wasm.
//!
//! Note: Tests that require WASM bindings (TrainingSession) can only
//! run in a WASM environment. Use `wasm-pack test` for those.
//! Tests here focus on pure Rust types that don't need WASM.

use framesmith_runtime_wasm::{CharacterState, DummyState, HitResult};

/// Test that we can load the real glitch.fspk and parse it.
#[test]
fn can_parse_real_fspk() {
    use framesmith_fspack::PackView;

    // Load the real test file
    let fspk_data = include_bytes!("../../../exports/glitch.fspk");
    let pack = PackView::parse(fspk_data);

    assert!(pack.is_ok(), "Should parse glitch.fspk successfully");

    let pack = pack.unwrap();
    // Verify it has moves
    assert!(pack.states().is_some(), "Should have moves section");
    let moves = pack.states().unwrap();
    assert!(moves.len() > 0, "Should have at least one move");
}

/// Test runtime simulation with real FSPK data.
#[test]
fn simulate_with_real_fspk() {
    use framesmith_fspack::PackView;
    use framesmith_runtime::{
        init_resources, next_frame, CharacterState as RtState, FrameInput,
    };

    let fspk_data = include_bytes!("../../../exports/glitch.fspk");
    let pack = PackView::parse(fspk_data).unwrap();

    // Initialize state
    let mut state = RtState::default();
    init_resources(&mut state, &pack);

    // Simulate a few frames
    let input = FrameInput::default();
    for _ in 0..10 {
        let result = next_frame(&state, &pack, &input);
        state = result.state;
    }

    // State should have advanced
    assert!(state.frame > 0 || state.current_state > 0, "State should have progressed");
}

#[test]
fn character_state_conversion_roundtrip() {
    use framesmith_runtime::CharacterState as RtState;

    let rt_state = RtState {
        current_state: 5,
        frame: 10,
        instance_duration: 0,
        hit_confirmed: true,
        block_confirmed: false,
        resources: [100, 50, 25, 0, 0, 0, 0, 0],
    };

    let js_state = CharacterState::from(&rt_state);

    assert_eq!(js_state.current_state, 5);
    assert_eq!(js_state.frame, 10);
    assert!(js_state.hit_confirmed);
    assert!(!js_state.block_confirmed);
    assert_eq!(js_state.resources, vec![100, 50, 25, 0, 0, 0, 0, 0]);
}

#[test]
fn hit_result_conversion_roundtrip() {
    use framesmith_runtime::HitResult as RtHit;

    let rt_hit = RtHit {
        attacker_move: 3,
        window_index: 1,
        damage: 100,
        chip_damage: 10,
        hitstun: 20,
        blockstun: 15,
        hitstop: 8,
        guard: 2,
        hit_pushback: 30,
        block_pushback: 25,
    };

    let js_hit = HitResult::from(&rt_hit);

    assert_eq!(js_hit.attacker_move, 3);
    assert_eq!(js_hit.window_index, 1);
    assert_eq!(js_hit.damage, 100);
    assert_eq!(js_hit.chip_damage, 10);
    assert_eq!(js_hit.hitstun, 20);
    assert_eq!(js_hit.blockstun, 15);
    assert_eq!(js_hit.hitstop, 8);
    assert_eq!(js_hit.guard, 2);
    assert_eq!(js_hit.hit_pushback, 30);
    assert_eq!(js_hit.block_pushback, 25);
}

#[test]
fn dummy_state_default_is_stand() {
    assert_eq!(DummyState::default(), DummyState::Stand);
}

#[test]
fn dummy_state_variants() {
    // Verify all variants are distinct
    assert_ne!(DummyState::Stand, DummyState::Crouch);
    assert_ne!(DummyState::Crouch, DummyState::Jump);
    assert_ne!(DummyState::Jump, DummyState::BlockStand);
    assert_ne!(DummyState::BlockStand, DummyState::BlockCrouch);
    assert_ne!(DummyState::BlockCrouch, DummyState::BlockAuto);
}
