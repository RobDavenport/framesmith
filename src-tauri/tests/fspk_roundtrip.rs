use framesmith_lib::{codegen, commands};

/// Create a minimal test character for use in tests.
fn make_test_character(id: &str) -> framesmith_lib::schema::Character {
    use framesmith_lib::schema::{Character, PropertyValue};
    use std::collections::BTreeMap;

    let mut properties = BTreeMap::new();
    properties.insert("archetype".to_string(), PropertyValue::String("test".to_string()));
    properties.insert("health".to_string(), PropertyValue::Number(1000.0));
    properties.insert("walk_speed".to_string(), PropertyValue::Number(3.0));
    properties.insert("back_walk_speed".to_string(), PropertyValue::Number(3.0));
    properties.insert("jump_height".to_string(), PropertyValue::Number(100.0));
    properties.insert("jump_duration".to_string(), PropertyValue::Number(40.0));
    properties.insert("dash_distance".to_string(), PropertyValue::Number(80.0));
    properties.insert("dash_duration".to_string(), PropertyValue::Number(20.0));

    Character {
        id: id.to_string(),
        name: "T".to_string(),
        properties,
        resources: vec![],
    }
}

#[test]
fn fspk_export_roundtrips_through_reader() {
    let char_data = commands::load_character("../characters".to_string(), "test_char".to_string())
        .expect("load test_char character");

    let bytes = codegen::export_fspk(&char_data, None).expect("export zx-fspack bytes");
    assert!(!bytes.is_empty(), "export should produce non-empty output");

    let pack = framesmith_fspack::PackView::parse(&bytes).expect("parse exported pack");

    assert!(
        pack.get_section(framesmith_fspack::SECTION_STRING_TABLE)
            .is_some(),
        "pack should contain a string table"
    );

    let moves = pack.states().expect("pack should contain a moves section");
    assert_eq!(moves.len(), char_data.moves.len());

    let mesh_keys = pack.mesh_keys().expect("pack should contain mesh keys");
    let keyframes_keys = pack
        .keyframes_keys()
        .expect("pack should contain keyframes keys");

    assert!(!mesh_keys.is_empty(), "expected at least one mesh key");
    assert!(
        !keyframes_keys.is_empty(),
        "expected at least one keyframes key"
    );
}

#[test]
fn fspk_move_record_fields_match_reader_layout() {
    use framesmith_lib::commands::CharacterData;
    use framesmith_lib::schema::{
        CancelTable, FrameHitbox, GuardType, MeterGain, Pushback, Rect, State, TriggerType,
    };

    let char_data = CharacterData {
        character: make_test_character("t"),
        moves: vec![State {
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
            move_type: Some("normal".to_string()),
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
            notifies: vec![],
            advanced_hurtboxes: None,
            pushboxes: vec![],
            properties: std::collections::BTreeMap::new(),
            base: None,
            id: None,
        }],
        cancel_table: CancelTable::default(),
    };

    let bytes = codegen::export_fspk(&char_data, None).expect("export zx-fspack bytes");
    let pack = framesmith_fspack::PackView::parse(&bytes).expect("parse exported pack");

    let moves = pack.states().expect("moves section");
    assert_eq!(moves.len(), 1);

    let mv = moves.get(0).expect("move 0");

    // Expected enum encodings for v1:
    // MoveType: normal=0, command_normal=1, special=2, super=3, movement=4, throw=5
    // Trigger: press=0, release=1, hold=2
    // Guard: high=0, mid=1, low=2, unblockable=3
    assert_eq!(mv.state_id(), 0);
    assert_eq!(mv.state_type(), 0);
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

    // === Full chain test: move → hurt_windows → shapes ===
    // This verifies the binary layout matches between encoder and view.
    let hurt_windows = pack.hurt_windows().expect("HURT_WINDOWS section");
    let hw = hurt_windows
        .get_at(mv.hurt_windows_off(), 0)
        .expect("hurt window 0");
    assert_eq!(hw.start_frame(), 0, "hurt window start frame");
    assert_eq!(hw.end_frame(), 17, "hurt window end frame");
    assert_eq!(hw.shapes_len(), 1, "hurt window shapes_len");

    // Verify shapes_off points to valid shape data
    let shapes = pack.shapes().expect("SHAPES section");
    let shape = shapes
        .get_at(hw.shapes_off(), 0)
        .expect("shape from hurt window");
    // Original rect: x=-10, y=-60, w=30, h=60
    // Q12.4: x=-160, y=-960, w=480, h=960
    assert_eq!(shape.kind(), framesmith_fspack::SHAPE_KIND_AABB);
    assert_eq!(shape.a_raw(), -160, "shape x (Q12.4)");
    assert_eq!(shape.b_raw(), -960, "shape y (Q12.4)");
    assert_eq!(shape.c_raw(), 480, "shape w (Q12.4)");
    assert_eq!(shape.d_raw(), 960, "shape h (Q12.4)");

    // === Full chain test: move → hit_windows → shapes ===
    let hit_windows = pack.hit_windows().expect("HIT_WINDOWS section");
    let hitw = hit_windows
        .get_at(mv.hit_windows_off(), 0)
        .expect("hit window 0");
    assert_eq!(hitw.start_frame(), 7, "hit window start frame");
    assert_eq!(hitw.end_frame(), 9, "hit window end frame");
    assert_eq!(hitw.shapes_len(), 1, "hit window shapes_len");

    let hit_shape = shapes
        .get_at(hitw.shapes_off(), 0)
        .expect("shape from hit window");
    // Original rect: x=0, y=-40, w=30, h=16
    // Q12.4: x=0, y=-640, w=480, h=256
    assert_eq!(hit_shape.kind(), framesmith_fspack::SHAPE_KIND_AABB);
    assert_eq!(hit_shape.a_raw(), 0, "hit shape x (Q12.4)");
    assert_eq!(hit_shape.b_raw(), -640, "hit shape y (Q12.4)");
    assert_eq!(hit_shape.c_raw(), 480, "hit shape w (Q12.4)");
    assert_eq!(hit_shape.d_raw(), 256, "hit shape h (Q12.4)");
}

/// Verify the full chain: move → push_windows → shapes.
/// This catches encoder/view format mismatches in pushbox collision data.
#[test]
fn fspk_pushbox_chain_roundtrip() {
    use framesmith_lib::commands::CharacterData;
    use framesmith_lib::schema::{
        CancelTable, FrameHitbox, GuardType, MeterGain, Pushback, Rect, State,
    };

    let char_data = CharacterData {
        character: make_test_character("t"),
        moves: vec![State {
            input: "5L".to_string(),
            name: "Test".to_string(),
            tags: vec![],
            startup: 5,
            active: 3,
            recovery: 7,
            damage: 0,
            hitstun: 0,
            blockstun: 0,
            hitstop: 0,
            guard: GuardType::Mid,
            hitboxes: vec![],
            hurtboxes: vec![],
            pushboxes: vec![FrameHitbox {
                frames: (0, 14),
                r#box: Rect {
                    x: -20,
                    y: -80,
                    w: 40,
                    h: 80,
                },
            }],
            pushback: Pushback { hit: 0, block: 0 },
            meter_gain: MeterGain { hit: 0, whiff: 0 },
            animation: "test".to_string(),
            move_type: None,
            trigger: None,
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
            notifies: vec![],
            advanced_hurtboxes: None,
            properties: std::collections::BTreeMap::new(),
            base: None,
            id: None,
        }],
        cancel_table: CancelTable::default(),
    };

    let bytes = codegen::export_fspk(&char_data, None).expect("export");
    let pack = framesmith_fspack::PackView::parse(&bytes).expect("parse");

    let moves = pack.states().expect("moves section");
    let mv = moves.get(0).expect("move 0");

    // Verify move points to push windows
    assert_eq!(mv.push_windows_len(), 1, "push_windows_len");

    // Full chain: move → push_windows → shapes
    let push_windows = pack.push_windows().expect("PUSH_WINDOWS section");
    let pw = push_windows
        .get_at(mv.push_windows_off(), 0)
        .expect("push window 0");

    assert_eq!(pw.start_frame(), 0, "push window start frame");
    assert_eq!(pw.end_frame(), 14, "push window end frame");
    assert_eq!(pw.shapes_len(), 1, "push window shapes_len");

    // Verify shapes_off is valid and points to correct shape
    let shapes = pack.shapes().expect("SHAPES section");
    let shape = shapes
        .get_at(pw.shapes_off(), 0)
        .expect("shape from push window - this would fail if format mismatched");

    // Original rect: x=-20, y=-80, w=40, h=80
    // Q12.4: x=-320, y=-1280, w=640, h=1280
    assert_eq!(shape.kind(), framesmith_fspack::SHAPE_KIND_AABB);
    assert_eq!(shape.a_raw(), -320, "pushbox x (Q12.4)");
    assert_eq!(shape.b_raw(), -1280, "pushbox y (Q12.4)");
    assert_eq!(shape.c_raw(), 640, "pushbox w (Q12.4)");
    assert_eq!(shape.d_raw(), 1280, "pushbox h (Q12.4)");
}

#[test]
fn fspk_exports_resources_and_events_sections() {
    use framesmith_lib::commands::CharacterData;
    use framesmith_lib::schema::{
        CancelTable, CharacterResource, Cost, EventArgValue, EventEmit, GuardType, MeterGain,
        StateNotify, OnHit, OnUse, Precondition, Pushback, ResourceDelta, State, TriggerType,
    };
    use std::collections::BTreeMap;

    // Move 0: on_hit event emit with args
    let mut hit_args = BTreeMap::new();
    hit_args.insert(
        "strength".to_string(),
        EventArgValue::String("light".to_string()),
    );
    hit_args.insert("scale".to_string(), EventArgValue::F32(1.25));
    let mv0 = State {
        input: "5L".to_string(),
        name: "Hit event".to_string(),
        guard: GuardType::Mid,
        animation: "stand_light".to_string(),
        trigger: Some(TriggerType::Press),
        on_hit: Some(OnHit {
            events: vec![EventEmit {
                id: "vfx.hit_sparks".to_string(),
                args: hit_args,
            }],
            ..Default::default()
        }),
        ..Default::default()
    };

    // Move 1: notify timeline event
    let mut notify_args = BTreeMap::new();
    notify_args.insert(
        "bone".to_string(),
        EventArgValue::String("hand_r".to_string()),
    );
    let mv1 = State {
        input: "5M".to_string(),
        name: "Notify event".to_string(),
        guard: GuardType::Mid,
        animation: "stand_medium".to_string(),
        notifies: vec![StateNotify {
            frame: 7,
            events: vec![EventEmit {
                id: "vfx.swing_trail".to_string(),
                args: notify_args,
            }],
        }],
        ..Default::default()
    };

    // Move 2: resource cost + precondition + on_use resource delta
    let mv2 = State {
        input: "236P".to_string(),
        name: "Resource gated".to_string(),
        guard: GuardType::Mid,
        animation: "special".to_string(),
        costs: Some(vec![Cost::Resource {
            name: "heat".to_string(),
            amount: 1,
        }]),
        preconditions: Some(vec![Precondition::Resource {
            name: "heat".to_string(),
            min: Some(1),
            max: None,
        }]),
        on_use: Some(OnUse {
            resource_deltas: vec![ResourceDelta {
                name: "heat".to_string(),
                delta: -1,
            }],
            ..Default::default()
        }),
        ..Default::default()
    };

    let mut character = make_test_character("t");
    character.resources = vec![CharacterResource {
        name: "heat".to_string(),
        start: 0,
        max: 10,
    }];

    let char_data = CharacterData {
        character,
        moves: vec![
            State {
                pushback: Pushback { hit: 0, block: 0 },
                meter_gain: MeterGain { hit: 0, whiff: 0 },
                ..mv0
            },
            State {
                pushback: Pushback { hit: 0, block: 0 },
                meter_gain: MeterGain { hit: 0, whiff: 0 },
                ..mv1
            },
            State {
                pushback: Pushback { hit: 0, block: 0 },
                meter_gain: MeterGain { hit: 0, whiff: 0 },
                ..mv2
            },
        ],
        cancel_table: CancelTable::default(),
    };

    let bytes = codegen::export_fspk(&char_data, None).expect("export zx-fspack bytes");
    let pack = framesmith_fspack::PackView::parse(&bytes).expect("parse exported pack");

    // Resources section exists and decodes
    let resources = pack
        .resource_defs()
        .expect("expected RESOURCE_DEFS section");
    assert_eq!(resources.len(), 1);
    let res0 = resources.get(0).expect("resource 0");
    let name = pack
        .string(res0.name_off(), res0.name_len())
        .expect("resource name string");
    assert_eq!(name, "heat");
    assert_eq!(res0.start(), 0);
    assert_eq!(res0.max(), 10);

    // Per-move extras exist and point into backing arrays
    let extras = pack.state_extras().expect("expected MOVE_EXTRAS section");
    assert_eq!(extras.len(), 3);

    let emits = pack.event_emits().expect("expected EVENT_EMITS section");
    let args = pack.event_args().expect("expected EVENT_ARGS section");

    let idx_5l = pack
        .find_state_by_input("5L")
        .expect("state 5L should exist")
        .0;
    let idx_5m = pack
        .find_state_by_input("5M")
        .expect("state 5M should exist")
        .0;
    let idx_236p = pack
        .find_state_by_input("236P")
        .expect("state 236P should exist")
        .0;

    // 5L: on_hit emit -> id + args
    let ex_5l = extras.get(idx_5l).expect("extras 5L");
    let (on_hit_off, on_hit_len) = ex_5l.on_hit_emits();
    assert_eq!(on_hit_len, 1);
    let e0 = emits.get_at(on_hit_off, 0).expect("5L on_hit emit 0");
    let e0_id = pack
        .string(e0.id_off(), e0.id_len())
        .expect("emit id string");
    assert_eq!(e0_id, "vfx.hit_sparks");
    let (args_off, args_len) = e0.args();
    assert_eq!(args_len, 2);
    let a0 = args.get_at(args_off, 0).expect("arg 0");
    let a0_key = pack
        .string(a0.key_off(), a0.key_len())
        .expect("arg key string");
    assert!(a0_key == "scale" || a0_key == "strength");

    // 5M: notify event
    let notifies = pack
        .move_notifies()
        .expect("expected MOVE_NOTIFIES section");
    let ex_5m = extras.get(idx_5m).expect("extras 5M");
    let (notify_off, notify_len) = ex_5m.notifies();
    assert_eq!(notify_len, 1);
    let n0 = notifies.get_at(notify_off, 0).expect("notify 0");
    assert_eq!(n0.frame(), 7);
    let (n_emit_off, n_emit_len) = n0.emits();
    assert_eq!(n_emit_len, 1);
    let n_emit = emits.get_at(n_emit_off, 0).expect("notify emit 0");
    let n_id = pack
        .string(n_emit.id_off(), n_emit.id_len())
        .expect("notify id");
    assert_eq!(n_id, "vfx.swing_trail");

    // 236P: cost + precondition + on_use delta
    let costs = pack
        .move_resource_costs()
        .expect("expected MOVE_RESOURCE_COSTS section");
    let pre = pack
        .move_resource_preconditions()
        .expect("expected MOVE_RESOURCE_PRECONDITIONS section");
    let deltas = pack
        .move_resource_deltas()
        .expect("expected MOVE_RESOURCE_DELTAS section");

    let ex_236p = extras.get(idx_236p).expect("extras 236P");
    let (cost_off, cost_len) = ex_236p.resource_costs();
    assert_eq!(cost_len, 1);
    let c0 = costs.get_at(cost_off, 0).expect("cost 0");
    let c0_name = pack
        .string(c0.name_off(), c0.name_len())
        .expect("cost name");
    assert_eq!(c0_name, "heat");
    assert_eq!(c0.amount(), 1);

    let (pre_off, pre_len) = ex_236p.resource_preconditions();
    assert_eq!(pre_len, 1);
    let p0 = pre.get_at(pre_off, 0).expect("precondition 0");
    let p0_name = pack
        .string(p0.name_off(), p0.name_len())
        .expect("precondition name");
    assert_eq!(p0_name, "heat");
    assert_eq!(p0.min(), Some(1));
    assert_eq!(p0.max(), None);

    let (d_off, d_len) = ex_236p.resource_deltas();
    assert_eq!(d_len, 1);
    let d0 = deltas.get_at(d_off, 0).expect("delta 0");
    let d0_name = pack
        .string(d0.name_off(), d0.name_len())
        .expect("delta name");
    assert_eq!(d0_name, "heat");
    assert_eq!(d0.delta(), -1);
    assert_eq!(
        d0.trigger(),
        framesmith_fspack::RESOURCE_DELTA_TRIGGER_ON_USE
    );
}

#[test]
fn fspk_exports_move_input_notation() {
    use framesmith_lib::commands::CharacterData;
    use framesmith_lib::schema::{CancelTable, GuardType, MeterGain, Pushback, State};

    fn read_u32_le(bytes: &[u8], off: usize) -> u32 {
        u32::from_le_bytes([bytes[off], bytes[off + 1], bytes[off + 2], bytes[off + 3]])
    }

    fn read_u16_le(bytes: &[u8], off: usize) -> u16 {
        u16::from_le_bytes([bytes[off], bytes[off + 1]])
    }

    // Minimal character with a single move and no optional extras.
    // MOVE_EXTRAS should still be present because every move has an input.
    let char_data = CharacterData {
        character: make_test_character("t"),
        moves: vec![State {
            input: "5L".to_string(),
            name: "Test Jab".to_string(),
            guard: GuardType::Mid,
            animation: "stand_light".to_string(),
            pushback: Pushback { hit: 0, block: 0 },
            meter_gain: MeterGain { hit: 0, whiff: 0 },
            ..Default::default()
        }],
        cancel_table: CancelTable::default(),
    };

    let bytes = codegen::export_fspk(&char_data, None).expect("export zx-fspack bytes");
    let pack = framesmith_fspack::PackView::parse(&bytes).expect("parse exported pack");

    let extras_data = pack
        .get_section(framesmith_fspack::SECTION_STATE_EXTRAS)
        .expect("expected MOVE_EXTRAS section");
    assert_eq!(extras_data.len(), 72, "expected one 72-byte extras record");

    // The input notation string ref is stored at byte offset 56 within the record.
    let input_off = read_u32_le(extras_data, 56);
    let input_len = read_u16_le(extras_data, 60);

    let input = pack
        .string(input_off, input_len)
        .expect("input notation string");
    assert_eq!(input, "5L");
}

#[test]
fn tags_survive_roundtrip() {
    use framesmith_lib::commands::CharacterData;
    use framesmith_lib::schema::{
        CancelTable, GuardType, MeterGain, Pushback, State, Tag,
    };

    let char_data = CharacterData {
        character: make_test_character("t"),
        moves: vec![
            State {
                input: "5L".to_string(),
                name: "Light".to_string(),
                tags: vec![Tag::new("normal").unwrap(), Tag::new("light").unwrap()],
                guard: GuardType::Mid,
                animation: "5L".to_string(),
                pushback: Pushback { hit: 0, block: 0 },
                meter_gain: MeterGain { hit: 0, whiff: 0 },
                ..Default::default()
            },
            State {
                input: "5M".to_string(),
                name: "Medium".to_string(),
                tags: vec![Tag::new("normal").unwrap(), Tag::new("medium").unwrap()],
                guard: GuardType::Mid,
                animation: "5M".to_string(),
                pushback: Pushback { hit: 0, block: 0 },
                meter_gain: MeterGain { hit: 0, whiff: 0 },
                ..Default::default()
            },
        ],
        cancel_table: CancelTable::default(),
    };

    let bytes = codegen::export_fspk(&char_data, None).expect("export");
    let pack = framesmith_fspack::PackView::parse(&bytes).expect("parse");

    // Verify state tag sections exist
    assert!(
        pack.state_tag_ranges().is_some(),
        "STATE_TAG_RANGES section should exist"
    );

    // Move 0 should have tags ["normal", "light"]
    let tags0: Vec<&str> = pack.state_tags(0).expect("state 0 tags").collect();
    assert_eq!(tags0.len(), 2);
    assert_eq!(tags0[0], "normal");
    assert_eq!(tags0[1], "light");

    // Move 1 should have tags ["normal", "medium"]
    let tags1: Vec<&str> = pack.state_tags(1).expect("state 1 tags").collect();
    assert_eq!(tags1.len(), 2);
    assert_eq!(tags1[0], "normal");
    assert_eq!(tags1[1], "medium");
}

#[test]
fn empty_tags_roundtrip() {
    use framesmith_lib::commands::CharacterData;
    use framesmith_lib::schema::{CancelTable, GuardType, MeterGain, Pushback, State};

    let char_data = CharacterData {
        character: make_test_character("t"),
        moves: vec![State {
            input: "5L".to_string(),
            name: "Light".to_string(),
            tags: vec![], // No tags
            guard: GuardType::Mid,
            animation: "5L".to_string(),
            pushback: Pushback { hit: 0, block: 0 },
            meter_gain: MeterGain { hit: 0, whiff: 0 },
            ..Default::default()
        }],
        cancel_table: CancelTable::default(),
    };

    let bytes = codegen::export_fspk(&char_data, None).expect("export");
    let pack = framesmith_fspack::PackView::parse(&bytes).expect("parse");

    // Tag sections should not exist when no move has tags
    assert!(
        pack.state_tag_ranges().is_none(),
        "STATE_TAG_RANGES section should not exist when no tags"
    );
}

#[test]
fn cancel_tag_rules_roundtrip() {
    use framesmith_lib::commands::CharacterData;
    use framesmith_lib::schema::{
        CancelCondition, CancelTable, CancelTagRule, GuardType, MeterGain, Pushback,
        State, Tag,
    };

    // Create moves with tags
    let mv0 = State {
        input: "5L".to_string(),
        name: "Light".to_string(),
        tags: vec![Tag::new("normal").unwrap()],
        guard: GuardType::Mid,
        animation: "5L".to_string(),
        pushback: Pushback { hit: 0, block: 0 },
        meter_gain: MeterGain { hit: 0, whiff: 0 },
        ..Default::default()
    };
    let mv1 = State {
        input: "236P".to_string(),
        name: "Fireball".to_string(),
        tags: vec![Tag::new("special").unwrap()],
        guard: GuardType::Mid,
        animation: "236P".to_string(),
        pushback: Pushback { hit: 0, block: 0 },
        meter_gain: MeterGain { hit: 0, whiff: 0 },
        ..Default::default()
    };

    // Create cancel table with tag rule: normal can cancel to special on hit
    let cancel_table = CancelTable {
        tag_rules: vec![CancelTagRule {
            from: "normal".to_string(),
            to: "special".to_string(),
            on: CancelCondition::HIT,
            after_frame: 0,
            before_frame: 255,
        }],
        ..Default::default()
    };

    let char_data = CharacterData {
        character: make_test_character("t"),
        moves: vec![mv0, mv1],
        cancel_table,
    };

    // Export and parse
    let bytes = codegen::export_fspk(&char_data, None).expect("export");
    let pack = framesmith_fspack::PackView::parse(&bytes).expect("parse");

    // Verify cancel tag rules section exists
    let rules = pack
        .cancel_tag_rules()
        .expect("CANCEL_TAG_RULES section should exist");
    assert_eq!(rules.len(), 1);

    // Verify rule content
    let rule = rules.get(0).expect("rule 0");
    assert_eq!(rule.from_tag(), Some("normal"));
    assert_eq!(rule.to_tag(), Some("special"));
    assert_eq!(rule.condition(), 0b001); // HIT bit
    assert_eq!(rule.min_frame(), 0);
    assert_eq!(rule.max_frame(), 255);

    // Verify tags on moves (order-independent)
    let idx_5l = pack
        .find_state_by_input("5L")
        .expect("state 5L should exist")
        .0;
    let idx_236p = pack
        .find_state_by_input("236P")
        .expect("state 236P should exist")
        .0;

    let tags_l: Vec<&str> = pack.state_tags(idx_5l).expect("5L tags").collect();
    assert_eq!(tags_l, vec!["normal"]);
    let tags_236p: Vec<&str> = pack.state_tags(idx_236p).expect("236P tags").collect();
    assert_eq!(tags_236p, vec!["special"]);
}

#[test]
fn cancel_denies_roundtrip() {
    use framesmith_lib::commands::CharacterData;
    use framesmith_lib::schema::{CancelTable, GuardType, MeterGain, Pushback, State};

    // Create two moves
    let mv0 = State {
        input: "5L".to_string(),
        name: "Light".to_string(),
        guard: GuardType::Mid,
        animation: "5L".to_string(),
        pushback: Pushback { hit: 0, block: 0 },
        meter_gain: MeterGain { hit: 0, whiff: 0 },
        ..Default::default()
    };
    let mv1 = State {
        input: "jump".to_string(),
        name: "Jump".to_string(),
        guard: GuardType::Mid,
        animation: "jump".to_string(),
        pushback: Pushback { hit: 0, block: 0 },
        meter_gain: MeterGain { hit: 0, whiff: 0 },
        ..Default::default()
    };

    // Create cancel table with deny: 5L cannot cancel to jump
    let mut deny = std::collections::HashMap::new();
    deny.insert("5L".to_string(), vec!["jump".to_string()]);

    let cancel_table = CancelTable {
        deny,
        ..Default::default()
    };

    let char_data = CharacterData {
        character: make_test_character("t"),
        moves: vec![mv0, mv1],
        cancel_table,
    };

    // Export and parse
    let bytes = codegen::export_fspk(&char_data, None).expect("export");
    let pack = framesmith_fspack::PackView::parse(&bytes).expect("parse");

    // Verify deny exists: move 0 (5L) to move 1 (jump)
    assert!(
        pack.has_cancel_deny(0, 1),
        "5L (0) should be denied from canceling to jump (1)"
    );
    // Verify reverse is not denied
    assert!(
        !pack.has_cancel_deny(1, 0),
        "jump (1) should not be denied from canceling to 5L (0)"
    );
}

#[test]
fn test_cancel_condition_bitfield_roundtrip() {
    use framesmith_lib::schema::{CancelCondition, CancelTable, CancelTagRule, cancel_flags};

    // Test string shorthand
    let json = r#"{"from": "normal", "to": "special", "on": "hit"}"#;
    let rule: CancelTagRule = serde_json::from_str(json).unwrap();
    assert_eq!(rule.on.0, cancel_flags::HIT);

    // Test array format
    let json = r#"{"from": "normal", "to": "special", "on": ["hit", "block"]}"#;
    let rule: CancelTagRule = serde_json::from_str(json).unwrap();
    assert_eq!(rule.on.0, cancel_flags::HIT | cancel_flags::BLOCK);

    // Test "always" shorthand
    let json = r#"{"from": "any", "to": "any", "on": "always"}"#;
    let rule: CancelTagRule = serde_json::from_str(json).unwrap();
    assert_eq!(rule.on.0, cancel_flags::ALWAYS);

    // Test roundtrip serialization
    let table = CancelTable {
        tag_rules: vec![
            CancelTagRule {
                from: "normal".to_string(),
                to: "special".to_string(),
                on: CancelCondition(cancel_flags::HIT | cancel_flags::BLOCK), // hit + block
                after_frame: 0,
                before_frame: 255,
            },
        ],
        deny: Default::default(),
    };

    let json = serde_json::to_string(&table).unwrap();
    let parsed: CancelTable = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.tag_rules[0].on.0, cancel_flags::HIT | cancel_flags::BLOCK);
}

// =============================================================================
// State Properties Roundtrip Tests
// =============================================================================

/// Decoded property value from FSPK binary format.
#[derive(Debug, PartialEq)]
enum DecodedPropValue {
    Number(f64),   // Q24.8 converted back to f64
    Bool(bool),
    String(String),
}

/// Decode property records from raw bytes and string pool.
/// Returns a map of property names to decoded values.
fn decode_property_records(
    props_raw: &[u8],
    string_pool: &[u8],
) -> std::collections::BTreeMap<String, DecodedPropValue> {
    use std::collections::BTreeMap;

    const PROP_RECORD_SIZE: usize = 12;
    const PROP_TYPE_Q24_8: u8 = 0;
    const PROP_TYPE_BOOL: u8 = 1;
    const PROP_TYPE_STR: u8 = 2;

    let mut result = BTreeMap::new();

    for chunk in props_raw.chunks_exact(PROP_RECORD_SIZE) {
        // Parse record fields
        let name_off = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]) as usize;
        let name_len = u16::from_le_bytes([chunk[4], chunk[5]]) as usize;
        let value_type = chunk[6];
        // chunk[7] is reserved

        // Get property name from string pool
        let name = std::str::from_utf8(&string_pool[name_off..name_off + name_len])
            .expect("valid utf8 name")
            .to_string();

        // Decode value based on type
        let value = match value_type {
            PROP_TYPE_Q24_8 => {
                let q24_8 = i32::from_le_bytes([chunk[8], chunk[9], chunk[10], chunk[11]]);
                DecodedPropValue::Number(q24_8 as f64 / 256.0)
            }
            PROP_TYPE_BOOL => {
                let val = u32::from_le_bytes([chunk[8], chunk[9], chunk[10], chunk[11]]);
                DecodedPropValue::Bool(val != 0)
            }
            PROP_TYPE_STR => {
                let str_off = u16::from_le_bytes([chunk[8], chunk[9]]) as usize;
                let str_len = u16::from_le_bytes([chunk[10], chunk[11]]) as usize;
                let s = std::str::from_utf8(&string_pool[str_off..str_off + str_len])
                    .expect("valid utf8 string value")
                    .to_string();
                DecodedPropValue::String(s)
            }
            _ => panic!("unknown property type: {}", value_type),
        };

        result.insert(name, value);
    }

    result
}

#[test]
fn state_properties_scalar_survive_roundtrip() {
    use framesmith_lib::commands::CharacterData;
    use framesmith_lib::schema::{
        CancelTable, GuardType, MeterGain, PropertyValue, Pushback, State,
    };
    use std::collections::BTreeMap;

    let mut props = BTreeMap::new();
    props.insert("custom_startup".to_string(), PropertyValue::Number(5.0));
    props.insert("is_ex".to_string(), PropertyValue::Bool(true));
    props.insert("effect".to_string(), PropertyValue::String("spark".to_string()));

    let char_data = CharacterData {
        character: make_test_character("t"),
        moves: vec![State {
            input: "5L".to_string(),
            name: "Test".to_string(),
            guard: GuardType::Mid,
            animation: "test".to_string(),
            pushback: Pushback { hit: 0, block: 0 },
            meter_gain: MeterGain { hit: 0, whiff: 0 },
            properties: props.clone(),
            ..Default::default()
        }],
        cancel_table: CancelTable::default(),
    };

    let bytes = codegen::export_fspk(&char_data, None).expect("export");
    let pack = framesmith_fspack::PackView::parse(&bytes).expect("parse");

    // Verify STATE_PROPS section exists
    assert!(
        pack.get_section(framesmith_fspack::SECTION_STATE_PROPS).is_some(),
        "STATE_PROPS section should exist"
    );

    // Get raw props for state 0 and string pool
    let props_raw = pack.state_props_raw(0).expect("state 0 should have props");
    assert!(!props_raw.is_empty());

    let string_pool = pack.string_pool();
    let decoded = decode_property_records(props_raw, string_pool);

    assert_eq!(decoded.len(), 3);
    assert_eq!(decoded.get("custom_startup"), Some(&DecodedPropValue::Number(5.0)));
    assert_eq!(decoded.get("is_ex"), Some(&DecodedPropValue::Bool(true)));
    assert_eq!(
        decoded.get("effect"),
        Some(&DecodedPropValue::String("spark".to_string()))
    );
}

#[test]
fn state_properties_nested_flattened_on_export() {
    use framesmith_lib::commands::CharacterData;
    use framesmith_lib::schema::{
        CancelTable, GuardType, MeterGain, PropertyValue, Pushback, State,
    };
    use std::collections::BTreeMap;

    // Create nested Object - will be flattened to "movement.distance", "movement.direction"
    let mut movement = BTreeMap::new();
    movement.insert("distance".to_string(), PropertyValue::Number(80.0));
    movement.insert("direction".to_string(), PropertyValue::String("forward".to_string()));

    // Create nested Array - will be flattened to "effects.0", "effects.1", "effects.2"
    let effects = vec![
        PropertyValue::String("spark".to_string()),
        PropertyValue::Number(2.0),
        PropertyValue::Bool(true),
    ];

    let mut props = BTreeMap::new();
    props.insert("movement".to_string(), PropertyValue::Object(movement.clone()));
    props.insert("effects".to_string(), PropertyValue::Array(effects.clone()));

    let char_data = CharacterData {
        character: make_test_character("t"),
        moves: vec![State {
            input: "236P".to_string(),
            name: "Special".to_string(),
            guard: GuardType::Mid,
            animation: "special".to_string(),
            pushback: Pushback { hit: 0, block: 0 },
            meter_gain: MeterGain { hit: 0, whiff: 0 },
            properties: props,
            ..Default::default()
        }],
        cancel_table: CancelTable::default(),
    };

    let bytes = codegen::export_fspk(&char_data, None).expect("export");
    let pack = framesmith_fspack::PackView::parse(&bytes).expect("parse");

    let props_raw = pack.state_props_raw(0).expect("state 0 should have props");
    let string_pool = pack.string_pool();
    let decoded = decode_property_records(props_raw, string_pool);

    // Nested Object is flattened with dot notation
    assert_eq!(decoded.get("movement.distance"), Some(&DecodedPropValue::Number(80.0)));
    assert_eq!(
        decoded.get("movement.direction"),
        Some(&DecodedPropValue::String("forward".to_string()))
    );

    // Nested Array is flattened with index notation
    assert_eq!(decoded.get("effects.0"), Some(&DecodedPropValue::String("spark".to_string())));
    assert_eq!(decoded.get("effects.1"), Some(&DecodedPropValue::Number(2.0)));
    assert_eq!(decoded.get("effects.2"), Some(&DecodedPropValue::Bool(true)));

    // Total: 2 from movement + 3 from effects = 5 flattened properties
    assert_eq!(decoded.len(), 5);
}

#[test]
fn state_without_properties_has_no_props_raw() {
    use framesmith_lib::commands::CharacterData;
    use framesmith_lib::schema::{
        CancelTable, GuardType, MeterGain, Pushback, State,
    };

    // State with empty properties
    let char_data = CharacterData {
        character: make_test_character("t"),
        moves: vec![State {
            input: "5L".to_string(),
            name: "Test".to_string(),
            guard: GuardType::Mid,
            animation: "test".to_string(),
            pushback: Pushback { hit: 0, block: 0 },
            meter_gain: MeterGain { hit: 0, whiff: 0 },
            // properties defaults to empty BTreeMap
            ..Default::default()
        }],
        cancel_table: CancelTable::default(),
    };

    let bytes = codegen::export_fspk(&char_data, None).expect("export");
    let pack = framesmith_fspack::PackView::parse(&bytes).expect("parse");

    // STATE_PROPS section should not exist when no state has properties
    assert!(
        pack.get_section(framesmith_fspack::SECTION_STATE_PROPS).is_none(),
        "STATE_PROPS section should not exist when no states have properties"
    );

    // state_props_raw should return None
    assert!(pack.state_props_raw(0).is_none());
    assert!(!pack.has_state_props(0));
}

#[test]
fn mixed_states_with_and_without_properties() {
    use framesmith_lib::commands::CharacterData;
    use framesmith_lib::schema::{
        CancelTable, GuardType, MeterGain, PropertyValue, Pushback, State,
    };
    use std::collections::BTreeMap;

    let mut props = BTreeMap::new();
    props.insert("damage_bonus".to_string(), PropertyValue::Number(10.0));

    // Move 0: no properties (input sorts to "236P")
    // Move 1: has properties (input sorts to "5L")
    let mv_no_props = State {
        input: "236P".to_string(),
        name: "Special".to_string(),
        guard: GuardType::Mid,
        animation: "special".to_string(),
        pushback: Pushback { hit: 0, block: 0 },
        meter_gain: MeterGain { hit: 0, whiff: 0 },
        // Empty properties
        ..Default::default()
    };

    let mv_with_props = State {
        input: "5L".to_string(),
        name: "Light".to_string(),
        guard: GuardType::Mid,
        animation: "light".to_string(),
        pushback: Pushback { hit: 0, block: 0 },
        meter_gain: MeterGain { hit: 0, whiff: 0 },
        properties: props.clone(),
        ..Default::default()
    };

    let char_data = CharacterData {
        character: make_test_character("t"),
        moves: vec![mv_no_props, mv_with_props],
        cancel_table: CancelTable::default(),
    };

    let bytes = codegen::export_fspk(&char_data, None).expect("export");
    let pack = framesmith_fspack::PackView::parse(&bytes).expect("parse");

    // Moves are sorted by input: "236P" (idx 0), "5L" (idx 1)
    let idx_236p = pack.find_state_by_input("236P").expect("find 236P").0;
    let idx_5l = pack.find_state_by_input("5L").expect("find 5L").0;

    // 236P should have no props
    assert!(
        pack.state_props_raw(idx_236p).is_none(),
        "236P should have no properties"
    );
    assert!(!pack.has_state_props(idx_236p));

    // 5L should have props
    let props_raw = pack.state_props_raw(idx_5l).expect("5L should have props");
    let string_pool = pack.string_pool();
    let decoded = decode_property_records(props_raw, string_pool);

    assert_eq!(decoded.len(), 1);
    assert_eq!(
        decoded.get("damage_bonus"),
        Some(&DecodedPropValue::Number(10.0))
    );
}

// =============================================================================
// Schema-Based Property Export Tests
// =============================================================================

#[test]
fn schema_section_present_when_rules_have_property_schema() {
    use framesmith_lib::commands::CharacterData;
    use framesmith_lib::rules::{MergedRules, PropertySchema, RulesFile};
    use framesmith_lib::schema::{
        CancelTable, GuardType, MeterGain, PropertyValue, Pushback, State,
    };
    use std::collections::BTreeMap;

    // Create a move with properties
    let mut props = BTreeMap::new();
    props.insert("damage".to_string(), PropertyValue::Number(100.0));
    props.insert("is_super".to_string(), PropertyValue::Bool(true));

    let char_data = CharacterData {
        character: make_test_character("t"),
        moves: vec![State {
            input: "5L".to_string(),
            name: "Test".to_string(),
            guard: GuardType::Mid,
            animation: "test".to_string(),
            pushback: Pushback { hit: 0, block: 0 },
            meter_gain: MeterGain { hit: 0, whiff: 0 },
            properties: props,
            ..Default::default()
        }],
        cancel_table: CancelTable::default(),
    };

    // Create rules with property schema
    // Must include all character props from make_test_character()
    let rules = RulesFile {
        version: 1,
        registry: None,
        apply: vec![],
        validate: vec![],
        properties: Some(PropertySchema {
            character: vec![
                "archetype".to_string(),
                "back_walk_speed".to_string(),
                "dash_distance".to_string(),
                "dash_duration".to_string(),
                "health".to_string(),
                "jump_duration".to_string(),
                "jump_height".to_string(),
                "walk_speed".to_string(),
            ],
            state: vec!["damage".to_string(), "is_super".to_string()],
        }),
        tags: Some(vec!["normal".to_string(), "special".to_string()]),
    };
    let merged = MergedRules::merge(Some(&rules), None);

    // Export with schema
    let bytes = codegen::export_fspk(&char_data, Some(&merged)).expect("export");
    let pack = framesmith_fspack::PackView::parse(&bytes).expect("parse");

    // Verify SECTION_SCHEMA exists
    assert!(
        pack.get_section(framesmith_fspack::SECTION_SCHEMA).is_some(),
        "SECTION_SCHEMA should be present when rules have property schema"
    );

    // Verify schema can be read
    let schema = pack.schema().expect("schema should be parseable");
    assert_eq!(schema.char_prop_count(), 8);  // 8 character props
    assert_eq!(schema.state_prop_count(), 2); // 2 state props
    assert_eq!(schema.tag_count(), 2);

    // Verify a few property names are accessible (order matches schema definition)
    assert_eq!(schema.char_prop_name(0), Some("archetype"));
    assert_eq!(schema.char_prop_name(4), Some("health"));
    assert_eq!(schema.state_prop_name(0), Some("damage"));
    assert_eq!(schema.state_prop_name(1), Some("is_super"));
    assert_eq!(schema.tag_name(0), Some("normal"));
    assert_eq!(schema.tag_name(1), Some("special"));
}

#[test]
fn schema_section_absent_when_no_property_schema() {
    use framesmith_lib::commands::CharacterData;
    use framesmith_lib::schema::{CancelTable, GuardType, MeterGain, Pushback, State};

    let char_data = CharacterData {
        character: make_test_character("t"),
        moves: vec![State {
            input: "5L".to_string(),
            name: "Test".to_string(),
            guard: GuardType::Mid,
            animation: "test".to_string(),
            pushback: Pushback { hit: 0, block: 0 },
            meter_gain: MeterGain { hit: 0, whiff: 0 },
            ..Default::default()
        }],
        cancel_table: CancelTable::default(),
    };

    // Export without rules (no schema)
    let bytes = codegen::export_fspk(&char_data, None).expect("export");
    let pack = framesmith_fspack::PackView::parse(&bytes).expect("parse");

    // Verify SECTION_SCHEMA is absent
    assert!(
        pack.get_section(framesmith_fspack::SECTION_SCHEMA).is_none(),
        "SECTION_SCHEMA should be absent when no property schema"
    );
    assert!(pack.schema().is_none());
}

#[test]
fn schema_based_property_records_are_8_bytes() {
    use framesmith_lib::commands::CharacterData;
    use framesmith_lib::rules::{MergedRules, PropertySchema, RulesFile};
    use framesmith_lib::schema::{
        CancelTable, GuardType, MeterGain, PropertyValue, Pushback, State,
    };
    use std::collections::BTreeMap;

    // Create character with properties
    let mut char_props = BTreeMap::new();
    char_props.insert("health".to_string(), PropertyValue::Number(1000.0));
    char_props.insert("walkSpeed".to_string(), PropertyValue::Number(3.5));

    let mut character = make_test_character("t");
    character.properties = char_props;

    let char_data = CharacterData {
        character,
        moves: vec![State {
            input: "5L".to_string(),
            name: "Test".to_string(),
            guard: GuardType::Mid,
            animation: "test".to_string(),
            pushback: Pushback { hit: 0, block: 0 },
            meter_gain: MeterGain { hit: 0, whiff: 0 },
            ..Default::default()
        }],
        cancel_table: CancelTable::default(),
    };

    // Create rules with property schema
    let rules = RulesFile {
        version: 1,
        registry: None,
        apply: vec![],
        validate: vec![],
        properties: Some(PropertySchema {
            character: vec!["health".to_string(), "walkSpeed".to_string()],
            state: vec![],
        }),
        tags: None,
    };
    let merged = MergedRules::merge(Some(&rules), None);

    // Export with schema
    let bytes = codegen::export_fspk(&char_data, Some(&merged)).expect("export");
    let pack = framesmith_fspack::PackView::parse(&bytes).expect("parse");

    // Verify CHARACTER_PROPS section size matches 8-byte records
    // 2 properties * 8 bytes = 16 bytes
    let char_props_section = pack
        .get_section(framesmith_fspack::SECTION_CHARACTER_PROPS)
        .expect("CHARACTER_PROPS section");
    assert_eq!(
        char_props_section.len(),
        16,
        "With schema, CHARACTER_PROPS should use 8-byte records (2 props * 8 = 16)"
    );

    // Verify we can read schema-based properties
    let schema_props = pack.schema_character_props().expect("schema character props");
    assert_eq!(schema_props.len(), 2);

    // Get the schema for name lookups
    let schema = pack.schema().expect("schema");

    // Read the properties
    let prop0 = schema_props.get(0).expect("prop 0");
    let prop0_name = schema.char_prop_name(prop0.schema_id()).expect("prop 0 name");
    assert!(prop0_name == "health" || prop0_name == "walkSpeed");
}

#[test]
fn export_with_schema_rejects_unknown_property() {
    use framesmith_lib::commands::CharacterData;
    use framesmith_lib::rules::{MergedRules, PropertySchema, RulesFile};
    use framesmith_lib::schema::{
        CancelTable, GuardType, MeterGain, PropertyValue, Pushback, State,
    };
    use std::collections::BTreeMap;

    // Create a move with a property NOT in the schema
    let mut props = BTreeMap::new();
    props.insert("unknownProp".to_string(), PropertyValue::Number(42.0));

    let char_data = CharacterData {
        character: make_test_character("t"),
        moves: vec![State {
            input: "5L".to_string(),
            name: "Test".to_string(),
            guard: GuardType::Mid,
            animation: "test".to_string(),
            pushback: Pushback { hit: 0, block: 0 },
            meter_gain: MeterGain { hit: 0, whiff: 0 },
            properties: props,
            ..Default::default()
        }],
        cancel_table: CancelTable::default(),
    };

    // Create rules with property schema that doesn't include "unknownProp"
    let rules = RulesFile {
        version: 1,
        registry: None,
        apply: vec![],
        validate: vec![],
        properties: Some(PropertySchema {
            character: vec![],
            state: vec!["damage".to_string(), "startup".to_string()],
        }),
        tags: None,
    };
    let merged = MergedRules::merge(Some(&rules), None);

    // Export should fail with helpful error
    let result = codegen::export_fspk(&char_data, Some(&merged));
    assert!(result.is_err(), "Export should fail when property not in schema");
    let err = result.unwrap_err();
    assert!(
        err.contains("unknownProp") || err.contains("unknown"),
        "Error should mention the unknown property: {}",
        err
    );
}
