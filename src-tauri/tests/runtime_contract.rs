use framesmith_fspack::PackView;
use framesmith_lib::{codegen::export_fspk, commands::CharacterData, schema::*};
use framesmith_runtime::{
    apply_resource_costs, can_cancel_to, next_frame, CharacterState, FrameInput,
};

fn data() -> CharacterData {
    CharacterData {
        character: Character {
            id: "probe".into(),
            name: "Probe".into(),
            properties: Default::default(),
            resources: vec![CharacterResource {
                name: "charge".into(),
                start: 10,
                max: 100,
            }],
        },
        moves: vec![
            State {
                input: "a".into(),
                recovery: 10,
                ..Default::default()
            },
            State {
                input: "b".into(),
                recovery: 10,
                ..Default::default()
            },
        ],
        cancel_table: CancelTable {
            tag_rules: vec![CancelTagRule {
                from: "any".into(),
                to: "any".into(),
                on: CancelCondition::ALWAYS,
                after_frame: 0,
                before_frame: 255,
            }],
            ..Default::default()
        },
    }
}

#[test]
fn authored_property_numbers_survive_text_reload() {
    let expected = PropertyValue::Number(9_007_199_254_740_991.0);
    let text = serde_json::to_string(&expected).unwrap();
    let reloaded: PropertyValue = serde_json::from_str(&text).unwrap();
    assert_eq!(
        reloaded, expected,
        "saving and reopening must not change f64 data"
    );
    let mut original = data();
    original
        .character
        .properties
        .insert("precise".into(), expected);
    let authored = serde_json::to_string(&original.character).unwrap();
    let mut loaded = data();
    loaded.character = serde_json::from_str(&authored).unwrap();
    let bytes = export_fspk(&loaded, None).unwrap();
    let pack = PackView::parse(&bytes).unwrap();
    assert_eq!(
        pack.payload().unwrap().root().to_json(),
        serde_json::to_value(&original).unwrap()
    );
}

#[test]
fn aggregate_duration_does_not_wrap_at_256() {
    let mut data = data();
    data.moves[0].startup = 255;
    data.moves[0].recovery = 2;
    let bytes = export_fspk(&data, None).unwrap();
    let pack = PackView::parse(&bytes).unwrap();
    let mut state = CharacterState::default();
    for frame in 1..=257 {
        let result = next_frame(&state, &pack, &FrameInput::default());
        assert_eq!(result.move_ended, frame == 257, "frame {frame}");
        assert_eq!(u32::from(result.state.frame), frame);
        state = result.state;
    }
}

#[test]
fn transition_clears_duration_and_replays_identically() {
    let bytes = export_fspk(&data(), None).unwrap();
    let pack = PackView::parse(&bytes).unwrap();
    let before = CharacterState {
        instance_duration: 3,
        hit_confirmed: true,
        ..Default::default()
    };
    let input = FrameInput {
        requested_state: Some(1),
    };
    let result = next_frame(&before, &pack, &input);
    assert_eq!(result.state.current_state, 1);
    assert_eq!(result.state.instance_duration, 0);
    assert!(!result.state.hit_confirmed);
    assert_eq!(result.state, next_frame(&before, &pack, &input).state);
    assert_eq!(before.instance_duration, 3);
}

#[test]
fn invalid_action_cannot_become_a_state() {
    let bytes = export_fspk(&data(), None).unwrap();
    let pack = PackView::parse(&bytes).unwrap();
    let state = CharacterState::default();
    assert!(!can_cancel_to(&state, &pack, u16::MAX));
    let result = next_frame(
        &state,
        &pack,
        &FrameInput {
            requested_state: Some(u16::MAX),
        },
    );
    assert_eq!(result.state.current_state, 0);
    assert!(!can_cancel_to(
        &CharacterState {
            current_state: u16::MAX,
            ..state
        },
        &pack,
        0
    ));
}

#[test]
fn exact_input_selectors_and_combined_confirmations_work() {
    let mut data = data();
    data.cancel_table.tag_rules[0].from = "a".into();
    data.cancel_table.tag_rules[0].to = "b".into();
    data.cancel_table.tag_rules[0].on = CancelCondition::BLOCK;
    let bytes = export_fspk(&data, None).unwrap();
    let pack = PackView::parse(&bytes).unwrap();
    let state = CharacterState {
        hit_confirmed: true,
        block_confirmed: true,
        ..Default::default()
    };
    assert!(can_cancel_to(&state, &pack, 1));
}

#[test]
fn costs_are_atomic_and_gate_cancels() {
    let mut data = data();
    data.moves[1].costs = Some(vec![
        Cost::Resource {
            name: "charge".into(),
            amount: 6,
        },
        Cost::Resource {
            name: "charge".into(),
            amount: 6,
        },
    ]);
    let bytes = export_fspk(&data, None).unwrap();
    let pack = PackView::parse(&bytes).unwrap();
    let before = CharacterState {
        resources: [10, 0, 0, 0, 0, 0, 0, 0],
        ..Default::default()
    };
    let mut candidate = before;
    assert!(!apply_resource_costs(&mut candidate, &pack, 1));
    assert_eq!(candidate, before, "failed cost cannot consume resources");
    assert!(!can_cancel_to(&before, &pack, 1));
    let result = next_frame(
        &before,
        &pack,
        &FrameInput {
            requested_state: Some(1),
        },
    );
    assert_eq!(result.state.current_state, 0);
    assert_eq!(result.state.resources, before.resources);
    candidate.resources[0] = 12;
    assert!(can_cancel_to(&candidate, &pack, 1));
    assert!(apply_resource_costs(&mut candidate, &pack, 1));
    assert_eq!(candidate.resources[0], 0);
}

#[test]
fn unknown_resource_does_not_satisfy_a_requirement() {
    let mut data = data();
    data.moves[1].preconditions = Some(vec![Precondition::Resource {
        name: "missing".into(),
        min: Some(1),
        max: None,
    }]);
    let bytes = export_fspk(&data, None).unwrap();
    let pack = PackView::parse(&bytes).unwrap();
    assert!(!can_cancel_to(&CharacterState::default(), &pack, 1));
}

#[test]
fn malformed_pack_structure_is_rejected() {
    let bytes = export_fspk(&data(), None).unwrap();
    let mut corrupt = bytes.clone();
    corrupt[4..8].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(PackView::parse(&corrupt).is_err(), "unknown version/flags");
    let mut corrupt = bytes.clone();
    corrupt[20..24].copy_from_slice(&0u32.to_le_bytes());
    assert!(PackView::parse(&corrupt).is_err(), "section aliases header");
    let mut corrupt = bytes.clone();
    corrupt[32..36].copy_from_slice(&bytes[16..20]);
    assert!(PackView::parse(&corrupt).is_err(), "duplicate section");
    let mut corrupt = bytes.clone();
    corrupt[28..32].copy_from_slice(&3u32.to_le_bytes());
    assert!(PackView::parse(&corrupt).is_err(), "invalid alignment");
    assert!(PackView::parse(&bytes[..bytes.len() - 1]).is_err());
}

#[test]
fn shipped_characters_have_full_fidelity_binary_payloads() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../characters");
    let mut count = 0;
    for entry in std::fs::read_dir(&root).unwrap() {
        let path = entry.unwrap().path();
        if !path.join("character.json").is_file() {
            continue;
        }
        let id = path.file_name().unwrap().to_str().unwrap();
        let data =
            framesmith_lib::commands::load_character(root.to_string_lossy().into(), id.into())
                .unwrap();
        let bytes = export_fspk(&data, None).unwrap();
        let pack = PackView::parse(&bytes).unwrap();
        assert_eq!(pack.version(), 2);
        assert_eq!(
            pack.payload().unwrap().root().to_json(),
            serde_json::to_value(&data).unwrap(),
            "{id}"
        );
        for (index, state) in data.moves.iter().enumerate() {
            assert_eq!(
                pack.state_id(index),
                Some(state.id.as_deref().unwrap_or(&state.input))
            );
            assert!(pack.state_props_raw(data.moves.len()).is_none());
        }
        assert_eq!(
            bytes,
            export_fspk(&data, None).unwrap(),
            "deterministic {id}"
        );
        count += 1;
    }
    assert!(count > 0);
    println!("full-fidelity binary roundtrips: {count} shipped characters");
}

#[test]
fn variant_identity_nested_values_and_tag_only_schema_survive() {
    let mut data = data();
    data.moves[1].input = "a".into();
    data.moves[1].id = Some("a~charged".into());
    data.cancel_table.tag_rules[0].to = "a~charged".into();
    data.character.properties = serde_json::from_value(serde_json::json!({
        "health": 1000, "precise": 0.12345678901234567, "huge": i64::MAX,
        "empty": [], "a.b": 1, "a": {"b": 2, "flags": [true, "剣"]}
    }))
    .unwrap();
    let bytes = export_fspk(&data, None).unwrap();
    let pack = PackView::parse(&bytes).unwrap();
    assert_eq!(
        pack.payload().unwrap().root().to_json(),
        serde_json::to_value(&data).unwrap()
    );
    assert!(can_cancel_to(&CharacterState::default(), &pack, 1));
    data.moves.reverse();
    assert_eq!(bytes, export_fspk(&data, None).unwrap());

    let rules = framesmith_lib::rules::MergedRules {
        tags: Some(vec!["normal".into()]),
        ..Default::default()
    };
    let bytes = export_fspk(&data, Some(&rules)).unwrap();
    let pack = PackView::parse(&bytes).unwrap();
    assert!(
        pack.character_props().is_none(),
        "schema bytes are not 12-byte records"
    );
    assert!(!pack.schema_character_props().unwrap().is_empty());
    assert!(pack.state_props_raw(usize::MAX).is_none());
}

#[test]
fn malformed_nested_offsets_and_mutation_walks_are_bounded() {
    use framesmith_fspack::{SECTION_STATE_EXTRAS, SECTION_STATE_PROPS};
    let mut data = data();
    data.moves[0]
        .properties
        .insert("probe".into(), PropertyValue::Bool(true));
    let bytes = export_fspk(&data, None).unwrap();
    let pack = PackView::parse(&bytes).unwrap();
    let offset = |kind| pack.get_section(kind).unwrap().as_ptr() as usize - bytes.as_ptr() as usize;
    for (kind, field) in [(SECTION_STATE_EXTRAS, 56), (SECTION_STATE_PROPS, 0)] {
        let mut bad = bytes.clone();
        let off = offset(kind) + field;
        bad[off..off + 4].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(PackView::parse(&bad).is_err());
    }
    for end in 0..bytes.len() {
        assert!(PackView::parse(&bytes[..end]).is_err());
    }
    let mut seed = 0xF5_20_26u32;
    for _ in 0..2048 {
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        let mut bad = bytes.clone();
        let index = seed as usize % bad.len();
        bad[index] ^= 0xff;
        // Numeric/text mutations can remain valid; success must stay safe to consume.
        if let Ok(pack) = PackView::parse(&bad) {
            if let Some(payload) = pack.payload() {
                std::hint::black_box(payload.root().to_json());
            }
            std::hint::black_box(next_frame(
                &CharacterState::default(),
                &pack,
                &FrameInput::default(),
            ));
        }
    }
    let before = CharacterState {
        resources: [7; 8],
        ..Default::default()
    };
    data.character.resources = (0..9)
        .map(|i| CharacterResource {
            name: format!("r{i}"),
            start: 0,
            max: 10,
        })
        .collect();
    let bytes = export_fspk(&data, None).unwrap();
    let pack = PackView::parse(&bytes).unwrap();
    let mut state = before;
    assert!(!framesmith_runtime::init_resources(&mut state, &pack));
    assert_eq!(state, before);
}
