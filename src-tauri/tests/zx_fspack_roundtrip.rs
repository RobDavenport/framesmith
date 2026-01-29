use d_developmentnethercore_projectframesmith_lib::{codegen, commands};

#[test]
fn zx_fspack_export_roundtrips_through_reader() {
    let char_data = commands::load_character("../characters".to_string(), "glitch".to_string())
        .expect("load glitch character");

    let bytes = codegen::export_zx_fspack(&char_data).expect("export zx-fspack bytes");
    assert!(!bytes.is_empty(), "export should produce non-empty output");

    let pack = framesmith_fspack::PackView::parse(&bytes).expect("parse exported pack");

    assert!(
        pack.get_section(framesmith_fspack::SECTION_STRING_TABLE)
            .is_some(),
        "pack should contain a string table"
    );

    let moves = pack.moves().expect("pack should contain a moves section");
    assert_eq!(moves.len(), char_data.moves.len());

    let mesh_keys = pack.mesh_keys().expect("pack should contain mesh keys");
    let keyframes_keys = pack
        .keyframes_keys()
        .expect("pack should contain keyframes keys");

    assert!(mesh_keys.len() > 0, "expected at least one mesh key");
    assert!(
        keyframes_keys.len() > 0,
        "expected at least one keyframes key"
    );
}

#[test]
fn zx_fspack_move_record_fields_match_reader_layout() {
    use d_developmentnethercore_projectframesmith_lib::commands::CharacterData;
    use d_developmentnethercore_projectframesmith_lib::schema::{
        CancelTable, Character, FrameHitbox, GuardType, MeterGain, Move, MoveType, Pushback, Rect,
        TriggerType,
    };
    use std::collections::HashMap;

    let char_data = CharacterData {
        character: Character {
            id: "t".to_string(),
            name: "T".to_string(),
            archetype: "test".to_string(),
            health: 1000,
            walk_speed: 3.0,
            back_walk_speed: 3.0,
            jump_height: 100,
            jump_duration: 40,
            dash_distance: 80,
            dash_duration: 20,
        },
        moves: vec![Move {
            input: "5L".to_string(),
            name: "Test Jab".to_string(),
            tags: vec![],
            startup: 7,
            active: 3,
            recovery: 8,
            damage: 30,
            hitstun: 17,
            blockstun: 11,
            hitstop: 6,
            guard: GuardType::Mid,
            hitboxes: vec![FrameHitbox {
                frames: (7, 9),
                r#box: Rect {
                    x: 0,
                    y: -40,
                    w: 30,
                    h: 16,
                },
            }],
            hurtboxes: vec![FrameHitbox {
                frames: (0, 17),
                r#box: Rect {
                    x: -10,
                    y: -60,
                    w: 30,
                    h: 60,
                },
            }],
            pushback: Pushback { hit: 2, block: 2 },
            meter_gain: MeterGain { hit: 5, whiff: 2 },
            animation: "stand_light".to_string(),
            move_type: Some(MoveType::Normal),
            trigger: Some(TriggerType::Press),
            parent: None,
            total: None,
            hits: None,
            preconditions: None,
            costs: None,
            movement: None,
            super_freeze: None,
            on_use: None,
            on_hit: None,
            on_block: None,
            advanced_hurtboxes: None,
        }],
        cancel_table: CancelTable {
            chains: HashMap::new(),
            special_cancels: vec![],
            super_cancels: vec![],
            jump_cancels: vec![],
        },
    };

    let bytes = codegen::export_zx_fspack(&char_data).expect("export zx-fspack bytes");
    let pack = framesmith_fspack::PackView::parse(&bytes).expect("parse exported pack");

    let moves = pack.moves().expect("moves section");
    assert_eq!(moves.len(), 1);

    let mv = moves.get(0).expect("move 0");

    // Expected enum encodings for v1:
    // MoveType: normal=0, command_normal=1, special=2, super=3, movement=4, throw=5
    // Trigger: press=0, release=1, hold=2
    // Guard: high=0, mid=1, low=2, unblockable=3
    assert_eq!(mv.move_id(), 0);
    assert_eq!(mv.move_type(), 0);
    assert_eq!(mv.trigger(), 0);
    assert_eq!(mv.guard(), 1);
    assert_eq!(mv.flags(), 0);
    assert_eq!(mv.startup(), 7);
    assert_eq!(mv.active(), 3);
    assert_eq!(mv.recovery(), 8);
    assert_eq!(mv.total(), 18);
    assert_eq!(mv.damage(), 30);
    assert_eq!(mv.hitstun(), 17);
    assert_eq!(mv.blockstun(), 11);
    assert_eq!(mv.hitstop(), 6);

    // One animation => key index 0
    assert_eq!(mv.mesh_key(), 0);
    assert_eq!(mv.keyframes_key(), 0);

    assert_eq!(mv.hit_windows_off(), 0);
    assert_eq!(mv.hit_windows_len(), 1);
    assert_eq!(mv.hurt_windows_off(), 0);
    assert_eq!(mv.hurt_windows_len(), 1);
}
