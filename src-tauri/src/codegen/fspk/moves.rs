//! Move packing and asset key generation.

use std::collections::HashMap;

use crate::codegen::fspk_format::KEY_NONE;
use crate::commands::CharacterData;
use crate::schema::State;

use super::packing::{guard_type_to_u8, pack_hit_window, pack_hurt_window, pack_move_record, pack_shape};
use super::types::{CancelLookup, PackedMoveData, StrRef, StringTable};
use super::utils::{checked_u16, checked_u32};

/// Pack all moves into binary sections.
///
/// Returns packed move data with all backing arrays.
///
/// The `anim_to_index` map provides indices into the MESH_KEYS/KEYFRAMES_KEYS arrays
/// for each animation name. If None, all moves use KEY_NONE for asset references.
///
/// The `cancel_lookup` provides cancel information for setting MoveRecord.flags.
/// If None, all flags are 0.
pub fn pack_moves(
    moves: &[State],
    anim_to_index: Option<&HashMap<String, u16>>,
    cancel_lookup: Option<&CancelLookup>,
) -> Result<PackedMoveData, String> {
    let mut packed = PackedMoveData {
        moves: Vec::new(),
        shapes: Vec::new(),
        hit_windows: Vec::new(),
        hurt_windows: Vec::new(),
        push_windows: Vec::new(),
    };

    for (idx, mv) in moves.iter().enumerate() {
        let move_id = checked_u16(idx, "move_id")?;

        // Look up animation index if map is provided
        let anim_index = anim_to_index
            .and_then(|map| {
                if mv.animation.is_empty() {
                    None
                } else {
                    map.get(&mv.animation).copied()
                }
            })
            .unwrap_or(KEY_NONE);

        // Track offsets before adding this move's data
        let hit_windows_off = checked_u32(packed.hit_windows.len(), "hit_windows_off")?;
        let hurt_windows_off = checked_u16(packed.hurt_windows.len(), "hurt_windows_off")?;
        let push_windows_off = checked_u16(packed.push_windows.len(), "push_windows_off")?;

        // Pack hitboxes -> shapes + hit_windows
        for hb in &mv.hitboxes {
            let shape_off = checked_u32(packed.shapes.len(), "shape_off")?;
            packed.shapes.extend_from_slice(&pack_shape(&hb.r#box));
            packed.hit_windows.extend_from_slice(&pack_hit_window(
                hb,
                shape_off,
                mv.damage,
                mv.hitstun,
                mv.blockstun,
                mv.hitstop,
                guard_type_to_u8(&mv.guard),
            ));
        }

        // Pack hurtboxes -> shapes + hurt_windows
        for hb in &mv.hurtboxes {
            let shape_off = checked_u32(packed.shapes.len(), "shape_off")?;
            packed.shapes.extend_from_slice(&pack_shape(&hb.r#box));
            packed.hurt_windows.extend_from_slice(&pack_hurt_window(hb, shape_off));
        }

        // Pack pushboxes -> shapes + push_windows (same 12-byte format as hurt windows)
        for pb in &mv.pushboxes {
            let shape_off = checked_u32(packed.shapes.len(), "shape_off")?;
            packed.shapes.extend_from_slice(&pack_shape(&pb.r#box));
            packed.push_windows.extend_from_slice(&pack_hurt_window(pb, shape_off));
        }

        // Calculate lengths
        let hit_windows_len = checked_u16(mv.hitboxes.len(), "hit_windows_len")?;
        let hurt_windows_len = checked_u16(mv.hurtboxes.len(), "hurt_windows_len")?;
        let push_windows_len = checked_u16(mv.pushboxes.len(), "push_windows_len")?;

        // Cancel flags are now handled via tag_rules, so MoveRecord.flags is always 0
        let flags: u8 = 0;
        let _ = cancel_lookup; // Silence unused warning; used later for deny resolution

        // Pack move record - mesh_key and keyframes_key both use the same animation index
        packed.moves.extend_from_slice(&pack_move_record(
            move_id,
            anim_index, // mesh_key
            anim_index, // keyframes_key
            mv,
            hit_windows_off,
            hit_windows_len,
            hurt_windows_off,
            hurt_windows_len,
            push_windows_off,
            push_windows_len,
            flags,
        ));
    }

    Ok(packed)
}

/// Build asset key arrays from character data.
///
/// Returns two vectors of string references:
/// - `mesh_keys`: Keys for mesh assets, format: "{character_id}.{animation}"
/// - `keyframes_keys`: Keys for keyframes assets, format: "{animation}"
///
/// Keys are sorted deterministically by their string value to ensure
/// reproducible output. Duplicate animations are deduplicated.
pub fn build_asset_keys(
    char_data: &CharacterData,
    strings: &mut StringTable,
) -> Result<(Vec<StrRef>, Vec<StrRef>), String> {
    // Collect unique animation names
    let mut animations: Vec<&str> = char_data
        .moves
        .iter()
        .filter(|m| !m.animation.is_empty())
        .map(|m| m.animation.as_str())
        .collect();

    // Deduplicate and sort for determinism
    animations.sort();
    animations.dedup();

    let character_id = &char_data.character.id;

    // Build mesh keys: "{character_id}.{animation}"
    let mesh_keys: Vec<StrRef> = animations
        .iter()
        .map(|anim| {
            let mesh_key = format!("{}.{}", character_id, anim);
            strings.intern(&mesh_key)
        })
        .collect::<Result<Vec<_>, _>>()?;

    // Build keyframes keys: just the animation name
    let keyframes_keys: Vec<StrRef> = animations
        .iter()
        .map(|anim| strings.intern(anim))
        .collect::<Result<Vec<_>, _>>()?;

    Ok((mesh_keys, keyframes_keys))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::fspk_format::STATE_RECORD_SIZE;
    use crate::schema::{CancelTable, Character, GuardType, MeterGain, Pushback, State};
    use std::collections::BTreeMap;

    fn make_test_character(id: &str) -> Character {
        use crate::schema::PropertyValue;
        let mut properties = BTreeMap::new();
        properties.insert("health".to_string(), PropertyValue::Number(1000.0));

        Character {
            id: id.to_string(),
            name: "Test Character".to_string(),
            properties,
            resources: vec![],
        }
    }

    fn make_test_move(input: &str, animation: &str) -> State {
        State {
            input: input.to_string(),
            name: format!("{} attack", input),
            tags: vec![],
            startup: 5,
            active: 3,
            recovery: 10,
            damage: 50,
            hitstun: 15,
            blockstun: 10,
            hitstop: 5,
            guard: GuardType::Mid,
            hitboxes: vec![],
            hurtboxes: vec![],
            pushback: Pushback { hit: 10, block: 5 },
            meter_gain: MeterGain { hit: 10, whiff: 5 },
            animation: animation.to_string(),
            ..Default::default()
        }
    }

    fn make_empty_cancel_table() -> CancelTable {
        CancelTable::default()
    }

    #[test]
    fn test_build_asset_keys_deterministic() {
        let char_data = CharacterData {
            character: make_test_character("test_char"),
            moves: vec![
                make_test_move("5H", "stand_heavy"),
                make_test_move("5L", "stand_light"),
                make_test_move("5M", "stand_medium"),
            ],
            cancel_table: make_empty_cancel_table(),
        };

        let mut strings1 = StringTable::new();
        let (mesh_keys1, kf_keys1) = build_asset_keys(&char_data, &mut strings1).unwrap();

        let char_data2 = CharacterData {
            character: make_test_character("test_char"),
            moves: vec![
                make_test_move("5M", "stand_medium"),
                make_test_move("5L", "stand_light"),
                make_test_move("5H", "stand_heavy"),
            ],
            cancel_table: make_empty_cancel_table(),
        };

        let mut strings2 = StringTable::new();
        let (mesh_keys2, kf_keys2) = build_asset_keys(&char_data2, &mut strings2).unwrap();

        assert_eq!(mesh_keys1.len(), mesh_keys2.len());
        assert_eq!(kf_keys1.len(), kf_keys2.len());
        assert_eq!(strings1.into_bytes(), strings2.into_bytes());
    }

    #[test]
    fn test_build_asset_keys_deduplication() {
        let char_data = CharacterData {
            character: make_test_character("test_char"),
            moves: vec![
                make_test_move("5L", "stand_light"),
                make_test_move("2L", "stand_light"), // Same animation
                make_test_move("5M", "stand_medium"),
            ],
            cancel_table: make_empty_cancel_table(),
        };

        let mut strings = StringTable::new();
        let (mesh_keys, kf_keys) = build_asset_keys(&char_data, &mut strings).unwrap();

        assert_eq!(mesh_keys.len(), 2, "Duplicate animations should be deduplicated");
        assert_eq!(kf_keys.len(), 2);
    }

    #[test]
    fn test_pack_moves_empty() {
        let packed = pack_moves(&[], None, None).unwrap();
        assert_eq!(packed.moves.len(), 0);
        assert_eq!(packed.shapes.len(), 0);
        assert_eq!(packed.hit_windows.len(), 0);
        assert_eq!(packed.hurt_windows.len(), 0);
    }

    #[test]
    fn test_pack_moves_count_matches() {
        let moves = vec![
            make_test_move("5L", "stand_light"),
            make_test_move("5M", "stand_medium"),
        ];
        let packed = pack_moves(&moves, None, None).unwrap();

        let move_count = packed.moves.len() / STATE_RECORD_SIZE;
        assert_eq!(move_count, 2);
    }
}
