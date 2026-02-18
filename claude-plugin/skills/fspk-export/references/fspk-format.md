# FSPK Binary Format Reference

Complete byte-level specification for the FSPK (Framesmith Pack) container format.

## Container Header (16 bytes)

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0 | 4 | magic | `"FSPK"` (0x46, 0x53, 0x50, 0x4B) |
| 4 | 4 | flags | Reserved (currently `0`) |
| 8 | 4 | total_len | Total size of the pack in bytes |
| 12 | 4 | section_count | Number of sections following the header |

## Section Header (16 bytes each)

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0 | 4 | kind | Section type identifier (see Section Types) |
| 4 | 4 | offset | Byte offset of section data from start of file |
| 8 | 4 | len | Length of section data in bytes |
| 12 | 4 | align | Alignment requirement (typically 4) |

## Section Types

| Kind | ID | Description |
|------|-----|-------------|
| STRING_TABLE | 1 | Raw UTF-8 string data |
| MESH_KEYS | 2 | Array of StrRef pointing to mesh asset keys |
| KEYFRAMES_KEYS | 3 | Array of StrRef pointing to animation asset keys |
| STATES | 4 | Array of StateRecord structs |
| HIT_WINDOWS | 5 | Array of HitWindow24 structs |
| HURT_WINDOWS | 6 | Array of HurtWindow12 structs |
| SHAPES | 7 | Array of Shape12 structs |
| CANCELS_U16 | 8 | Array of u16 state IDs for cancel targets |
| RESOURCE_DEFS | 9 | Array of ResourceDef12 structs |
| STATE_EXTRAS | 10 | Array of StateExtras72 structs (parallel to STATES) |
| EVENT_EMITS | 11 | Array of EventEmit16 structs |
| EVENT_ARGS | 12 | Array of EventArg20 structs |
| STATE_NOTIFIES | 13 | Array of StateNotify12 structs |
| STATE_RESOURCE_COSTS | 14 | Array of StateResourceCost12 structs |
| STATE_RESOURCE_PRECONDITIONS | 15 | Array of StateResourcePrecondition12 structs |
| STATE_RESOURCE_DELTAS | 16 | Array of StateResourceDelta16 structs |
| STATE_TAG_RANGES | 17 | Array of StateTagRange8 structs (parallel to STATES) |
| STATE_TAGS | 18 | Array of StrRef pointing to tag strings |
| CANCEL_TAG_RULES | 19 | Array of CancelTagRule24 structs |
| CANCEL_DENIES | 20 | Array of CancelDeny4 structs |
| CHARACTER_PROPS | 21 | Array of CharacterProp12 structs |
| PUSH_WINDOWS | 22 | Array of PushWindow12 structs |
| STATE_PROPS | 23 | Per-state properties (index + CharacterProp12 records) |
| SCHEMA | 24 | Property and tag schema definitions |

## Core Data Structures

### StrRef (8 bytes)

String references into STRING_TABLE:

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0 | 4 | off | Byte offset into STRING_TABLE |
| 4 | 2 | len | String length in bytes |
| 6 | 2 | _pad | Padding (reserved) |

### StateRecord (36 bytes)

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0 | 2 | state_id | Index in the STATES array |
| 2 | 2 | mesh_key | Index into MESH_KEYS (0xFFFF = none) |
| 4 | 2 | keyframes_key | Index into KEYFRAMES_KEYS (0xFFFF = none) |
| 6 | 1 | state_type | State type enum |
| 7 | 1 | trigger | Input trigger type |
| 8 | 1 | guard | Guard type (high/mid/low) |
| 9 | 1 | flags | State flags (see below) |
| 10 | 1 | startup | Startup frames |
| 11 | 1 | active | Active frames |
| 12 | 1 | recovery | Recovery frames |
| 13 | 1 | _reserved | Reserved |
| 14 | 2 | total | Total frame duration |
| 16 | 2 | damage | Damage value |
| 18 | 1 | hitstun | Hitstun frames |
| 19 | 1 | blockstun | Blockstun frames |
| 20 | 1 | hitstop | Hitstop frames |
| 21 | 1 | _reserved | Reserved |
| 22 | 4 | hit_windows_off | Byte offset within HIT_WINDOWS section |
| 26 | 2 | hit_windows_len | Number of hit windows |
| 28 | 2 | hurt_windows_off | Byte offset within HURT_WINDOWS (compressed u16) |
| 30 | 2 | hurt_windows_len | Number of hurt windows |
| 32 | 2 | push_windows_off | Byte offset within PUSH_WINDOWS (compressed u16) |
| 34 | 2 | push_windows_len | Number of push windows |

**State Flags (flags byte):**

| Bit | Flag | Description |
|-----|------|-------------|
| 0x01 | CHAIN | Has chain cancel routes in CANCELS_U16 |
| 0x02 | SPECIAL | Can cancel into special moves |
| 0x04 | SUPER | Can cancel into super moves |
| 0x08 | JUMP | Can cancel into jump |
| 0x10 | SELF_GATLING | Can cancel into itself |

### HitWindow24 (24 bytes)

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0 | 1 | start_f | Start frame |
| 1 | 1 | end_f | End frame |
| 2 | 1 | guard | Guard type |
| 3 | 1 | _reserved | Reserved |
| 4 | 2 | dmg | Damage |
| 6 | 2 | chip | Chip damage (0 = none) |
| 8 | 1 | hitstun | Hitstun frames |
| 9 | 1 | blockstun | Blockstun frames |
| 10 | 1 | hitstop | Hitstop frames |
| 11 | 1 | _reserved | Reserved |
| 12 | 4 | shapes_off | Offset into SHAPES section |
| 16 | 2 | shapes_len | Number of shapes |
| 18 | 4 | cancels_off | Offset into CANCELS_U16 section |
| 22 | 2 | cancels_len | Number of cancel targets |

### HurtWindow12 (12 bytes)

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0 | 1 | start_f | Start frame |
| 1 | 1 | end_f | End frame |
| 2 | 2 | hurt_flags | Hurtbox flags (invincibility, etc.) |
| 4 | 4 | shapes_off | Offset into SHAPES section |
| 8 | 2 | shapes_len | Number of shapes |
| 10 | 2 | _reserved | Reserved |

### PushWindow12 (12 bytes)

Same layout as HurtWindow12 but for body collision boxes.

### Shape12 (12 bytes)

Q12.4 fixed-point coordinates (1/16 pixel precision):

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0 | 1 | kind | 0=aabb, 1=rect, 2=circle, 3=capsule |
| 1 | 1 | flags | Reserved |
| 2 | 2 | a | Q12.4: x (aabb/rect/circle), x1 (capsule) |
| 4 | 2 | b | Q12.4: y (aabb/rect/circle), y1 (capsule) |
| 6 | 2 | c | Q12.4: width (aabb/rect), radius (circle), x2 (capsule) |
| 8 | 2 | d | Q12.4: height (aabb/rect), unused (circle), y2 (capsule) |
| 10 | 2 | e | Q8.8: angle (rect), radius (capsule) |

## Extended Data Structures

### StateExtras72 (72 bytes)

Per-state offsets/lengths for optional arrays (parallel to STATES). Each range is 8 bytes: `off(u32) + len(u16) + _pad(u16)`.

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0 | 8 | on_use_emits | Range into EVENT_EMITS |
| 8 | 8 | on_hit_emits | Range into EVENT_EMITS |
| 16 | 8 | on_block_emits | Range into EVENT_EMITS |
| 24 | 8 | notifies | Range into STATE_NOTIFIES |
| 32 | 8 | resource_costs | Range into STATE_RESOURCE_COSTS |
| 40 | 8 | resource_preconditions | Range into STATE_RESOURCE_PRECONDITIONS |
| 48 | 8 | resource_deltas | Range into STATE_RESOURCE_DELTAS |
| 56 | 8 | input_notation | StrRef to notation (e.g., "5L", "236P") |
| 64 | 8 | cancels | Range into CANCELS_U16 for chain targets |

### ResourceDef12 (12 bytes)

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0 | 8 | name | StrRef to resource name |
| 8 | 2 | start | Starting amount |
| 10 | 2 | max | Max amount |

### EventEmit16 (16 bytes)

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0 | 8 | id | StrRef to event id |
| 8 | 8 | args | Range into EVENT_ARGS |

### EventArg20 (20 bytes)

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0 | 8 | key | StrRef to arg key |
| 8 | 1 | tag | Value type: 0=bool, 1=i64, 2=f32, 3=string/enum |
| 9 | 3 | _reserved | Reserved (0) |
| 12 | 8 | value | Type-dependent payload |

Value encoding by tag: 0 (bool) = u64 0/nonzero, 1 (i64) = i64 LE, 2 (f32) = f32 LE lower 4 bytes, 3 (string) = StrRef packed as off(u32)+len(u16)+_pad(u16).

### StateNotify12 (12 bytes)

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0 | 2 | frame | Frame number |
| 2 | 2 | _pad | Reserved |
| 4 | 8 | emits | Range into EVENT_EMITS |

### StateResourceCost12 (12 bytes)

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0 | 8 | name | StrRef to resource name |
| 8 | 2 | amount | Cost amount |
| 10 | 2 | _pad | Reserved |

### StateResourcePrecondition12 (12 bytes)

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0 | 8 | name | StrRef to resource name |
| 8 | 2 | min | Minimum required (0xFFFF = none) |
| 10 | 2 | max | Maximum allowed (0xFFFF = none) |

### StateResourceDelta16 (16 bytes)

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0 | 8 | name | StrRef to resource name |
| 8 | 4 | delta | Signed delta (i32) |
| 12 | 1 | trigger | 0=on_use, 1=on_hit, 2=on_block |
| 13 | 3 | _pad | Reserved |

### StateTagRange8 (8 bytes)

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0 | 4 | offset | Byte offset into STATE_TAGS |
| 4 | 2 | count | Number of tags |
| 6 | 2 | _pad | Reserved |

### CancelTagRule24 (24 bytes)

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0 | 4 | from_tag_off | StrRef offset for source (0xFFFFFFFF = any) |
| 4 | 2 | from_tag_len | StrRef length |
| 6 | 2 | _pad1 | Reserved |
| 8 | 4 | to_tag_off | StrRef offset for target (0xFFFFFFFF = any) |
| 12 | 2 | to_tag_len | StrRef length |
| 14 | 2 | _pad2 | Reserved |
| 16 | 1 | condition | 0=always, 1=on_hit, 2=on_block, 3=on_whiff |
| 17 | 1 | min_frame | Min frame for cancel (0 = no min) |
| 18 | 1 | max_frame | Max frame for cancel (0 = no max) |
| 19 | 1 | flags | Reserved |
| 20 | 4 | _pad3 | Reserved |

### CancelDeny4 (4 bytes)

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0 | 2 | from_idx | Source state index |
| 2 | 2 | to_idx | Target state index |

### CharacterProp12 (12 bytes)

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0 | 4 | name_off | Byte offset into STRING_TABLE |
| 4 | 2 | name_len | Property name length |
| 6 | 1 | value_type | 0=Q24.8 number, 1=bool, 2=string |
| 7 | 1 | _reserved | Reserved |
| 8 | 4 | value | Type-dependent payload |

Q24.8: signed fixed-point, range ~+/-8M with 1/256 precision.

### SchemaProp8 (8 bytes)

Compact format when SCHEMA section is present (replaces CharacterProp12):

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0 | 2 | schema_id | Index into schema's property list |
| 2 | 1 | value_type | 0=Q24.8, 1=bool, 2=string ref |
| 3 | 1 | _reserved | Reserved |
| 4 | 4 | value | Type-dependent payload |

### SCHEMA Section Layout

1. **Header (8 bytes):** char_prop_count(u16) + state_prop_count(u16) + tag_count(u16) + _pad(u16)
2. **Character property names:** `[StrRef; char_prop_count]`
3. **State property names:** `[StrRef; state_prop_count]`
4. **Tag names:** `[StrRef; tag_count]`

### STATE_PROPS Section Layout

1. **Index** (8 bytes per state, parallel to STATES): offset(u32) + byte_len(u16) + _pad(u16)
2. **Property Data**: Concatenated CharacterProp12 (or SchemaProp8 if schema present)

States without properties have `(0, 0)` in their index entry.

## Parse Errors

| Error | Description |
|-------|-------------|
| `TooShort` | Input data too short for valid header |
| `InvalidMagic` | Magic bytes are not "FSPK" |
| `OutOfBounds` | Section offset/length exceeds data bounds |
