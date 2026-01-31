//! Integration tests using real pack data.

use framesmith_runtime::*;

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
