//! Integration tests using real pack data.

use framesmith_runtime::*;

// This test requires a test fixture - skip for now if not available
#[test]
#[ignore = "requires test fixture"]
fn roundtrip_with_test_character() {
    // TODO: Load test_char.fspk fixture
    // let pack_data = include_bytes!("../fixtures/test_char.fspk");
    // let pack = PackView::parse(pack_data).unwrap();
    //
    // let mut state = CharacterState::default();
    // init_resources(&mut state, &pack);
    //
    // // Simulate a few frames
    // let input = FrameInput::default();
    // let result = next_frame(&state, &pack, &input);
    // assert!(!result.move_ended);
}

#[test]
fn state_is_deterministic() {
    let state = CharacterState {
        current_move: 5,
        frame: 10,
        hit_confirmed: true,
        block_confirmed: false,
        resources: [100, 50, 0, 0, 0, 0, 0, 0],
    };

    let copy1 = state;
    let copy2 = state;

    assert_eq!(copy1, copy2);
    assert_eq!(copy1.current_move, 5);
    assert_eq!(copy1.frame, 10);
    assert!(copy1.hit_confirmed);
}
