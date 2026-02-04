//! Main FSPK export function.

use std::collections::HashMap;

use crate::codegen::fspk_format::{
    write_u16_le, write_u32_le, write_u8, FLAGS_RESERVED, HEADER_SIZE, MAGIC,
    SECTION_CANCEL_DENIES, SECTION_CANCEL_TAG_RULES, SECTION_CHARACTER_PROPS,
    SECTION_EVENT_ARGS, SECTION_EVENT_EMITS, SECTION_HEADER_SIZE, SECTION_HIT_WINDOWS,
    SECTION_HURT_WINDOWS, SECTION_KEYFRAMES_KEYS, SECTION_MESH_KEYS, SECTION_MOVE_NOTIFIES,
    SECTION_MOVE_RESOURCE_COSTS, SECTION_MOVE_RESOURCE_DELTAS,
    SECTION_MOVE_RESOURCE_PRECONDITIONS, SECTION_PUSH_WINDOWS, SECTION_RESOURCE_DEFS,
    SECTION_SHAPES, SECTION_STATES, SECTION_STATE_EXTRAS, SECTION_STATE_PROPS,
    SECTION_STATE_TAGS, SECTION_STATE_TAG_RANGES, SECTION_STRING_TABLE, STATE_EXTRAS72_SIZE,
    STRREF_SIZE,
};
use crate::commands::CharacterData;

use super::moves::{build_asset_keys, pack_moves};
use super::properties::{pack_character_props, pack_state_props};
use super::sections::{
    pack_event_args, OPT_U16_NONE, RESOURCE_DELTA_TRIGGER_ON_BLOCK,
    RESOURCE_DELTA_TRIGGER_ON_HIT, RESOURCE_DELTA_TRIGGER_ON_USE,
};
use super::types::{CancelLookup, StringTable};
use super::utils::{
    align_up, checked_u16, checked_u32, write_i32_le, write_range, write_section_header,
    write_strref,
};

/// Export character data to FSPK binary format.
///
/// Returns the packed binary data as a Vec<u8>.
#[allow(clippy::vec_init_then_push)] // Intentional: base sections first, optional sections conditionally added
pub fn export_fspk(char_data: &CharacterData) -> Result<Vec<u8>, String> {
    // Canonicalize move ordering so move indices are deterministic.
    // (Do this here as a backstop even if callers already sorted.)
    let mut char_data = char_data.clone();
    char_data.moves.sort_by(|a, b| a.input.cmp(&b.input));

    // Step 1: Build string table and asset keys
    let mut strings = StringTable::new();
    let (mesh_keys, keyframes_keys) = build_asset_keys(&char_data, &mut strings)?;

    // Build animation-to-index map for pack_moves
    // The keys are sorted alphabetically, so we can create the map from the sorted animations
    let mut animations: Vec<&str> = char_data
        .moves
        .iter()
        .filter(|m| !m.animation.is_empty())
        .map(|m| m.animation.as_str())
        .collect();
    animations.sort();
    animations.dedup();

    let mut anim_to_index: HashMap<String, u16> = HashMap::new();
    for (i, anim) in animations.iter().enumerate() {
        let idx = checked_u16(i, "animation index")?;
        anim_to_index.insert((*anim).to_string(), idx);
    }

    // Build input-to-index map for resolving chain targets
    let input_to_index: HashMap<&str, u16> = char_data
        .moves
        .iter()
        .enumerate()
        .map(|(i, m)| (m.input.as_str(), i as u16))
        .collect();

    // Build cancel lookup for resolving deny entries
    let cancel_lookup = CancelLookup { input_to_index };

    // Step 2: Pack moves with animation indices and cancel lookup
    let packed = pack_moves(&char_data.moves, Some(&anim_to_index), Some(&cancel_lookup))?;

    // Step 3: Pack optional sections (resources, events, notifies)
    let mut resource_defs_data: Vec<u8> = Vec::new();
    if !char_data.character.resources.is_empty() {
        for res in &char_data.character.resources {
            let name = strings.intern(&res.name)?;
            write_strref(&mut resource_defs_data, name);
            write_u16_le(&mut resource_defs_data, res.start);
            write_u16_le(&mut resource_defs_data, res.max);
        }
    }

    let mut event_emits_data: Vec<u8> = Vec::new();
    let mut event_args_data: Vec<u8> = Vec::new();
    let mut move_notifies_data: Vec<u8> = Vec::new();
    let mut move_resource_costs_data: Vec<u8> = Vec::new();
    let mut move_resource_preconditions_data: Vec<u8> = Vec::new();
    let mut move_resource_deltas_data: Vec<u8> = Vec::new();

    // MOVE_EXTRAS is always parallel to MOVES when present.
    // 9 fields: on_use_emits, on_hit_emits, on_block_emits, notifies, costs, pre, deltas, input, cancels
    let mut move_extras_records: Vec<[(u32, u16); 9]> = Vec::with_capacity(char_data.moves.len());

    let mut any_move_extras = false;

    for mv in &char_data.moves {
        let on_use_events = mv
            .on_use
            .as_ref()
            .map(|x| x.events.as_slice())
            .unwrap_or(&[]);
        let on_hit_events = mv
            .on_hit
            .as_ref()
            .map(|x| x.events.as_slice())
            .unwrap_or(&[]);
        let on_block_events = mv
            .on_block
            .as_ref()
            .map(|x| x.events.as_slice())
            .unwrap_or(&[]);

        let on_use_emits_off = checked_u32(event_emits_data.len(), "on_use_emits_off")?;
        let on_use_emits_len = checked_u16(on_use_events.len(), "on_use_emits_len")?;
        for emit in on_use_events {
            let (args_off, args_len) = pack_event_args(&emit.args, &mut event_args_data, &mut strings)?;

            let id = strings.intern(&emit.id)?;
            write_strref(&mut event_emits_data, id);
            write_range(&mut event_emits_data, args_off, args_len);
        }

        let on_hit_emits_off = checked_u32(event_emits_data.len(), "on_hit_emits_off")?;
        let on_hit_emits_len = checked_u16(on_hit_events.len(), "on_hit_emits_len")?;
        for emit in on_hit_events {
            let (args_off, args_len) = pack_event_args(&emit.args, &mut event_args_data, &mut strings)?;

            let id = strings.intern(&emit.id)?;
            write_strref(&mut event_emits_data, id);
            write_range(&mut event_emits_data, args_off, args_len);
        }

        let on_block_emits_off = checked_u32(event_emits_data.len(), "on_block_emits_off")?;
        let on_block_emits_len = checked_u16(on_block_events.len(), "on_block_emits_len")?;
        for emit in on_block_events {
            let (args_off, args_len) = pack_event_args(&emit.args, &mut event_args_data, &mut strings)?;

            let id = strings.intern(&emit.id)?;
            write_strref(&mut event_emits_data, id);
            write_range(&mut event_emits_data, args_off, args_len);
        }

        // Move notifies
        let notifies_off = checked_u32(move_notifies_data.len(), "notifies_off")?;
        let notifies_len = checked_u16(mv.notifies.len(), "notifies_len")?;
        for notify in &mv.notifies {
            let notify_emits_off = checked_u32(event_emits_data.len(), "notify_emits_off")?;
            let notify_emits_len = checked_u16(notify.events.len(), "notify_emits_len")?;

            for emit in &notify.events {
                let (args_off, args_len) = pack_event_args(&emit.args, &mut event_args_data, &mut strings)?;

                let id = strings.intern(&emit.id)?;
                write_strref(&mut event_emits_data, id);
                write_range(&mut event_emits_data, args_off, args_len);
            }

            // MoveNotify12: frame(u16) + pad(u16) + emits_off(u32) + emits_len(u16) + pad(u16)
            write_u16_le(&mut move_notifies_data, notify.frame);
            write_u16_le(&mut move_notifies_data, 0);
            write_range(&mut move_notifies_data, notify_emits_off, notify_emits_len);
        }

        // Move resource costs (Cost::Resource only)
        let costs_off = checked_u32(move_resource_costs_data.len(), "costs_off")?;
        let mut costs_len: u16 = 0;
        if let Some(costs) = &mv.costs {
            for cost in costs {
                if let crate::schema::Cost::Resource { name, amount } = cost {
                    let rname = strings.intern(name)?;
                    write_strref(&mut move_resource_costs_data, rname);
                    write_u16_le(&mut move_resource_costs_data, *amount);
                    write_u16_le(&mut move_resource_costs_data, 0);
                    costs_len = costs_len
                        .checked_add(1)
                        .ok_or_else(|| "move resource costs count overflows u16".to_string())?;
                }
            }
        }

        // Move resource preconditions (Precondition::Resource only)
        let pre_off = checked_u32(move_resource_preconditions_data.len(), "pre_off")?;
        let mut pre_len: u16 = 0;
        if let Some(preconditions) = &mv.preconditions {
            for pre in preconditions {
                if let crate::schema::Precondition::Resource { name, min, max } = pre {
                    let rname = strings.intern(name)?;
                    write_strref(&mut move_resource_preconditions_data, rname);
                    write_u16_le(
                        &mut move_resource_preconditions_data,
                        min.unwrap_or(OPT_U16_NONE),
                    );
                    write_u16_le(
                        &mut move_resource_preconditions_data,
                        max.unwrap_or(OPT_U16_NONE),
                    );
                    pre_len = pre_len.checked_add(1).ok_or_else(|| {
                        "move resource preconditions count overflows u16".to_string()
                    })?;
                }
            }
        }

        // Move resource deltas (on_use/on_hit/on_block)
        let deltas_off = checked_u32(move_resource_deltas_data.len(), "deltas_off")?;
        let mut deltas_len: u16 = 0;
        if let Some(on_use) = &mv.on_use {
            for d in &on_use.resource_deltas {
                let rname = strings.intern(&d.name)?;
                write_strref(&mut move_resource_deltas_data, rname);
                write_i32_le(&mut move_resource_deltas_data, d.delta);
                write_u8(
                    &mut move_resource_deltas_data,
                    RESOURCE_DELTA_TRIGGER_ON_USE,
                );
                move_resource_deltas_data.extend_from_slice(&[0, 0, 0]);
                deltas_len = deltas_len
                    .checked_add(1)
                    .ok_or_else(|| "move resource deltas count overflows u16".to_string())?;
            }
        }
        if let Some(on_hit) = &mv.on_hit {
            for d in &on_hit.resource_deltas {
                let rname = strings.intern(&d.name)?;
                write_strref(&mut move_resource_deltas_data, rname);
                write_i32_le(&mut move_resource_deltas_data, d.delta);
                write_u8(
                    &mut move_resource_deltas_data,
                    RESOURCE_DELTA_TRIGGER_ON_HIT,
                );
                move_resource_deltas_data.extend_from_slice(&[0, 0, 0]);
                deltas_len = deltas_len
                    .checked_add(1)
                    .ok_or_else(|| "move resource deltas count overflows u16".to_string())?;
            }
        }
        if let Some(on_block) = &mv.on_block {
            for d in &on_block.resource_deltas {
                let rname = strings.intern(&d.name)?;
                write_strref(&mut move_resource_deltas_data, rname);
                write_i32_le(&mut move_resource_deltas_data, d.delta);
                write_u8(
                    &mut move_resource_deltas_data,
                    RESOURCE_DELTA_TRIGGER_ON_BLOCK,
                );
                move_resource_deltas_data.extend_from_slice(&[0, 0, 0]);
                deltas_len = deltas_len
                    .checked_add(1)
                    .ok_or_else(|| "move resource deltas count overflows u16".to_string())?;
            }
        }

        // Intern the move input notation string.
        let input_ref = strings.intern(&mv.input)?;

        let record = [
            (on_use_emits_off, on_use_emits_len),
            (on_hit_emits_off, on_hit_emits_len),
            (on_block_emits_off, on_block_emits_len),
            (notifies_off, notifies_len),
            (costs_off, costs_len),
            (pre_off, pre_len),
            (deltas_off, deltas_len),
            (input_ref.0, input_ref.1),
            (0, 0), // cancels field (reserved, chains removed)
        ];

        // Always emit MOVE_EXTRAS when there are moves, since every move has an input.
        any_move_extras = true;
        move_extras_records.push(record);
    }

    // Step 4: Build section data
    // Mesh keys section: array of StrRef
    let mut mesh_keys_data = Vec::with_capacity(mesh_keys.len() * STRREF_SIZE);
    for strref in &mesh_keys {
        write_strref(&mut mesh_keys_data, *strref);
    }

    // Keyframes keys section: array of StrRef
    let mut keyframes_keys_data = Vec::with_capacity(keyframes_keys.len() * STRREF_SIZE);
    for strref in &keyframes_keys {
        write_strref(&mut keyframes_keys_data, *strref);
    }

    let mut move_extras_data: Vec<u8> = Vec::new();
    if any_move_extras {
        move_extras_data.reserve(move_extras_records.len() * STATE_EXTRAS72_SIZE);
        for rec in &move_extras_records {
            for (off, len) in rec {
                write_range(&mut move_extras_data, *off, *len);
            }
        }
    }

    // Omit backing sections if they have no data.
    if event_emits_data.is_empty() {
        // If no emits, args are unreachable; omit for cleanliness.
        event_args_data.clear();
    }

    // Build state tag sections (one range entry per move, tags are StrRefs)
    // Note: move_type (the "type" field) is also included as a tag so that
    // tag-based cancel rules can match on it (e.g., "system" -> "any")
    let mut state_tag_ranges_data: Vec<u8> = Vec::new();
    let mut state_tags_data: Vec<u8> = Vec::new();
    let any_tags = char_data
        .moves
        .iter()
        .any(|m| !m.tags.is_empty() || m.move_type.is_some());

    if any_tags {
        for mv in &char_data.moves {
            let tag_offset = checked_u32(state_tags_data.len(), "state_tags_offset")?;
            // Count includes move_type if present
            let type_count = if mv.move_type.is_some() { 1 } else { 0 };
            let tag_count = checked_u16(mv.tags.len() + type_count, "state_tags_count")?;

            // Write range entry: offset(4) + count(2) + padding(2)
            write_u32_le(&mut state_tag_ranges_data, tag_offset);
            write_u16_le(&mut state_tag_ranges_data, tag_count);
            write_u16_le(&mut state_tag_ranges_data, 0); // padding

            // Write move_type as first tag if present (so cancel rules can match on type)
            if let Some(ref move_type) = mv.move_type {
                let (str_off, str_len) = strings.intern(move_type.as_str())?;
                write_strref(&mut state_tags_data, (str_off, str_len));
            }

            // Write explicit tag StrRefs
            for tag in &mv.tags {
                let (str_off, str_len) = strings.intern(tag.as_str())?;
                write_strref(&mut state_tags_data, (str_off, str_len));
            }
        }
    }

    // Encode cancel tag rules
    // CancelTagRule24: from_tag StrRef (8) + to_tag StrRef (8) + condition (1) + min_frame (1) + max_frame (1) + flags (1) + padding (4) = 24
    let mut cancel_tag_rules_data: Vec<u8> = Vec::new();
    for rule in &char_data.cancel_table.tag_rules {
        // from_tag StrRef (8 bytes) - use 0xFFFFFFFF sentinel for "any"
        if rule.from == "any" {
            write_u32_le(&mut cancel_tag_rules_data, 0xFFFFFFFF);
            write_u16_le(&mut cancel_tag_rules_data, 0);
            write_u16_le(&mut cancel_tag_rules_data, 0);
        } else {
            let (off, len) = strings.intern(&rule.from)?;
            write_strref(&mut cancel_tag_rules_data, (off, len));
        }

        // to_tag StrRef (8 bytes) - use 0xFFFFFFFF sentinel for "any"
        if rule.to == "any" {
            write_u32_le(&mut cancel_tag_rules_data, 0xFFFFFFFF);
            write_u16_le(&mut cancel_tag_rules_data, 0);
            write_u16_le(&mut cancel_tag_rules_data, 0);
        } else {
            let (off, len) = strings.intern(&rule.to)?;
            write_strref(&mut cancel_tag_rules_data, (off, len));
        }

        // condition (1 byte) - now a bitfield
        let condition: u8 = rule.on.to_binary();
        write_u8(&mut cancel_tag_rules_data, condition);
        // min_frame (1 byte)
        write_u8(&mut cancel_tag_rules_data, rule.after_frame);
        // max_frame (1 byte)
        write_u8(&mut cancel_tag_rules_data, rule.before_frame);
        // flags (1 byte) - reserved
        write_u8(&mut cancel_tag_rules_data, 0);
        // padding (4 bytes)
        write_u32_le(&mut cancel_tag_rules_data, 0);
    }

    // Encode cancel denies
    // CancelDeny4: from_idx (u16) + to_idx (u16) = 4 bytes
    let mut cancel_denies_data: Vec<u8> = Vec::new();
    for (from_input, deny_list) in &char_data.cancel_table.deny {
        if let Some(&from_idx) = cancel_lookup.input_to_index.get(from_input.as_str()) {
            for to_input in deny_list {
                if let Some(&to_idx) = cancel_lookup.input_to_index.get(to_input.as_str()) {
                    write_u16_le(&mut cancel_denies_data, from_idx);
                    write_u16_le(&mut cancel_denies_data, to_idx);
                }
            }
        }
    }

    // Pack character properties (fixed 12-byte records)
    let character_props_data = pack_character_props(&char_data.character, &mut strings)?;

    // Pack per-state properties into STATE_PROPS section.
    // Format: Fixed 12-byte property records (same as CHARACTER_PROPS).
    // We build a parallel index: state_props_index[state_idx] = (offset, byte_len)
    // If a state has no properties, its entry is (0, 0).
    let mut state_props_data: Vec<u8> = Vec::new();
    let mut state_props_index: Vec<(u32, u16)> = Vec::with_capacity(char_data.moves.len());

    for mv in &char_data.moves {
        if mv.properties.is_empty() {
            state_props_index.push((0, 0));
        } else {
            let offset = checked_u32(state_props_data.len(), "state_props offset")?;
            let (props_blob, _count) = pack_state_props(&mv.properties, &mut strings)?;
            let byte_len = checked_u16(props_blob.len(), "state_props byte length")?;
            state_props_data.extend(props_blob);
            state_props_index.push((offset, byte_len));
        }
    }

    let string_table_data = strings.into_bytes();

    struct SectionData {
        kind: u32,
        align: u32,
        bytes: Vec<u8>,
    }

    let mut sections: Vec<SectionData> = Vec::new();

    // Base v1 sections (always present, same order)
    sections.push(SectionData {
        kind: SECTION_STRING_TABLE,
        align: 1,
        bytes: string_table_data,
    });
    sections.push(SectionData {
        kind: SECTION_MESH_KEYS,
        align: 4,
        bytes: mesh_keys_data,
    });
    sections.push(SectionData {
        kind: SECTION_KEYFRAMES_KEYS,
        align: 4,
        bytes: keyframes_keys_data,
    });
    sections.push(SectionData {
        kind: SECTION_STATES,
        align: 4,
        bytes: packed.moves,
    });
    sections.push(SectionData {
        kind: SECTION_HIT_WINDOWS,
        align: 4,
        bytes: packed.hit_windows,
    });
    sections.push(SectionData {
        kind: SECTION_HURT_WINDOWS,
        align: 4,
        bytes: packed.hurt_windows,
    });
    sections.push(SectionData {
        kind: SECTION_PUSH_WINDOWS,
        align: 4,
        bytes: packed.push_windows,
    });
    sections.push(SectionData {
        kind: SECTION_SHAPES,
        align: 4,
        bytes: packed.shapes,
    });

    // Optional sections (only present if data)
    if !resource_defs_data.is_empty() {
        sections.push(SectionData {
            kind: SECTION_RESOURCE_DEFS,
            align: 4,
            bytes: resource_defs_data,
        });
    }
    if any_move_extras {
        sections.push(SectionData {
            kind: SECTION_STATE_EXTRAS,
            align: 4,
            bytes: move_extras_data,
        });
    }
    if !event_emits_data.is_empty() {
        sections.push(SectionData {
            kind: SECTION_EVENT_EMITS,
            align: 4,
            bytes: event_emits_data,
        });
    }
    if !event_args_data.is_empty() {
        sections.push(SectionData {
            kind: SECTION_EVENT_ARGS,
            align: 4,
            bytes: event_args_data,
        });
    }
    if !move_notifies_data.is_empty() {
        sections.push(SectionData {
            kind: SECTION_MOVE_NOTIFIES,
            align: 4,
            bytes: move_notifies_data,
        });
    }
    if !move_resource_costs_data.is_empty() {
        sections.push(SectionData {
            kind: SECTION_MOVE_RESOURCE_COSTS,
            align: 4,
            bytes: move_resource_costs_data,
        });
    }
    if !move_resource_preconditions_data.is_empty() {
        sections.push(SectionData {
            kind: SECTION_MOVE_RESOURCE_PRECONDITIONS,
            align: 4,
            bytes: move_resource_preconditions_data,
        });
    }
    if !move_resource_deltas_data.is_empty() {
        sections.push(SectionData {
            kind: SECTION_MOVE_RESOURCE_DELTAS,
            align: 4,
            bytes: move_resource_deltas_data,
        });
    }
    if !state_tag_ranges_data.is_empty() {
        sections.push(SectionData {
            kind: SECTION_STATE_TAG_RANGES,
            align: 4,
            bytes: state_tag_ranges_data,
        });
        sections.push(SectionData {
            kind: SECTION_STATE_TAGS,
            align: 4,
            bytes: state_tags_data,
        });
    }
    if !cancel_tag_rules_data.is_empty() {
        sections.push(SectionData {
            kind: SECTION_CANCEL_TAG_RULES,
            align: 4,
            bytes: cancel_tag_rules_data,
        });
    }
    if !cancel_denies_data.is_empty() {
        sections.push(SectionData {
            kind: SECTION_CANCEL_DENIES,
            align: 4,
            bytes: cancel_denies_data,
        });
    }
    if !character_props_data.is_empty() {
        sections.push(SectionData {
            kind: SECTION_CHARACTER_PROPS,
            align: 4,
            bytes: character_props_data,
        });
    }
    if !state_props_data.is_empty() {
        // STATE_PROPS section contains fixed 12-byte property records for each state.
        // The state_props_index tracks (offset, byte_len) for each state in parallel to STATES.
        // States without properties have (0, 0) entries.
        // Nested properties are flattened at export time using dot notation.
        //
        // Format:
        // - STATE_PROPS_INDEX: Array of (offset: u32, byte_len: u16, pad: u16) = 8 bytes per state
        // - STATE_PROPS_DATA: Concatenated 12-byte property records
        //
        // We pack both into a single section, with index first:
        let mut state_props_section: Vec<u8> =
            Vec::with_capacity(state_props_index.len() * 8 + state_props_data.len());

        // Write index entries (8 bytes each: offset u32 + len u16 + padding u16)
        for (off, len) in &state_props_index {
            write_u32_le(&mut state_props_section, *off);
            write_u16_le(&mut state_props_section, *len);
            write_u16_le(&mut state_props_section, 0); // padding
        }

        // Adjust offsets to account for index size
        let index_size = state_props_index.len() * 8;
        for chunk in state_props_section.chunks_mut(8) {
            if chunk.len() >= 6 {
                let old_off = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                let len_val = u16::from_le_bytes([chunk[4], chunk[5]]);
                // Only adjust non-empty entries
                if len_val > 0 {
                    let new_off = old_off
                        .checked_add(index_size as u32)
                        .ok_or_else(|| "state_props offset overflow".to_string())?;
                    chunk[0..4].copy_from_slice(&new_off.to_le_bytes());
                }
            }
        }

        // Append the actual props data
        state_props_section.extend(state_props_data);

        sections.push(SectionData {
            kind: SECTION_STATE_PROPS,
            align: 4,
            bytes: state_props_section,
        });
    }

    if sections.len() > 24 {
        return Err(format!(
            "Too many sections ({}), MAX_SECTIONS is 24",
            sections.len()
        ));
    }

    // Step 5: Calculate section offsets (honor per-section alignment)
    let section_count = checked_u32(sections.len(), "section_count")?;
    let header_and_sections_size = HEADER_SIZE + (sections.len() * SECTION_HEADER_SIZE);
    let mut current_offset: usize = header_and_sections_size;

    #[derive(Clone, Copy)]
    struct SectionHeader {
        kind: u32,
        off: u32,
        len: u32,
        align: u32,
    }

    let mut section_headers: Vec<SectionHeader> = Vec::with_capacity(sections.len());
    for s in &sections {
        current_offset = align_up(current_offset, s.align)?;
        let off = checked_u32(current_offset, "section offset")?;
        let len = checked_u32(s.bytes.len(), "section length")?;
        section_headers.push(SectionHeader {
            kind: s.kind,
            off,
            len,
            align: s.align,
        });
        current_offset = current_offset
            .checked_add(s.bytes.len())
            .ok_or_else(|| "section offset overflow".to_string())?;
    }

    let total_len = checked_u32(current_offset, "total_len")?;

    // Step 6: Build the final binary
    let mut output = Vec::with_capacity(current_offset);
    output.extend_from_slice(&MAGIC);
    write_u32_le(&mut output, FLAGS_RESERVED);
    write_u32_le(&mut output, total_len);
    write_u32_le(&mut output, section_count);

    for h in &section_headers {
        write_section_header(&mut output, h.kind, h.off, h.len, h.align);
    }

    for (i, s) in sections.into_iter().enumerate() {
        let h = section_headers[i];
        let target = h.off as usize;
        if output.len() > target {
            return Err(format!(
                "section {} starts before current output (off={} len={})",
                i, h.off, h.len
            ));
        }
        output.resize(target, 0);
        output.extend_from_slice(&s.bytes);
    }

    debug_assert_eq!(
        output.len(),
        total_len as usize,
        "Output size mismatch: expected {}, got {}",
        total_len,
        output.len()
    );

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::fspk_format::{HURT_WINDOW12_SIZE, SECTION_STATE_EXTRAS};
    use crate::schema::{
        CancelTable, Character, CharacterResource, FrameHitbox, GuardType, MeterGain, Pushback,
        Rect, State,
    };

    fn read_u32_le(bytes: &[u8], off: usize) -> u32 {
        u32::from_le_bytes([bytes[off], bytes[off + 1], bytes[off + 2], bytes[off + 3]])
    }

    /// Create a minimal test character.
    fn make_test_character(id: &str) -> Character {
        use crate::schema::PropertyValue;
        use std::collections::BTreeMap;

        let mut properties = BTreeMap::new();
        properties.insert("archetype".to_string(), PropertyValue::String("rushdown".to_string()));
        properties.insert("health".to_string(), PropertyValue::Number(1000.0));
        properties.insert("walk_speed".to_string(), PropertyValue::Number(3.5));
        properties.insert("back_walk_speed".to_string(), PropertyValue::Number(2.5));
        properties.insert("jump_height".to_string(), PropertyValue::Number(120.0));
        properties.insert("jump_duration".to_string(), PropertyValue::Number(40.0));
        properties.insert("dash_distance".to_string(), PropertyValue::Number(80.0));
        properties.insert("dash_duration".to_string(), PropertyValue::Number(20.0));

        Character {
            id: id.to_string(),
            name: "Test Character".to_string(),
            properties,
            resources: vec![],
        }
    }

    /// Create a minimal test state with the given input and animation.
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
            pushboxes: vec![],
            properties: std::collections::BTreeMap::new(),
            base: None,
            id: None,
        }
    }

    /// Create an empty cancel table.
    fn make_empty_cancel_table() -> CancelTable {
        CancelTable::default()
    }

    fn make_move_with_hitboxes() -> State {
        State {
            input: "5L".to_string(),
            name: "Light Punch".to_string(),
            tags: vec![],
            startup: 5,
            active: 3,
            recovery: 10,
            damage: 500,
            hitstun: 12,
            blockstun: 8,
            hitstop: 10,
            guard: GuardType::Mid,
            hitboxes: vec![FrameHitbox {
                frames: (5, 8),
                r#box: Rect {
                    x: 10,
                    y: 20,
                    w: 50,
                    h: 60,
                },
            }],
            hurtboxes: vec![FrameHitbox {
                frames: (1, 18),
                r#box: Rect {
                    x: -20,
                    y: 0,
                    w: 40,
                    h: 80,
                },
            }],
            pushback: Pushback { hit: 10, block: 5 },
            meter_gain: MeterGain { hit: 10, whiff: 5 },
            animation: "stand_light".to_string(),
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
            pushboxes: vec![],
            properties: std::collections::BTreeMap::new(),
            base: None,
            id: None,
        }
    }

    #[test]
    fn test_export_fspk_magic_and_section_count() {
        // Create minimal character data
        let char_data = CharacterData {
            character: make_test_character("test"),
            moves: vec![make_test_move("5L", "stand_light")],
            cancel_table: make_empty_cancel_table(),
        };

        let result = export_fspk(&char_data);
        assert!(result.is_ok(), "export_fspk should succeed");

        let bytes = result.unwrap();

        // Verify magic bytes "FSPK"
        assert!(bytes.len() >= 16, "Output should have at least header size");
        assert_eq!(&bytes[0..4], b"FSPK", "Magic should be FSPK");

        // Verify flags
        let flags = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        assert_eq!(flags, 0, "Flags should be 0");

        // Verify total length matches actual output
        let total_len = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
        assert_eq!(
            total_len as usize,
            bytes.len(),
            "Total length should match actual output size"
        );

        // Verify section count: 8 base + STATE_EXTRAS + CHARACTER_PROPS = 10
        let section_count = u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]);
        assert_eq!(section_count, 10, "Section count should be 10");
    }

    #[test]
    fn test_export_fspk_empty_character() {
        // Create character data with no moves
        let char_data = CharacterData {
            character: make_test_character("empty"),
            moves: vec![],
            cancel_table: make_empty_cancel_table(),
        };

        let result = export_fspk(&char_data);
        assert!(
            result.is_ok(),
            "export_fspk should succeed with no moves"
        );

        let bytes = result.unwrap();

        // Should still have valid FSPK header
        assert_eq!(&bytes[0..4], b"FSPK");

        // Section count should be 8 base + CHARACTER_PROPS = 9 (no moves = no STATE_EXTRAS)
        let section_count = u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]);
        assert_eq!(section_count, 9);
    }

    #[test]
    fn test_export_fspk_is_deterministic_for_shuffled_moves() {
        let move_a = make_test_move("5M", "stand_medium");
        let move_b = make_test_move("5L", "stand_light");
        let move_c = make_test_move("236P", "fireball");

        let char_data1 = CharacterData {
            character: make_test_character("test"),
            moves: vec![move_a.clone(), move_b.clone(), move_c.clone()],
            cancel_table: make_empty_cancel_table(),
        };

        let char_data2 = CharacterData {
            character: make_test_character("test"),
            moves: vec![move_c, move_a, move_b],
            cancel_table: make_empty_cancel_table(),
        };

        let bytes1 = export_fspk(&char_data1).unwrap();
        let bytes2 = export_fspk(&char_data2).unwrap();
        assert_eq!(
            bytes1, bytes2,
            "Export should be deterministic regardless of move order"
        );
    }

    #[test]
    fn test_export_fspk_section_headers() {
        let char_data = CharacterData {
            character: make_test_character("test"),
            moves: vec![make_test_move("5L", "stand_light")],
            cancel_table: make_empty_cancel_table(),
        };

        let bytes = export_fspk(&char_data).unwrap();

        let section_count = read_u32_le(&bytes, 12) as usize;
        let header_end = HEADER_SIZE + (section_count * SECTION_HEADER_SIZE);
        assert!(
            bytes.len() >= header_end,
            "Output should have room for all section headers"
        );

        // Check that section kinds are correct (base v1 sections in order)
        // STRING_TABLE(1), MESH_KEYS(2), KEYFRAMES_KEYS(3), STATES(4), HIT_WINDOWS(5),
        // HURT_WINDOWS(6), PUSH_WINDOWS(22), SHAPES(7)
        // (CANCELS_U16 removed - chains are deprecated)
        let expected_base_kinds = [1, 2, 3, 4, 5, 6, 22, 7];
        for (i, &expected_kind) in expected_base_kinds.iter().enumerate() {
            let offset = HEADER_SIZE + i * SECTION_HEADER_SIZE;
            let kind = u32::from_le_bytes([
                bytes[offset],
                bytes[offset + 1],
                bytes[offset + 2],
                bytes[offset + 3],
            ]);
            assert_eq!(
                kind, expected_kind,
                "Section {} should have kind {}",
                i, expected_kind
            );
        }

        // MOVE_EXTRAS and CHARACTER_PROPS are expected when there are moves.
        // 8 base + STATE_EXTRAS + CHARACTER_PROPS = 10
        assert_eq!(
            section_count, 10,
            "Expected STATE_EXTRAS and CHARACTER_PROPS sections to be present"
        );
        let extras_kind_off = HEADER_SIZE + 8 * SECTION_HEADER_SIZE;
        let extras_kind = u32::from_le_bytes([
            bytes[extras_kind_off],
            bytes[extras_kind_off + 1],
            bytes[extras_kind_off + 2],
            bytes[extras_kind_off + 3],
        ]);
        assert_eq!(extras_kind, SECTION_STATE_EXTRAS);
    }

    #[test]
    fn test_export_fspk_section_offsets_aligned_and_non_overlapping() {
        let char_data = CharacterData {
            character: make_test_character("test"),
            moves: vec![make_test_move("5L", "stand_light")],
            cancel_table: make_empty_cancel_table(),
        };

        let bytes = export_fspk(&char_data).unwrap();
        let section_count = read_u32_le(&bytes, 12) as usize;
        let header_end = HEADER_SIZE + section_count * SECTION_HEADER_SIZE;

        let mut prev_end = header_end as u32;
        for i in 0..section_count {
            let base = HEADER_SIZE + i * SECTION_HEADER_SIZE;
            let off = read_u32_le(&bytes, base + 4);
            let len = read_u32_le(&bytes, base + 8);
            let align = read_u32_le(&bytes, base + 12);

            assert!(align != 0, "section {} has zero alignment", i);
            assert_eq!(
                off % align,
                0,
                "section {} offset {} must be aligned to {}",
                i,
                off,
                align
            );
            assert!(
                off >= prev_end,
                "section {} overlaps previous: off={} prev_end={}",
                i,
                off,
                prev_end
            );

            prev_end = off + len;
        }
    }

    #[test]
    fn test_export_fspk_rejects_string_len_overflow() {
        let mut character = make_test_character("test");
        character.resources = vec![CharacterResource {
            name: "a".repeat(u16::MAX as usize + 1),
            start: 0,
            max: 1,
        }];

        let char_data = CharacterData {
            character,
            moves: vec![make_test_move("5L", "stand_light")],
            cancel_table: make_empty_cancel_table(),
        };

        let result = export_fspk(&char_data);
        assert!(result.is_err(), "expected overflow to return Err");
    }

    #[test]
    fn test_export_fspk_rejects_hurt_windows_off_overflow() {
        fn hb() -> FrameHitbox {
            FrameHitbox {
                frames: (0, 0),
                r#box: Rect {
                    x: 0,
                    y: 0,
                    w: 1,
                    h: 1,
                },
            }
        }

        // Ensure the second move's hurt_windows_off (u16 byte offset) overflows.
        let hurtbox_count = (u16::MAX as usize / HURT_WINDOW12_SIZE) + 1;
        let hurtboxes: Vec<FrameHitbox> = (0..hurtbox_count).map(|_| hb()).collect();

        let mut mv1 = make_test_move("5L", "stand_light");
        mv1.hurtboxes = hurtboxes;
        let mv2 = make_test_move("5M", "stand_medium");

        let char_data = CharacterData {
            character: make_test_character("test"),
            moves: vec![mv1, mv2],
            cancel_table: make_empty_cancel_table(),
        };

        let result = export_fspk(&char_data);
        assert!(result.is_err(), "expected overflow to return Err");
    }

    #[test]
    fn test_export_fspk_with_animation_keys() {
        // Create character with multiple moves sharing some animations
        let char_data = CharacterData {
            character: make_test_character("test"),
            moves: vec![
                make_test_move("5L", "stand_light"),
                make_test_move("5M", "stand_medium"),
                make_test_move("2L", "stand_light"), // Shares animation with 5L
            ],
            cancel_table: make_empty_cancel_table(),
        };

        let bytes = export_fspk(&char_data).unwrap();

        // Verify magic
        assert_eq!(&bytes[0..4], b"FSPK");

        // The string table should contain "test.stand_light", "stand_light",
        // "test.stand_medium", "stand_medium" (deduplicated, sorted)
        // Just verify the export succeeded and has reasonable size
        assert!(bytes.len() > HEADER_SIZE + 8 * SECTION_HEADER_SIZE);
    }

    // ==========================================================================
    // Roundtrip Tests (Export + Parse via framesmith-fspack reader)
    // ==========================================================================

    /// Roundtrip test: export character data and parse it back with framesmith_fspack.
    ///
    /// This test verifies that:
    /// 1. Exported bytes can be successfully parsed by the reader crate
    /// 2. Move count matches
    /// 3. Keyframes keys exist when moves have animations
    /// 4. String table can be resolved
    #[test]
    fn test_roundtrip_export_and_parse() {
        // Create a character with multiple moves and animations
        let char_data = CharacterData {
            character: make_test_character("test_char"),
            moves: vec![
                make_test_move("5L", "stand_light"),
                make_test_move("5M", "stand_medium"),
                make_test_move("5H", "stand_heavy"),
            ],
            cancel_table: make_empty_cancel_table(),
        };

        // Export to bytes
        let bytes = export_fspk(&char_data).expect("export should succeed");

        // Parse with framesmith_fspack reader
        let pack = framesmith_fspack::PackView::parse(&bytes).expect("parse should succeed");

        // 8 base + STATE_EXTRAS + CHARACTER_PROPS = 10 sections
        assert_eq!(pack.section_count(), 10);

        // Verify move count matches
        let moves = pack.states().expect("should have MOVES section");
        assert_eq!(moves.len(), 3, "move count should match");

        // Verify keyframes keys exist (since all moves have animations)
        let kf_keys = pack
            .keyframes_keys()
            .expect("should have KEYFRAMES_KEYS section");
        assert_eq!(
            kf_keys.len(),
            3,
            "should have 3 unique keyframes keys (stand_heavy, stand_light, stand_medium - sorted)"
        );

        // Verify mesh keys exist
        let mesh_keys = pack.mesh_keys().expect("should have MESH_KEYS section");
        assert_eq!(mesh_keys.len(), 3, "should have 3 mesh keys");

        // Verify we can resolve a string from the string table
        // First mesh key should be "test_char.stand_heavy" (sorted alphabetically)
        let (off, len) = mesh_keys.get(0).expect("should get mesh key 0");
        let mesh_key_str = pack
            .string(off, len)
            .expect("should resolve mesh key string");
        assert_eq!(mesh_key_str, "test_char.stand_heavy");

        // First keyframes key should be "stand_heavy" (sorted alphabetically)
        let (kf_off, kf_len) = kf_keys.get(0).expect("should get keyframes key 0");
        let kf_key_str = pack
            .string(kf_off, kf_len)
            .expect("should resolve keyframes key string");
        assert_eq!(kf_key_str, "stand_heavy");
    }

    /// Roundtrip test with a character with hitboxes and hurtboxes.
    #[test]
    fn test_roundtrip_with_hitboxes() {
        let char_data = CharacterData {
            character: make_test_character("test"),
            moves: vec![make_move_with_hitboxes()],
            cancel_table: make_empty_cancel_table(),
        };

        // Export to bytes
        let bytes = export_fspk(&char_data).expect("export should succeed");

        // Parse with framesmith_fspack reader
        let pack = framesmith_fspack::PackView::parse(&bytes).expect("parse should succeed");

        // Verify move count
        let moves = pack.states().expect("should have MOVES section");
        assert_eq!(moves.len(), 1);

        // Get the move via the typed view
        let mv = moves.get(0).expect("should get move 0");
        assert_eq!(mv.state_id(), 0);

        // Verify the move has a valid mesh/keyframes key (not KEY_NONE since it has an animation)
        // The move "make_move_with_hitboxes" has animation "stand_light"
        assert_ne!(
            mv.mesh_key(),
            framesmith_fspack::KEY_NONE,
            "mesh_key should not be KEY_NONE"
        );
        assert_ne!(
            mv.keyframes_key(),
            framesmith_fspack::KEY_NONE,
            "keyframes_key should not be KEY_NONE"
        );

        // Verify HIT_WINDOWS section exists and has data
        let hit_windows = pack
            .get_section(framesmith_fspack::SECTION_HIT_WINDOWS)
            .expect("should have HIT_WINDOWS section");
        assert!(
            !hit_windows.is_empty(),
            "HIT_WINDOWS section should have data"
        );

        // Verify HURT_WINDOWS section exists and has data
        let hurt_windows = pack
            .get_section(framesmith_fspack::SECTION_HURT_WINDOWS)
            .expect("should have HURT_WINDOWS section");
        assert!(
            !hurt_windows.is_empty(),
            "HURT_WINDOWS section should have data"
        );

        // Verify SHAPES section exists and has data
        let shapes = pack
            .get_section(framesmith_fspack::SECTION_SHAPES)
            .expect("should have SHAPES section");
        assert!(!shapes.is_empty(), "SHAPES section should have data");
    }

    /// Roundtrip test with empty character (no moves).
    #[test]
    fn test_roundtrip_empty_character() {
        let char_data = CharacterData {
            character: make_test_character("empty"),
            moves: vec![],
            cancel_table: make_empty_cancel_table(),
        };

        // Export to bytes
        let bytes = export_fspk(&char_data).expect("export should succeed");

        // Parse with framesmith_fspack reader
        let pack = framesmith_fspack::PackView::parse(&bytes).expect("parse should succeed");

        // 8 base + CHARACTER_PROPS = 9 (no moves = no STATE_EXTRAS)
        assert_eq!(pack.section_count(), 9);

        // Verify moves section is empty
        let moves = pack.states().expect("should have MOVES section");
        assert_eq!(moves.len(), 0);
        assert!(moves.is_empty());

        // Verify keyframes keys section is empty
        let kf_keys = pack
            .keyframes_keys()
            .expect("should have KEYFRAMES_KEYS section");
        assert!(kf_keys.is_empty());
    }

    /// Roundtrip test verifying animation deduplication.
    #[test]
    fn test_roundtrip_animation_deduplication() {
        // Create character with moves that share animations
        let char_data = CharacterData {
            character: make_test_character("test"),
            moves: vec![
                make_test_move("5L", "stand_light"),
                make_test_move("2L", "stand_light"), // Same animation as 5L
                make_test_move("5M", "stand_medium"),
                make_test_move("2M", "stand_medium"), // Same animation as 5M
            ],
            cancel_table: make_empty_cancel_table(),
        };

        // Export to bytes
        let bytes = export_fspk(&char_data).expect("export should succeed");

        // Parse with framesmith_fspack reader
        let pack = framesmith_fspack::PackView::parse(&bytes).expect("parse should succeed");

        // Verify we have 4 moves
        let moves = pack.states().expect("should have MOVES section");
        assert_eq!(moves.len(), 4);

        // But only 2 unique animations (stand_light, stand_medium)
        let kf_keys = pack
            .keyframes_keys()
            .expect("should have KEYFRAMES_KEYS section");
        assert_eq!(kf_keys.len(), 2, "should have only 2 unique keyframes keys");

        // Verify both keyframes keys can be resolved
        for i in 0..kf_keys.len() {
            let (off, len) = kf_keys.get(i).expect("should get keyframes key");
            let key_str = pack.string(off, len).expect("should resolve string");
            assert!(
                key_str == "stand_light" || key_str == "stand_medium",
                "unexpected keyframes key: {}",
                key_str
            );
        }
    }

    /// Roundtrip test verifying moves with empty animations get KEY_NONE.
    #[test]
    fn test_roundtrip_moves_without_animation() {
        let char_data = CharacterData {
            character: make_test_character("test"),
            moves: vec![
                make_test_move("5L", "stand_light"),
                make_test_move("idle", ""), // No animation
            ],
            cancel_table: make_empty_cancel_table(),
        };

        // Export to bytes
        let bytes = export_fspk(&char_data).expect("export should succeed");

        // Parse with framesmith_fspack reader
        let pack = framesmith_fspack::PackView::parse(&bytes).expect("parse should succeed");

        // Verify we have 2 moves
        let moves = pack.states().expect("should have MOVES section");
        assert_eq!(moves.len(), 2);

        // But only 1 keyframes key (stand_light)
        let kf_keys = pack
            .keyframes_keys()
            .expect("should have KEYFRAMES_KEYS section");
        assert_eq!(kf_keys.len(), 1);

        // First move (5L) should have a valid key
        let mv0 = moves.get(0).expect("should get move 0");
        assert_ne!(mv0.mesh_key(), framesmith_fspack::KEY_NONE);

        // Second move (idle) should have KEY_NONE since it has no animation
        let mv1 = moves.get(1).expect("should get move 1");
        assert_eq!(mv1.mesh_key(), framesmith_fspack::KEY_NONE);
        assert_eq!(mv1.keyframes_key(), framesmith_fspack::KEY_NONE);
    }

    // NOTE: Chain cancel tests removed - chains are deprecated.
    // Cancel rules are now handled through tag_rules with bitfield conditions.
}
