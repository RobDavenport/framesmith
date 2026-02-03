//! Binary record packing for shapes, hitboxes, and move records.

use crate::codegen::fspk_format::{
    to_q12_4, to_q12_4_unsigned, HIT_WINDOW24_SIZE, HURT_WINDOW12_SIZE, SHAPE12_SIZE,
    SHAPE_KIND_AABB, STATE_RECORD_SIZE,
};
use crate::schema::{FrameHitbox, GuardType, Rect, State};

/// Pack a Rect into a Shape12 (AABB) structure.
///
/// Shape12 layout:
/// - kind (u8): shape type (0 = AABB)
/// - flags (u8): reserved
/// - a (i16): x position (Q12.4)
/// - b (i16): y position (Q12.4)
/// - c (u16): width (Q12.4 unsigned)
/// - d (u16): height (Q12.4 unsigned)
/// - e (i16): unused for AABB
pub fn pack_shape(rect: &Rect) -> [u8; SHAPE12_SIZE] {
    let mut buf = [0u8; SHAPE12_SIZE];
    buf[0] = SHAPE_KIND_AABB; // kind
    buf[1] = 0; // flags

    let x = to_q12_4(rect.x as f32);
    let y = to_q12_4(rect.y as f32);
    let w = to_q12_4_unsigned(rect.w as f32);
    let h = to_q12_4_unsigned(rect.h as f32);

    buf[2..4].copy_from_slice(&x.to_le_bytes()); // a = x
    buf[4..6].copy_from_slice(&y.to_le_bytes()); // b = y
    buf[6..8].copy_from_slice(&w.to_le_bytes()); // c = w
    buf[8..10].copy_from_slice(&h.to_le_bytes()); // d = h
    buf[10..12].copy_from_slice(&0i16.to_le_bytes()); // e = 0

    buf
}

/// Convert GuardType to u8 for binary encoding.
pub fn guard_type_to_u8(guard: &GuardType) -> u8 {
    match guard {
        GuardType::High => 0,
        GuardType::Mid => 1,
        GuardType::Low => 2,
        GuardType::Unblockable => 3,
    }
}

/// Pack a FrameHitbox into a HitWindow24 structure.
///
/// HitWindow24 layout (24 bytes) - must match view.rs HitWindowView:
/// - 0: start_frame (u8)
/// - 1: end_frame (u8)
/// - 2: guard (u8)
/// - 3: reserved (u8)
/// - 4-5: damage (u16 LE)
/// - 6-7: chip_damage (u16 LE)
/// - 8: hitstun (u8)
/// - 9: blockstun (u8)
/// - 10: hitstop (u8)
/// - 11: reserved (u8)
/// - 12-15: shapes_off (u32 LE)
/// - 16-17: shapes_len (u16 LE)
/// - 18-21: cancels_off (u32 LE)
/// - 22-23: cancels_len (u16 LE)
pub fn pack_hit_window(
    hb: &FrameHitbox,
    shapes_off: u32,
    damage: u16,
    hitstun: u8,
    blockstun: u8,
    hitstop: u8,
    guard: u8,
) -> [u8; HIT_WINDOW24_SIZE] {
    let mut buf = [0u8; HIT_WINDOW24_SIZE];

    buf[0] = hb.frames.0; // start_frame
    buf[1] = hb.frames.1; // end_frame
    buf[2] = guard; // guard
    buf[3] = 0; // reserved
    buf[4..6].copy_from_slice(&damage.to_le_bytes()); // damage
    buf[6..8].copy_from_slice(&0u16.to_le_bytes()); // chip_damage (TODO: add to schema)
    buf[8] = hitstun; // hitstun
    buf[9] = blockstun; // blockstun
    buf[10] = hitstop; // hitstop
    buf[11] = 0; // reserved
    buf[12..16].copy_from_slice(&shapes_off.to_le_bytes()); // shapes_off
    buf[16..18].copy_from_slice(&1u16.to_le_bytes()); // shapes_len = 1
    // bytes 18-27 are cancels/pushback (already zeroed, not used in v1)

    buf
}

/// Pack a FrameHitbox into a HurtWindow12 structure.
///
/// HurtWindow12 layout (12 bytes):
/// - frame_start (u8): first active frame
/// - frame_end (u8): last active frame
/// - shape_off (u32): offset into SHAPES section
/// - shape_count (u16): number of shapes (always 1 for v1)
/// - flags (u16): hurtbox flags (invuln, armor, etc.)
/// - reserved (2 bytes): padding
pub fn pack_hurt_window(hb: &FrameHitbox, shapes_off: u32) -> [u8; HURT_WINDOW12_SIZE] {
    let mut buf = [0u8; HURT_WINDOW12_SIZE];

    buf[0] = hb.frames.0; // frame_start
    buf[1] = hb.frames.1; // frame_end
    buf[2..6].copy_from_slice(&shapes_off.to_le_bytes()); // shape_off
    buf[6..8].copy_from_slice(&1u16.to_le_bytes()); // shape_count = 1
    buf[8..10].copy_from_slice(&0u16.to_le_bytes()); // flags = 0 for v1
                                                     // bytes 10-11 are reserved/padding (already zeroed)

    buf
}

/// Convert move type string to u8 for binary encoding.
/// Maps common type strings to fixed IDs for runtime compatibility.
pub fn move_type_to_u8(move_type: Option<&String>) -> u8 {
    match move_type.map(|s| s.as_str()) {
        Some("normal") => 0,
        Some("command_normal") => 1,
        Some("special") => 2,
        Some("super") => 3,
        Some("movement") => 4,
        Some("throw") => 5,
        Some("ex") => 6,
        Some("rekka") => 7,
        Some(_) => 255, // unknown custom type
        None => 0,      // default to normal
    }
}

/// Convert TriggerType to u8 for binary encoding.
pub fn trigger_type_to_u8(trigger: Option<&crate::schema::TriggerType>) -> u8 {
    use crate::schema::TriggerType;
    match trigger {
        Some(TriggerType::Press) => 0,
        Some(TriggerType::Release) => 1,
        Some(TriggerType::Hold) => 2,
        None => 0, // default to Press
    }
}

/// Pack a Move into a MoveRecord structure.
///
/// MoveRecord layout (36 bytes):
/// - 0-1: move_id (u16)
/// - 2-3: mesh_key (u16)
/// - 4-5: keyframes_key (u16)
/// - 6: move_type (u8)
/// - 7: trigger (u8)
/// - 8: guard (u8)
/// - 9: flags (u8)
/// - 10: startup (u8)
/// - 11: active (u8)
/// - 12: recovery (u8)
/// - 13: reserved (u8)
/// - 14-15: total (u16)
/// - 16-17: damage (u16)
/// - 18: hitstun (u8)
/// - 19: blockstun (u8)
/// - 20: hitstop (u8)
/// - 21: reserved (u8)
/// - 22-25: hit_windows_off (u32)
/// - 26-27: hit_windows_len (u16)
/// - 28-29: hurt_windows_off (u16)
/// - 30-31: hurt_windows_len (u16)
/// - 32-33: push_windows_off (u16)
/// - 34-35: push_windows_len (u16)
#[allow(clippy::too_many_arguments)] // Binary record packing requires all fields
pub fn pack_move_record(
    move_id: u16,
    mesh_key: u16,
    keyframes_key: u16,
    mv: &State,
    hit_windows_off: u32,
    hit_windows_len: u16,
    hurt_windows_off: u16,
    hurt_windows_len: u16,
    push_windows_off: u16,
    push_windows_len: u16,
    flags: u8,
) -> [u8; STATE_RECORD_SIZE] {
    let mut buf = [0u8; STATE_RECORD_SIZE];

    buf[0..2].copy_from_slice(&move_id.to_le_bytes()); // move_id
    buf[2..4].copy_from_slice(&mesh_key.to_le_bytes()); // mesh_key
    buf[4..6].copy_from_slice(&keyframes_key.to_le_bytes()); // keyframes_key
    buf[6] = move_type_to_u8(mv.move_type.as_ref()); // move_type
    buf[7] = trigger_type_to_u8(mv.trigger.as_ref()); // trigger
    buf[8] = guard_type_to_u8(&mv.guard); // guard
    buf[9] = flags; // cancel flags
    buf[10] = mv.startup; // startup
    buf[11] = mv.active; // active
    buf[12] = mv.recovery; // recovery
    buf[13] = 0; // reserved
    let total = mv
        .total
        .map(|t| t as u16)
        .unwrap_or_else(|| (mv.startup as u16) + (mv.active as u16) + (mv.recovery as u16));
    buf[14..16].copy_from_slice(&total.to_le_bytes()); // total
    buf[16..18].copy_from_slice(&mv.damage.to_le_bytes()); // damage
    buf[18] = mv.hitstun; // hitstun
    buf[19] = mv.blockstun; // blockstun
    buf[20] = mv.hitstop; // hitstop
    buf[21] = 0; // reserved
    buf[22..26].copy_from_slice(&hit_windows_off.to_le_bytes()); // hit_windows_off
    buf[26..28].copy_from_slice(&hit_windows_len.to_le_bytes()); // hit_windows_len
    buf[28..30].copy_from_slice(&hurt_windows_off.to_le_bytes()); // hurt_windows_off (u16)
    buf[30..32].copy_from_slice(&hurt_windows_len.to_le_bytes()); // hurt_windows_len
    buf[32..34].copy_from_slice(&push_windows_off.to_le_bytes()); // push_windows_off (u16)
    buf[34..36].copy_from_slice(&push_windows_len.to_le_bytes()); // push_windows_len

    buf
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::fspk_format::SHAPE_KIND_AABB;

    fn make_test_rect() -> Rect {
        Rect { x: 10, y: 20, w: 50, h: 60 }
    }

    fn make_test_hitbox() -> FrameHitbox {
        FrameHitbox {
            frames: (5, 8),
            r#box: make_test_rect(),
        }
    }

    #[test]
    fn test_pack_shape() {
        let rect = make_test_rect();
        let shape = pack_shape(&rect);

        assert_eq!(shape.len(), SHAPE12_SIZE);
        assert_eq!(shape[0], SHAPE_KIND_AABB);
        assert_eq!(shape[1], 0); // flags

        // x=10 -> Q12.4 = 160 = 0x00A0
        let x = i16::from_le_bytes([shape[2], shape[3]]);
        assert_eq!(x, 160);

        // y=20 -> Q12.4 = 320 = 0x0140
        let y = i16::from_le_bytes([shape[4], shape[5]]);
        assert_eq!(y, 320);

        // w=50 -> Q12.4 = 800 = 0x0320
        let w = u16::from_le_bytes([shape[6], shape[7]]);
        assert_eq!(w, 800);

        // h=60 -> Q12.4 = 960 = 0x03C0
        let h = u16::from_le_bytes([shape[8], shape[9]]);
        assert_eq!(h, 960);
    }

    #[test]
    fn test_pack_hit_window() {
        let hb = make_test_hitbox();
        let hw = pack_hit_window(&hb, 100, 500, 12, 8, 10, 1);

        assert_eq!(hw.len(), HIT_WINDOW24_SIZE);
        assert_eq!(hw[0], 5); // frame_start
        assert_eq!(hw[1], 8); // frame_end
        assert_eq!(hw[2], 1); // guard (mid)
    }

    #[test]
    fn test_pack_hurt_window() {
        let hb = make_test_hitbox();
        let hw = pack_hurt_window(&hb, 200);

        assert_eq!(hw.len(), HURT_WINDOW12_SIZE);
        assert_eq!(hw[0], 5); // frame_start
        assert_eq!(hw[1], 8); // frame_end

        let shape_off = u32::from_le_bytes([hw[2], hw[3], hw[4], hw[5]]);
        assert_eq!(shape_off, 200);
    }

    #[test]
    fn test_guard_type_encoding() {
        assert_eq!(guard_type_to_u8(&GuardType::High), 0);
        assert_eq!(guard_type_to_u8(&GuardType::Mid), 1);
        assert_eq!(guard_type_to_u8(&GuardType::Low), 2);
        assert_eq!(guard_type_to_u8(&GuardType::Unblockable), 3);
    }

    #[test]
    fn test_negative_coordinates() {
        let rect = Rect { x: -50, y: -100, w: 30, h: 40 };
        let shape = pack_shape(&rect);

        let x = i16::from_le_bytes([shape[2], shape[3]]);
        assert_eq!(x, -800); // -50 -> Q12.4 = -800

        let y = i16::from_le_bytes([shape[4], shape[5]]);
        assert_eq!(y, -1600); // -100 -> Q12.4 = -1600
    }
}
