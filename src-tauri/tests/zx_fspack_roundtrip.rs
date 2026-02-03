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
fn zx_fspack_export_roundtrips_through_reader() {
    let char_data = commands::load_character("../characters".to_string(), "test_char".to_string())
        .expect("load test_char character");

    let bytes = codegen::export_zx_fspack(&char_data).expect("export zx-fspack bytes");
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
fn zx_fspack_move_record_fields_match_reader_layout() {
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
            base: None,
            id: None,
        }],
        cancel_table: CancelTable::default(),
    };

    let bytes = codegen::export_zx_fspack(&char_data).expect("export zx-fspack bytes");
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
}

#[test]
fn zx_fspack_exports_resources_and_events_sections() {
    use framesmith_lib::commands::CharacterData;
    use framesmith_lib::schema::{
        CancelTable, CharacterResource, Cost, EventArgValue, EventEmit, GuardType, MeterGain,
        MoveNotify, OnHit, OnUse, Precondition, Pushback, ResourceDelta, State, TriggerType,
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
        notifies: vec![MoveNotify {
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

    let bytes = codegen::export_zx_fspack(&char_data).expect("export zx-fspack bytes");
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
fn zx_fspack_exports_move_input_notation() {
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

    let bytes = codegen::export_zx_fspack(&char_data).expect("export zx-fspack bytes");
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

    let bytes = codegen::export_zx_fspack(&char_data).expect("export");
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

    let bytes = codegen::export_zx_fspack(&char_data).expect("export");
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
    let bytes = codegen::export_zx_fspack(&char_data).expect("export");
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
    let bytes = codegen::export_zx_fspack(&char_data).expect("export");
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
