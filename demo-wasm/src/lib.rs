//! A small game consumer. Combat policy stays here, not in the helper crates.
use framesmith_fspack::{payload::ValueView, OwnedPack};
use framesmith_runtime::{
    aabb_overlap, check_hits, init_resources, next_frame, report_hit, Aabb, CharacterState,
    FrameInput, HitResult,
};
use serde::Serialize;
const LEFT: u8 = 1;
const RIGHT: u8 = 2;
const UP: u8 = 4;
const DOWN: u8 = 8;
const JUMP_BUTTON: u8 = 16;
const NORMAL: u8 = 32;
const SPECIAL: u8 = 64;
const INPUT_BITS: u16 = 127;
const IDLE: usize = 0;
const BLOCK: usize = 1;
const CROUCH: usize = 2;
const JUMP: usize = 3;
const JAB: usize = 4;
const MID: usize = 5;
const DRIVE: usize = 6;
const LOW: usize = 7;
const LAUNCH: usize = 8;
const AIR_L: usize = 9;
const AIR_M: usize = 10;
const AIR_H: usize = 11;
const SKILL: usize = 12;
const ANTI: usize = 13;
const SUPER: usize = 14;
const STUN: usize = 15;
const LAND: usize = 16;
const AIR_S: usize = 17;
const AIR_UP: usize = 18;
const IDS: [&str; 19] = [
    "idle",
    "guard",
    "crouch",
    "jump",
    "light",
    "medium",
    "heavy",
    "low",
    "launch",
    "air_light",
    "air_medium",
    "air_heavy",
    "special",
    "anti_air",
    "burst",
    "stun",
    "landing",
    "air_special",
    "air_up",
];
const HZ: u32 = 60; // Demo policy, not a FrameSmith clock requirement.
const MATCH_FRAMES: u32 = HZ * 180;
const SCALE: i32 = 256;
const PROJECTILES: usize = 8;
const STOCKS: u8 = 3;
const BLAST: [i32; 4] = [0, -470, 800, 210]; // Left, top, right, bottom; feet coordinates.
#[derive(Clone, Copy, Debug, Serialize)]
struct Platform {
    left: i32,
    right: i32,
    top: i32,
}
const PLATFORMS: [Platform; 4] = [
    Platform {
        left: 120,
        right: 680,
        top: 0,
    },
    Platform {
        left: 190,
        right: 320,
        top: -95,
    },
    Platform {
        left: 480,
        right: 610,
        top: -95,
    },
    Platform {
        left: 335,
        right: 465,
        top: -190,
    },
];
fn attacking(role: usize) -> bool {
    (JAB..=SUPER).contains(&role) || matches!(role, AIR_S | AIR_UP)
}
fn number(value: ValueView<'_>) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_i64().map(|v| v as f64))
        .or_else(|| value.as_u64().map(|v| v as f64))
}
fn required_number(v: ValueView<'_>, key: &str, min: f64, max: f64) -> Result<f64, String> {
    let n = v
        .get(key)
        .and_then(number)
        .ok_or_else(|| format!("Missing numeric payload: {key}"))?;
    if !n.is_finite() || n < min || n > max || (n * f64::from(SCALE)).fract() != 0.0 {
        return Err(format!("Out-of-range or non-quantized payload: {key}"));
    }
    Ok(n)
}
#[derive(Clone, Copy, Default)]
struct MoveRule {
    travel: i32,
    lift: i32,
    launch: i32,
    projectile: bool,
    projectile_speed: i32,
    projectile_y: i32,
    grab: bool,
    knockdown: bool,
    projectile_invuln: bool,
}
struct Definition {
    pack: OwnedPack,
    ids: [u16; IDS.len()],
    rules: [MoveRule; IDS.len()],
    name: String,
    color: String,
    style: String,
    weight: i32,
    speed: i32,
    jump: i32,
    gravity: i32,
    width: i32,
    height: i32,
    counter_bonus: i32,
    meter: usize,
    meter_max: u16,
    bytes: usize,
}
impl Definition {
    fn new(bytes: &[u8]) -> Result<Self, String> {
        let pack = OwnedPack::new(bytes.to_vec()).map_err(|e| e.to_string())?;
        let view = pack.view();
        let root = view
            .payload()
            .ok_or("Arena requires full-fidelity FSPK v2")?
            .root();
        let character = root.get("character").ok_or("Missing character payload")?;
        let props = character
            .get("properties")
            .ok_or("Missing character properties")?;
        let text = |v: ValueView<'_>, key: &str| {
            v.get(key)
                .and_then(ValueView::as_str)
                .map(str::to_owned)
                .ok_or_else(|| format!("Missing {key}"))
        };
        let name = text(character, "name")?;
        let color = text(props, "color")?;
        let style = text(props, "style")?;
        if !["shoto", "grappler", "zoner", "rushdown"].contains(&style.as_str()) {
            return Err("Unknown archetype".into());
        }
        if color.len() != 7
            || !color.starts_with('#')
            || !color[1..].bytes().all(|v| v.is_ascii_hexdigit())
        {
            return Err("Invalid color".into());
        }
        let weight = required_number(props, "weight", 40., 200.)? as i32;
        let speed = (required_number(props, "walk_speed", 0.25, 8.)? * SCALE as f64) as i32;
        let jump = (required_number(props, "jump_speed", 4., 14.)? * SCALE as f64) as i32;
        let gravity = (required_number(props, "gravity", 0.25, 1.)? * SCALE as f64) as i32;
        let width = required_number(props, "width", 20., 64.)? as i32;
        let height = required_number(props, "height", 60., 120.)? as i32;
        let counter_bonus = required_number(props, "counter_bonus", 0., 100.)? as i32;
        let states = view.states().ok_or("Missing compiled states")?;
        let mut ids = [0; IDS.len()];
        let mut rules = [MoveRule::default(); IDS.len()];
        for (role, name) in IDS.iter().enumerate() {
            ids[role] = (0..states.len())
                .find(|&i| view.state_id(i) == Some(*name))
                .ok_or_else(|| format!("Missing authored state '{name}'"))?
                as u16;
            let v = view
                .state_data(ids[role] as usize)
                .and_then(|v| v.get("properties"))
                .ok_or("Missing move properties")?;
            let n = |k: &str, min: f64, max: f64| -> Result<f64, String> {
                if v.get(k).is_some() {
                    required_number(v, k, min, max)
                } else {
                    Ok(0.)
                }
            };
            rules[role] = MoveRule {
                travel: (n("travel", 0., 10.)? * SCALE as f64) as i32,
                lift: (n("lift", 0., 14.)? * SCALE as f64) as i32,
                launch: (n("launch", -14., 0.)? * SCALE as f64) as i32,
                projectile: n("projectile", 0., 1.)? == 1.,
                projectile_speed: (n("projectile_speed", 0., 12.)? * SCALE as f64) as i32,
                projectile_y: (n("projectile_y", -8., 8.)? * SCALE as f64) as i32,
                grab: n("grab", 0., 1.)? == 1.,
                knockdown: n("knockdown", 0., 1.)? == 1.,
                projectile_invuln: n("projectile_invuln", 0., 1.)? == 1.,
            };
        }
        // ponytail: attack-local mirroring needs centered AABB body shapes. Reject other fixtures;
        // add facing-aware arbitrary shape queries only when this consumer needs asymmetric bodies.
        let shapes = view.shapes().ok_or("Missing shapes")?;
        if let Some(windows) = view.hurt_windows() {
            for i in 0..windows.len() {
                let w = windows.get(i).ok_or("Invalid hurt window")?;
                for j in 0..usize::from(w.shapes_len()) {
                    let s = shapes
                        .get_at(w.shapes_off(), j)
                        .ok_or("Invalid body shape")?;
                    if s.kind() != 0 || s.x_px() * 2 + s.width_px() as i32 != 0 {
                        return Err("Arena requires symmetric AABB hurtboxes".into());
                    }
                }
            }
        }
        let resources = view.resource_defs().ok_or("Missing resource definitions")?;
        let meter = (0..resources.len())
            .find(|&i| {
                resources
                    .get(i)
                    .is_some_and(|d| view.string(d.name_off(), d.name_len()) == Some("meter"))
            })
            .ok_or("Missing meter pool")?;
        let meter_max = resources.get(meter).ok_or("Invalid meter")?.max();
        let mut state = CharacterState::default();
        if !init_resources(&mut state, &view) {
            return Err("Unsupported resource capacity".into());
        }
        Ok(Self {
            pack,
            ids,
            rules,
            name,
            color,
            style,
            weight,
            speed,
            jump,
            gravity,
            width,
            height,
            counter_bonus,
            meter,
            meter_max,
            bytes: bytes.len(),
        })
    }
    fn role(&self, s: &CharacterState) -> usize {
        self.ids
            .iter()
            .position(|&id| id == s.current_state)
            .unwrap_or(STUN)
    }
    fn actor(&self, x: i32, facing: i32) -> Actor {
        let mut state = CharacterState {
            current_state: self.ids[IDLE],
            ..Default::default()
        };
        assert!(init_resources(&mut state, &self.pack.view()));
        Actor {
            state,
            x: x * SCALE,
            facing,
            stocks: STOCKS,
            platform: 0,
            ..Default::default()
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Actor {
    state: CharacterState,
    x: i32,
    y: i32,
    vx: i32,
    vy: i32,
    facing: i32,
    damage: i32,
    stocks: u8,
    platform: i8,
    jumps: u8,
    recovery_used: bool,
    drop_timer: u8,
    respawn: u8,
    invulnerable: u8,
    previous_input: u8,
    control: u8,
    buffer: u8,
    buffer_age: u8,
    connected: bool,
    serial: u32,
    combo: u8,
    combo_damage: i32,
    throw_immune: u8,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
struct Stats {
    hits: [u32; 2],
    cancels: [u32; 2],
    spent: [u32; 2],
    signals: [u32; 2],
    shots: [u32; 2],
    throws: [u32; 2],
    air_hits: [u32; 2],
    max_combo: [u8; 2],
    kos: [u32; 2],
    clashes: u32,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
struct Contact {
    tick: u32,
    who: usize,
    damage: i32,
    counter: bool,
    serial: u32,
    x: i32,
    y: i32,
    kind: u8,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Projectile {
    live: bool,
    owner: usize,
    role: usize,
    serial: u32,
    x: i32,
    y: i32,
    vx: i32,
    vy: i32,
    w: u32,
    h: u32,
    facing: i32,
    life: u16,
}
impl Projectile {
    fn bounds(&self) -> Aabb {
        Aabb {
            x: self.x / SCALE,
            y: self.y / SCALE,
            w: self.w,
            h: self.h,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct World {
    actors: [Actor; 2],
    projectiles: [Projectile; PROJECTILES],
    tick: u32,
    rng: u32,
    mode: u8,
    ai_wait: u16,
    hitstop: u8,
    winner: u8,
    super_tick: u32,
    super_who: usize,
    ko_tick: u32,
    ko_who: usize,
    stats: Stats,
    contact: Contact,
}
fn rng(w: &mut World) -> u32 {
    w.rng = w.rng.wrapping_mul(1664525).wrapping_add(1013904223);
    w.rng
}
fn axis(c: u8) -> i32 {
    i32::from(c & RIGHT != 0) - i32::from(c & LEFT != 0)
}
fn direction(from: i32, to: i32) -> u8 {
    if to >= from {
        RIGHT
    } else {
        LEFT
    }
}
fn bot(w: &mut World, d: &[Definition; 2]) -> u8 {
    let a = w.actors[1];
    let b = w.actors[0];
    let role = d[1].role(&a.state);
    if w.mode == 1 || a.respawn > 0 {
        return 0;
    }
    let inward = direction(a.x, 400 * SCALE);
    if role == STUN {
        return inward;
    }
    let forward = direction(a.x, b.x);
    let dist = (a.x - b.x).abs() / SCALE;
    // Recover before choosing attacks: no suicidal pursuit beyond the main deck.
    if a.platform < 0 && (a.x < 145 * SCALE || a.x > 655 * SCALE || a.y > 10 * SCALE) {
        if a.vy >= 0 && a.jumps < 2 && a.previous_input & JUMP_BUTTON == 0 {
            return inward | JUMP_BUTTON;
        }
        if a.jumps >= 2 && !a.recovery_used && a.vy >= 0 {
            return inward | UP | SPECIAL;
        }
        return inward;
    }
    if attacking(role) {
        return if a.state.hit_confirmed && matches!(role, JAB | MID | AIR_L | AIR_M) {
            NORMAL
        } else {
            0
        };
    }
    if a.platform == 0 && (a.x < 155 * SCALE || a.x > 645 * SCALE) {
        return inward;
    }
    if a.platform > 0 && b.y > a.y + 60 * SCALE && dist < 140 && a.previous_input & JUMP_BUTTON == 0
    {
        return DOWN | JUMP_BUTTON;
    }
    w.ai_wait = w.ai_wait.saturating_sub(1);
    if w.ai_wait == 0 {
        let choice = rng(w);
        w.ai_wait = 10 + (choice % 14) as u16;
        if a.platform >= 0 && (b.y < a.y - 45 * SCALE || dist > 145 && choice.is_multiple_of(4)) {
            return forward | JUMP_BUTTON;
        }
        if a.state.resources[d[1].meter] >= 50
            && dist < if d[1].style == "grappler" { 70 } else { 190 }
            && choice.is_multiple_of(5)
        {
            return forward | DOWN | SPECIAL;
        }
        if (a.y - b.y).abs() < 75 * SCALE {
            match d[1].style.as_str() {
                "grappler" if dist < 75 => return forward | SPECIAL,
                "zoner" if dist > 115 => return forward | SPECIAL,
                "shoto" if dist > 150 => return forward | SPECIAL,
                "rushdown" if (95..240).contains(&dist) => return forward | SPECIAL,
                _ => {}
            }
        }
        if dist < 100 {
            return forward | NORMAL | if b.y < a.y - 50 * SCALE { UP } else { 0 };
        }
    }
    if dist > 58 {
        forward
    } else {
        0
    }
}
fn enter(a: &mut Actor, id: u16, duration: u16) {
    a.state.current_state = id;
    a.state.frame = 0;
    a.state.instance_duration = duration;
    a.state.hit_confirmed = false;
    a.state.block_confirmed = false;
    a.connected = false;
}
fn reward(actor: &mut Actor, def: &Definition, state_id: u16, trigger: u8) {
    let p = def.pack.view();
    if let (Some(extra), Some(deltas)) = (
        p.state_extras().and_then(|s| s.get(state_id as usize)),
        p.move_resource_deltas(),
    ) {
        let (offset, count) = extra.resource_deltas();
        for i in 0..usize::from(count) {
            if let Some(delta) = deltas.get_at(offset, i) {
                if delta.trigger() == trigger
                    && p.string(delta.name_off(), delta.name_len()) == Some("meter")
                {
                    actor.state.resources[def.meter] =
                        (i64::from(actor.state.resources[def.meter]) + i64::from(delta.delta()))
                            .clamp(0, i64::from(def.meter_max)) as u16;
                }
            }
        }
    }
}

fn requested(a: &Actor, d: &Definition, pressed: u8) -> Option<usize> {
    if pressed & SPECIAL != 0 {
        return Some(if a.control & UP != 0 {
            ANTI
        } else if a.control & DOWN != 0 {
            SUPER
        } else if a.platform < 0 {
            AIR_S
        } else {
            SKILL
        });
    }
    if pressed & NORMAL != 0 {
        let role = d.role(&a.state);
        return Some(if a.platform < 0 {
            if a.control & UP != 0 {
                AIR_UP
            } else if a.control & DOWN != 0 {
                AIR_H
            } else if role == AIR_L && a.state.hit_confirmed {
                AIR_M
            } else if role == AIR_M && a.state.hit_confirmed {
                AIR_H
            } else if axis(a.control) != 0 {
                AIR_M
            } else {
                AIR_L
            }
        } else if a.control & UP != 0 {
            LAUNCH
        } else if a.control & DOWN != 0 {
            LOW
        } else if role == JAB && a.state.hit_confirmed {
            MID
        } else if role == MID && a.state.hit_confirmed || axis(a.control) != 0 {
            DRIVE
        } else {
            JAB
        });
    }
    if pressed & JUMP_BUTTON != 0 {
        return Some(JUMP);
    }
    None
}
fn neutral(a: &Actor) -> usize {
    if a.platform < 0 {
        JUMP
    } else if a.control & DOWN != 0 {
        CROUCH
    } else {
        IDLE
    }
}
fn advance(a: &mut Actor, d: &Definition, stats: &mut Stats, i: usize) -> bool {
    if a.respawn > 0 {
        a.respawn -= 1;
        return false;
    }
    a.invulnerable = a.invulnerable.saturating_sub(1);
    let role = d.role(&a.state);
    if role != STUN && !attacking(role) && axis(a.control) != 0 {
        a.facing = axis(a.control);
    }
    let buffered = a.buffer as usize;
    let allowed = !(buffered == JUMP && a.platform < 0 && a.jumps >= 2
        || buffered == ANTI && a.recovery_used);
    let request = if role == STUN {
        None
    } else if a.buffer_age > 0 && allowed {
        Some(d.ids[buffered])
    } else if role <= CROUCH {
        let n = neutral(a);
        (role != n).then_some(d.ids[n])
    } else {
        None
    };
    let before = a.state;
    let result = next_frame(
        &a.state,
        &d.pack.view(),
        &FrameInput {
            requested_state: request,
        },
    );
    a.state = result.state;
    let started = request == Some(a.state.current_state) && a.state.frame == 0;
    let mut super_started = false;
    if started {
        let next = d.role(&a.state);
        a.connected = false;
        if attacking(next) || next == JUMP {
            if attacking(role) {
                stats.cancels[i] += 1;
            }
            a.buffer_age = 0;
            a.serial += 1;
            stats.spent[i] +=
                u32::from(before.resources[d.meter].saturating_sub(a.state.resources[d.meter]));
            reward(a, d, a.state.current_state, 0);
            if next == JUMP {
                if a.platform > 0 && a.control & DOWN != 0 {
                    a.drop_timer = 12;
                    a.y += SCALE;
                    a.vy = 2 * SCALE;
                    a.jumps = 1;
                } else {
                    a.vy = -d.jump;
                    a.jumps += 1;
                }
                a.platform = -1;
            } else if next == ANTI {
                a.vy = -d.rules[next].lift;
                a.platform = -1;
                a.recovery_used = true;
                a.jumps = a.jumps.max(1);
            }
            super_started = next == SUPER;
        }
    }
    if result.move_ended {
        if role == STUN {
            a.throw_immune = 8;
            a.combo = 0;
            a.combo_damage = 0;
        }
        enter(a, d.ids[neutral(a)], 0);
    }
    a.buffer_age = a.buffer_age.saturating_sub(1);
    a.throw_immune = a.throw_immune.saturating_sub(1);
    a.drop_timer = a.drop_timer.saturating_sub(1);
    let role = d.role(&a.state);
    if role != STUN {
        // Direct horizontal control; no opponent attraction, side locks or air body walls.
        a.vx = axis(a.control) * d.speed;
        if a.platform >= 0 && attacking(role) {
            a.vx = 0;
        }
    } else if a.platform >= 0 {
        a.vx = a.vx * 7 / 8;
    }
    a.x += a.vx;
    let mv = d
        .pack
        .view()
        .states()
        .unwrap()
        .get(a.state.current_state as usize)
        .unwrap();
    if attacking(role) && a.state.frame < u16::from(mv.startup()) + u16::from(mv.active()) {
        a.x += d.rules[role].travel * a.facing;
    }
    if a.platform >= 0 {
        let p = PLATFORMS[a.platform as usize];
        if a.x < p.left * SCALE || a.x > p.right * SCALE {
            a.platform = -1;
            a.jumps = a.jumps.max(1);
        }
    }
    if a.platform < 0 {
        let old_y = a.y;
        a.y += a.vy;
        a.vy = (a.vy
            + d.gravity
            + if a.control & DOWN != 0 && a.vy > 0 && role != STUN {
                d.gravity
            } else {
                0
            })
        .min(24 * SCALE);
        if a.vy >= 0 {
            for k in (0..PLATFORMS.len()).rev() {
                let p = PLATFORMS[k];
                let top = p.top * SCALE;
                if k > 0 && a.drop_timer > 0 {
                    continue;
                }
                if old_y <= top && a.y >= top && a.x >= p.left * SCALE && a.x <= p.right * SCALE {
                    a.y = top;
                    a.vy = 0;
                    a.platform = k as i8;
                    a.jumps = 0;
                    a.recovery_used = false;
                    if role != STUN {
                        enter(a, d.ids[LAND], 0);
                    }
                    break;
                }
            }
        }
    }
    super_started
}
fn spawn_notes(w: &mut World, defs: &[Definition; 2], i: usize) {
    let a = w.actors[i];
    let def = &defs[i];
    let pack = def.pack.view();
    let role = def.role(&a.state);
    let rule = def.rules[role];
    if let (Some(extra), Some(notes), Some(emits)) = (
        pack.state_extras()
            .and_then(|e| e.get(a.state.current_state as usize)),
        pack.move_notifies(),
        pack.event_emits(),
    ) {
        let (off, count) = extra.notifies();
        for j in 0..usize::from(count) {
            let note = notes.get_at(off, j).unwrap();
            if note.frame() != a.state.frame {
                continue;
            }
            let (eoff, ecount) = note.emits();
            for k in 0..usize::from(ecount) {
                let event = emits.get_at(eoff, k).unwrap();
                w.stats.signals[i] += 1;
                if pack.string(event.id_off(), event.id_len()) != Some("projectile")
                    || !rule.projectile
                {
                    continue;
                }
                if let Some(q) = w.projectiles.iter_mut().find(|p| !p.live) {
                    let mv = pack
                        .states()
                        .unwrap()
                        .get(a.state.current_state as usize)
                        .unwrap();
                    let hit = pack
                        .hit_windows()
                        .unwrap()
                        .get_at(mv.hit_windows_off(), 0)
                        .unwrap();
                    let shape = pack.shapes().unwrap().get_at(hit.shapes_off(), 0).unwrap();
                    let rect = rectangle(shape, a.x / SCALE, a.y / SCALE, a.facing);
                    *q = Projectile {
                        live: true,
                        owner: i,
                        role,
                        serial: a.serial,
                        x: rect.x * SCALE,
                        y: rect.y * SCALE,
                        vx: rule.projectile_speed * a.facing,
                        vy: rule.projectile_y,
                        w: rect.w,
                        h: rect.h,
                        facing: a.facing,
                        life: 220,
                    };
                    w.stats.shots[i] += 1;
                }
            }
        }
    }
}
struct ContactHit {
    hit: HitResult,
    owner: usize,
    serial: u32,
    role: usize,
    facing: i32,
    counter: bool,
    kind: u8,
}
fn strike(w: &mut World, defs: &[Definition; 2], c: ContactHit) {
    let i = c.owner;
    let j = 1 - i;
    let rule = defs[i].rules[c.role];
    let hit = c.hit;
    if w.actors[j].invulnerable > 0 || w.actors[j].respawn > 0 {
        return;
    }
    let airborne = w.actors[j].platform < 0;
    let combo = if defs[j].role(&w.actors[j].state) == STUN {
        w.actors[j].combo.saturating_add(1)
    } else {
        1
    };
    let raw = i32::from(hit.damage) + if c.counter { defs[i].counter_bonus } else { 0 };
    let damage = (raw * (100 - i32::from(combo.saturating_sub(1)) * 12).max(35) / 400).max(1);
    if w.actors[i].serial == c.serial && w.actors[i].state.current_state == hit.attacker_move {
        report_hit(&mut w.actors[i].state);
    }
    w.stats.hits[i] += 1;
    w.stats.max_combo[i] = w.stats.max_combo[i].max(combo);
    if airborne {
        w.stats.air_hits[i] += 1;
    }
    if rule.grab {
        w.stats.throws[i] += 1;
    }
    reward(&mut w.actors[i], &defs[i], hit.attacker_move, 1);
    let b = &mut w.actors[j];
    b.damage = (b.damage + damage).min(999);
    b.combo = combo;
    b.combo_damage = if combo > 1 {
        b.combo_damage + damage
    } else {
        damage
    };
    let power = ((3 * SCALE
        + raw * SCALE / 24
        + b.damage * SCALE / 16
        + if rule.knockdown { SCALE } else { 0 })
        * 100
        / defs[j].weight)
        .min(28 * SCALE);
    b.vx = power * c.facing;
    b.vy = -(power * 2 / 3 + (-rule.launch).max(0) / 3 + SCALE);
    b.platform = -1;
    b.jumps = b.jumps.max(1);
    b.y -= 1;
    enter(
        b,
        defs[j].ids[STUN],
        (u16::from(hit.hitstun) / 2 + b.damage as u16 / 12).clamp(8, 42),
    );
    w.hitstop = w.hitstop.max(hit.hitstop.min(8));
    w.contact = Contact {
        tick: w.tick,
        who: i,
        damage,
        counter: c.counter,
        serial: w.contact.serial + 1,
        x: b.x / SCALE,
        y: b.y / SCALE - 50,
        kind: c.kind,
    };
}
fn contacts(w: &mut World, defs: &[Definition; 2]) {
    let hits = std::array::from_fn::<_, 2, _>(|i| {
        let a = w.actors[i];
        let b = w.actors[1 - i];
        let role = defs[i].role(&a.state);
        let rule = defs[i].rules[role];
        if a.connected || rule.projectile || a.respawn > 0 || b.respawn > 0 || b.invulnerable > 0 {
            return None;
        }
        if rule.grab && (b.throw_immune > 0 || defs[1 - i].role(&b.state) == STUN) {
            return None;
        }
        let hit = check_hits(
            &a.state,
            &defs[i].pack.view(),
            (a.x / SCALE * a.facing, a.y / SCALE),
            &b.state,
            &defs[1 - i].pack.view(),
            (b.x / SCALE * a.facing, b.y / SCALE),
        )
        .get(0)
        .copied()?;
        Some(ContactHit {
            hit,
            owner: i,
            serial: a.serial,
            role,
            facing: a.facing,
            counter: attacking(defs[1 - i].role(&b.state)),
            kind: if rule.grab { 2 } else { 0 },
        })
    });
    // Both melee contacts are planned before either is applied, preserving trades.
    for (i, c) in hits.into_iter().enumerate() {
        if let Some(c) = c {
            w.actors[i].connected = true;
            strike(w, defs, c);
        }
    }
    for k in 0..PROJECTILES {
        if !w.projectiles[k].live {
            continue;
        }
        let q = &mut w.projectiles[k];
        q.x += q.vx;
        q.y += q.vy;
        q.life = q.life.saturating_sub(1);
        if q.life == 0
            || q.x < BLAST[0] * SCALE
            || q.x > BLAST[2] * SCALE
            || q.y < BLAST[1] * SCALE
            || q.y > BLAST[3] * SCALE
        {
            q.live = false;
            continue;
        }
        for l in 0..k {
            if w.projectiles[l].live
                && w.projectiles[l].owner != w.projectiles[k].owner
                && aabb_overlap(&w.projectiles[l].bounds(), &w.projectiles[k].bounds())
            {
                w.projectiles[l].live = false;
                w.projectiles[k].live = false;
                w.stats.clashes += 1;
                break;
            }
        }
        if !w.projectiles[k].live {
            continue;
        }
        let q = w.projectiles[k];
        let j = 1 - q.owner;
        let b = w.actors[j];
        let role = defs[j].role(&b.state);
        if b.invulnerable > 0 || b.respawn > 0 || defs[j].rules[role].projectile_invuln {
            continue;
        }
        let p = defs[j].pack.view();
        let mv = p
            .states()
            .unwrap()
            .get(b.state.current_state as usize)
            .unwrap();
        let mut touched = false;
        for h in 0..usize::from(mv.hurt_windows_len()) {
            let win = p
                .hurt_windows()
                .unwrap()
                .get_at(mv.hurt_windows_off(), h)
                .unwrap();
            if !(u16::from(win.start_frame())..=u16::from(win.end_frame())).contains(&b.state.frame)
            {
                continue;
            }
            for s in 0..usize::from(win.shapes_len()) {
                let shape = p.shapes().unwrap().get_at(win.shapes_off(), s).unwrap();
                if aabb_overlap(
                    &q.bounds(),
                    &Aabb::from_shape(&shape, b.x / SCALE, b.y / SCALE),
                ) {
                    touched = true;
                }
            }
        }
        if touched {
            let p = defs[q.owner].pack.view();
            let id = defs[q.owner].ids[q.role];
            let mv = p.states().unwrap().get(id as usize).unwrap();
            let h = p
                .hit_windows()
                .unwrap()
                .get_at(mv.hit_windows_off(), 0)
                .unwrap();
            let hit = HitResult {
                attacker_move: id,
                window_index: 0,
                damage: h.damage(),
                chip_damage: h.chip_damage(),
                hitstun: h.hitstun(),
                blockstun: h.blockstun(),
                hitstop: h.hitstop(),
                guard: h.guard(),
                hit_pushback: h.hit_pushback_px(),
                block_pushback: h.block_pushback_px(),
            };
            w.projectiles[k].live = false;
            strike(
                w,
                defs,
                ContactHit {
                    hit,
                    owner: q.owner,
                    serial: q.serial,
                    role: q.role,
                    facing: q.facing,
                    counter: attacking(role),
                    kind: 1,
                },
            );
        }
    }
}
fn step_world(w: &mut World, defs: &[Definition; 2], input: u8) {
    if w.winner != 0 {
        return;
    }
    w.tick += 1;
    let cpu = if w.hitstop == 0 {
        bot(w, defs)
    } else {
        w.actors[1].control
    };
    for (i, control) in [input, cpu].into_iter().enumerate() {
        let a = &mut w.actors[i];
        let pressed = control & !a.previous_input;
        a.control = control;
        if a.respawn == 0 {
            if let Some(role) = requested(a, &defs[i], pressed) {
                a.buffer = role as u8;
                a.buffer_age = 8;
            }
        }
        a.previous_input = control;
    }
    if w.hitstop > 0 {
        w.hitstop -= 1;
    } else {
        for (i, d) in defs.iter().enumerate() {
            if advance(&mut w.actors[i], d, &mut w.stats, i) {
                w.hitstop = 10;
                w.super_tick = w.tick;
                w.super_who = i;
            }
            if w.actors[i].respawn == 0 {
                spawn_notes(w, defs, i);
            }
        }
        contacts(w, defs);
    }
    for (i, d) in defs.iter().enumerate() {
        let a = w.actors[i];
        if a.respawn == 0
            && (a.x < BLAST[0] * SCALE
                || a.x > BLAST[2] * SCALE
                || a.y < BLAST[1] * SCALE
                || a.y > BLAST[3] * SCALE)
        {
            w.stats.kos[1 - i] += 1;
            w.ko_tick = w.tick;
            w.ko_who = i;
            let stocks = a.stocks.saturating_sub(1);
            w.actors[i] = d.actor(if i == 0 { 260 } else { 540 }, if i == 0 { 1 } else { -1 });
            w.actors[i].stocks = stocks;
            w.actors[i].respawn = 60;
            w.actors[i].invulnerable = 90;
            for q in &mut w.projectiles {
                if q.owner == i {
                    q.live = false;
                }
            }
        }
    }
    if w.actors[0].stocks == 0 || w.actors[1].stocks == 0 || w.tick >= MATCH_FRAMES {
        let score = |a: Actor| i32::from(a.stocks) * 1000 - a.damage;
        w.winner = if score(w.actors[0]) > score(w.actors[1]) {
            1
        } else if score(w.actors[1]) > score(w.actors[0]) {
            2
        } else {
            3
        };
    }
}
#[derive(Serialize)]
pub struct Rect {
    x: i32,
    y: i32,
    w: u32,
    h: u32,
}
fn rectangle(shape: framesmith_fspack::ShapeView<'_>, x: i32, y: i32, facing: i32) -> Rect {
    let r = Aabb::from_shape(&shape, 0, 0);
    Rect {
        x: x + if facing == 1 { r.x } else { -r.x - r.w as i32 },
        y: y + r.y,
        w: r.w,
        h: r.h,
    }
}
#[derive(Serialize)]
pub struct FighterView<'a> {
    name: &'a str,
    style: &'a str,
    special: &'a str,
    anti: &'a str,
    super_name: &'a str,
    width: i32,
    height: i32,
    facing: i32,
    y: f64,
    crouching: bool,
    combo: u8,
    combo_damage: i32,
    color: &'a str,
    x: f64,
    damage: i32,
    stocks: u8,
    platform: i8,
    jumps_remaining: u8,
    recovery_ready: bool,
    respawn: u8,
    invulnerable: u8,
    vx: f64,
    vy: f64,
    meter: u16,
    max_meter: u16,
    id: &'a str,
    name_of_move: &'a str,
    frame: u16,
    total: u16,
    startup: u8,
    active: u8,
    phase: &'static str,
    confirmed: bool,
    hitboxes: Vec<Rect>,
    hurtboxes: Vec<Rect>,
    reach: i32,
}
#[derive(Serialize)]
pub struct View<'a> {
    tick: u32,
    remaining: u32,
    platforms: [Platform; 4],
    blast: [i32; 4],
    ko_tick: u32,
    ko_who: usize,
    super_tick: u32,
    super_who: usize,
    projectiles: Vec<ShotView>,
    winner: u8,
    hitstop: u8,
    rng: u32,
    mode: u8,
    actors: [FighterView<'a>; 2],
    stats: Stats,
    contact: Contact,
    recorded: usize,
    checkpoint: u32,
}
fn fighter_view<'a>(a: &Actor, def: &'a Definition, facing: i32) -> FighterView<'a> {
    let pack = def.pack.view();
    let mv = pack
        .states()
        .unwrap()
        .get(a.state.current_state as usize)
        .unwrap();
    let data = pack.state_data(a.state.current_state as usize).unwrap();
    let role = def.role(&a.state);
    let phase = if role == IDLE || role == CROUCH || role == JUMP || role == LAND {
        "ready"
    } else if role == BLOCK {
        "guard"
    } else if role == STUN {
        "stun"
    } else if a.state.frame < u16::from(mv.startup()) {
        "startup"
    } else if a.state.frame < u16::from(mv.startup()) + u16::from(mv.active()) {
        "active"
    } else {
        "recovery"
    };
    let mut hitboxes = Vec::new();
    let mut hurtboxes = Vec::new();
    let mut reach = 0;
    let shapes = pack.shapes().unwrap();
    if let Some(windows) = pack.hit_windows() {
        for i in 0..usize::from(mv.hit_windows_len()) {
            let window = windows.get_at(mv.hit_windows_off(), i).unwrap();
            for j in 0..usize::from(window.shapes_len()) {
                let shape = shapes.get_at(window.shapes_off(), j).unwrap();
                reach = reach.max(shape.x_px() + shape.width_px() as i32);
                if (u16::from(window.start_frame())..=u16::from(window.end_frame()))
                    .contains(&a.state.frame)
                    && !def.rules[role].projectile
                {
                    hitboxes.push(rectangle(shape, a.x / SCALE, a.y / SCALE, facing));
                }
            }
        }
    }
    if let Some(windows) = pack.hurt_windows() {
        for i in 0..usize::from(mv.hurt_windows_len()) {
            let window = windows.get_at(mv.hurt_windows_off(), i).unwrap();
            if (u16::from(window.start_frame())..=u16::from(window.end_frame()))
                .contains(&a.state.frame)
            {
                for j in 0..usize::from(window.shapes_len()) {
                    hurtboxes.push(rectangle(
                        shapes.get_at(window.shapes_off(), j).unwrap(),
                        a.x / SCALE,
                        a.y / SCALE,
                        facing,
                    ));
                }
            }
        }
    }
    FighterView {
        name: &def.name,
        style: &def.style,
        special: pack
            .state_data(def.ids[SKILL] as usize)
            .unwrap()
            .get("name")
            .and_then(ValueView::as_str)
            .unwrap(),
        anti: pack
            .state_data(def.ids[ANTI] as usize)
            .unwrap()
            .get("name")
            .and_then(ValueView::as_str)
            .unwrap(),
        super_name: pack
            .state_data(def.ids[SUPER] as usize)
            .unwrap()
            .get("name")
            .and_then(ValueView::as_str)
            .unwrap(),
        width: def.width,
        height: def.height,
        facing: a.facing,
        y: f64::from(a.y) / f64::from(SCALE),
        crouching: matches!(role, CROUCH | LOW | LAUNCH),
        combo: a.combo,
        combo_damage: a.combo_damage,
        color: &def.color,
        x: f64::from(a.x) / f64::from(SCALE),
        damage: a.damage,
        stocks: a.stocks,
        platform: a.platform,
        jumps_remaining: 2 - a.jumps,
        recovery_ready: !a.recovery_used,
        respawn: a.respawn,
        invulnerable: a.invulnerable,
        vx: f64::from(a.vx) / f64::from(SCALE),
        vy: f64::from(a.vy) / f64::from(SCALE),
        meter: a.state.resources[def.meter],
        max_meter: def.meter_max,
        id: pack.state_id(a.state.current_state as usize).unwrap(),
        name_of_move: data.get("name").and_then(ValueView::as_str).unwrap(),
        frame: a.state.frame,
        total: if a.state.instance_duration > 0 {
            a.state.instance_duration
        } else {
            mv.total()
        },
        startup: mv.startup(),
        active: mv.active(),
        phase,
        confirmed: a.state.hit_confirmed,
        hitboxes,
        hurtboxes,
        reach,
    }
}

#[derive(Serialize)]
pub struct ShotView {
    owner: usize,
    x: i32,
    y: i32,
    w: u32,
    h: u32,
    super_shot: bool,
}
pub struct Game {
    defs: [Definition; 2],
    world: World,
    base: World,
    // ponytail: exact per-frame states, bounded by a three-minute match; stream traces for longer games.
    tape: Vec<(u8, World)>,
}
impl Game {
    pub fn new(player: &[u8], opponent: &[u8], seed: u32, mode: u8) -> Result<Self, String> {
        if mode > 1 {
            return Err("Unknown opponent mode".into());
        }
        let defs = [Definition::new(player)?, Definition::new(opponent)?];
        let world = World {
            actors: [defs[0].actor(260, 1), defs[1].actor(540, -1)],
            projectiles: [Projectile::default(); PROJECTILES],
            ko_tick: 0,
            ko_who: 0,
            super_tick: 0,
            super_who: 0,
            tick: 0,
            rng: seed,
            mode,
            ai_wait: 25,
            hitstop: 0,
            winner: 0,
            stats: Stats::default(),
            contact: Contact::default(),
        };
        Ok(Self {
            defs,
            world,
            base: world,
            tape: Vec::with_capacity(MATCH_FRAMES as usize),
        })
    }
    pub fn step(&mut self, input: u16) -> Result<(), String> {
        if input > INPUT_BITS {
            return Err("Unknown input bits".into());
        }
        if self.world.winner == 0 {
            step_world(&mut self.world, &self.defs, input as u8);
            self.tape.push((input as u8, self.world));
        }
        Ok(())
    }
    pub fn view(&self) -> View<'_> {
        View {
            tick: self.world.tick,
            remaining: MATCH_FRAMES.saturating_sub(self.world.tick),
            platforms: PLATFORMS,
            blast: BLAST,
            ko_tick: self.world.ko_tick,
            ko_who: self.world.ko_who,
            super_tick: self.world.super_tick,
            super_who: self.world.super_who,
            projectiles: self
                .world
                .projectiles
                .iter()
                .filter(|p| p.live)
                .map(|p| ShotView {
                    owner: p.owner,
                    x: p.x / SCALE,
                    y: p.y / SCALE,
                    w: p.w,
                    h: p.h,
                    super_shot: p.role == SUPER,
                })
                .collect(),
            winner: self.world.winner,
            hitstop: self.world.hitstop,
            rng: self.world.rng,
            mode: self.world.mode,
            actors: [
                fighter_view(
                    &self.world.actors[0],
                    &self.defs[0],
                    self.world.actors[0].facing,
                ),
                fighter_view(
                    &self.world.actors[1],
                    &self.defs[1],
                    self.world.actors[1].facing,
                ),
            ],
            stats: self.world.stats,
            contact: self.world.contact,
            recorded: self.tape.len(),
            checkpoint: self.base.tick,
        }
    }
    pub fn checkpoint(&mut self) {
        self.base = self.world;
        self.tape.clear();
    }
    pub fn restore(&mut self) {
        self.world = self.base;
        self.tape.clear();
    }
    pub fn verify_replay(&self) -> Result<usize, String> {
        let mut replay = self.base;
        for (index, (input, expected)) in self.tape.iter().enumerate() {
            step_world(&mut replay, &self.defs, *input);
            if replay != *expected {
                return Err(format!(
                    "Replay divergence at frame {} / input {}: expected {:?}, actual {:?}",
                    index + 1,
                    input,
                    expected,
                    replay
                ));
            }
        }
        if replay != self.world {
            return Err("Live state diverged from recorded timeline".into());
        }
        Ok(self.tape.len())
    }
    pub fn pack_info(&self) -> [(usize, u32, usize); 2] {
        self.defs.each_ref().map(|d| {
            (
                d.bytes,
                d.pack.view().version(),
                d.pack.view().states().unwrap().len(),
            )
        })
    }
}

#[cfg(target_arch = "wasm32")]
mod wasm {
    use super::*;
    use wasm_bindgen::prelude::*;
    fn integer(v: f64, max: u32) -> Result<u32, JsValue> {
        if !v.is_finite() || v.fract() != 0.0 || v < 0.0 || v > f64::from(max) {
            return Err(JsValue::from_str("Expected a bounded unsigned integer"));
        }
        Ok(v as u32)
    }
    fn js<T: Serialize>(value: &T) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(value).map_err(|e| JsValue::from_str(&e.to_string()))
    }
    #[wasm_bindgen]
    pub struct Arena {
        game: Game,
    }
    #[wasm_bindgen]
    impl Arena {
        #[wasm_bindgen(constructor)]
        pub fn new(player: &[u8], opponent: &[u8], seed: f64, mode: f64) -> Result<Arena, JsValue> {
            let game = Game::new(
                player,
                opponent,
                integer(seed, u32::MAX)?,
                integer(mode, 1)? as u8,
            )
            .map_err(|e| JsValue::from_str(&e))?;
            Ok(Self { game })
        }
        pub fn step(&mut self, input: f64) -> Result<JsValue, JsValue> {
            self.game
                .step(integer(input, u32::from(INPUT_BITS))? as u16)
                .map_err(|e| JsValue::from_str(&e))?;
            js(&self.game.view())
        }
        pub fn view(&self) -> Result<JsValue, JsValue> {
            js(&self.game.view())
        }
        pub fn checkpoint(&mut self) -> Result<JsValue, JsValue> {
            self.game.checkpoint();
            self.view()
        }
        pub fn restore(&mut self) -> Result<JsValue, JsValue> {
            self.game.restore();
            self.view()
        }
        pub fn verify_replay(&self) -> Result<usize, JsValue> {
            self.game.verify_replay().map_err(|e| JsValue::from_str(&e))
        }
        pub fn pack_info(&self) -> Result<JsValue, JsValue> {
            js(&self.game.pack_info())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn pack(id: &str) -> Vec<u8> {
        std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("dist/packs/{id}.fspk")),
        )
        .expect("run build.py --packs-only")
    }
    fn game(a: &str, b: &str, mode: u8) -> Game {
        Game::new(&pack(a), &pack(b), 0xf5a17, mode).unwrap()
    }
    fn tick(g: &mut Game, input: u8, n: usize) {
        for _ in 0..n {
            g.step(u16::from(input)).unwrap();
        }
    }
    fn approach(g: &mut Game, gap: i32) {
        for _ in 0..120 {
            if g.world.actors[1].x - g.world.actors[0].x <= gap * SCALE {
                break;
            }
            tick(g, RIGHT, 1);
        }
        tick(g, 0, 1);
    }
    fn settle(g: &mut Game) {
        for _ in 0..100 {
            if g.world.actors[0].platform >= 0 {
                break;
            }
            tick(g, 0, 1);
        }
    }
    #[test]
    fn free_movement_crossovers_double_jump_and_platforms() {
        for name in ["relay", "bulwark", "sable", "zip"] {
            let mut g = game(name, "bulwark", 1);
            let x = g.world.actors[0].x;
            let other = g.world.actors[1].x;
            tick(&mut g, LEFT, 10);
            assert!(g.world.actors[0].x < x);
            assert_eq!(g.world.actors[1].x, other);
            let x = g.world.actors[0].x;
            tick(&mut g, 0, 10);
            assert_eq!(g.world.actors[0].x, x);
            tick(&mut g, RIGHT, 95);
            assert!(g.world.actors[0].x > other, "{name}");
            assert_eq!(g.world.actors[1].x, other);
            assert!(g.verify_replay().is_ok());
            let mut g = game(name, "relay", 1);
            tick(&mut g, JUMP_BUTTON, 1);
            tick(&mut g, 0, 8);
            tick(&mut g, JUMP_BUTTON, 1);
            assert_eq!(g.world.actors[0].jumps, 2);
            assert!(g.world.actors[0].vy < 0);
            tick(&mut g, 0, 1);
            let vy = g.world.actors[0].vy;
            tick(&mut g, JUMP_BUTTON, 1);
            assert!(g.world.actors[0].vy > vy, "no third jump");
            settle(&mut g);
            assert_eq!(g.world.actors[0].platform, 1, "{name}");
            assert_eq!(g.world.actors[0].jumps, 0);
            tick(&mut g, 0, 5);
            tick(&mut g, DOWN | JUMP_BUTTON, 1);
            assert_eq!(g.world.actors[0].platform, -1);
            settle(&mut g);
            assert_eq!(g.world.actors[0].platform, 0);
            assert!(g.verify_replay().is_ok());
        }
        let mut g = game("relay", "bulwark", 1);
        for expected in [1, 3, 2] {
            tick(&mut g, JUMP_BUTTON, 1);
            if expected != 1 {
                let target = if expected == 3 {
                    400 * SCALE
                } else {
                    540 * SCALE
                };
                for _ in 0..70 {
                    if g.world.actors[0].x >= target {
                        break;
                    }
                    tick(&mut g, RIGHT, 1);
                }
            }
            settle(&mut g);
            assert_eq!(g.world.actors[0].platform, expected, "{:?}", g.world);
            tick(&mut g, 0, 5);
        }
    }
    #[test]
    fn percentage_knockback_and_three_stocks() {
        let mut g = game("relay", "bulwark", 1);
        approach(&mut g, 55);
        tick(&mut g, NORMAL, 1);
        tick(&mut g, 0, 12);
        assert!(g.world.actors[1].damage > 0);
        assert!(g.world.actors[1].vx > 0);
        assert_eq!(g.world.actors[1].stocks, 3);
        let mut falls = game("relay", "bulwark", 1);
        for _ in 0..900 {
            tick(&mut falls, LEFT, 1);
            if falls.world.winner != 0 {
                break;
            }
        }
        assert_eq!(falls.world.actors[0].stocks, 0);
        assert_eq!(falls.world.winner, 2);
        assert_eq!(falls.world.stats.kos[1], 3);
        assert!(falls.verify_replay().is_ok());
        let end = falls.world;
        tick(&mut falls, NORMAL, 1);
        assert_eq!(falls.world, end);
    }
    #[test]
    fn projectile_checkpoint_recovery_and_invalid_inputs() {
        let mut g = game("sable", "relay", 1);
        tick(&mut g, SPECIAL, 1);
        tick(&mut g, 0, 18);
        assert!(g.world.projectiles.iter().any(|p| p.live));
        g.checkpoint();
        let saved = g.world;
        tick(&mut g, 0, 90);
        let end = g.world;
        assert!(g.world.stats.hits[0] > 0);
        assert!(g.verify_replay().is_ok());
        g.restore();
        assert_eq!(g.world, saved);
        tick(&mut g, 0, 90);
        assert_eq!(g.world, end);
        let before = g.world;
        assert!(g.step(128).is_err());
        assert_eq!(g.world, before);
        for name in ["relay", "bulwark", "sable", "zip"] {
            let mut g = game(name, "relay", 1);
            tick(&mut g, LEFT, 45);
            tick(&mut g, 0, 1);
            tick(&mut g, UP | SPECIAL, 1);
            assert!(g.world.actors[0].recovery_used, "{name}");
            assert!(g.world.actors[0].vy < 0);
            let count = g.world.actors[0].serial;
            tick(&mut g, 0, 50);
            tick(&mut g, UP | SPECIAL, 1);
            assert_eq!(
                g.world.actors[0].serial, count,
                "recovery cannot loop: {name}"
            );
        }
    }
    #[test]
    fn all_matchups_use_real_binary_moves_and_replay() {
        let mut count = 0;
        for name in ["relay", "bulwark", "sable", "zip"] {
            for rival in ["relay", "bulwark", "sable", "zip"] {
                let mut g = game(name, rival, 0);
                assert_eq!(g.pack_info()[0].2, 19);
                for n in 0..1800 {
                    let a = g.world.actors[0];
                    let b = g.world.actors[1];
                    let dir = direction(a.x, b.x);
                    let input = if a.platform < 0 && (a.x < 150 * SCALE || a.x > 650 * SCALE) {
                        direction(a.x, 400 * SCALE)
                            | if a.jumps < 2 && n % 18 == 0 {
                                JUMP_BUTTON
                            } else if !a.recovery_used && a.vy > 0 {
                                UP | SPECIAL
                            } else {
                                0
                            }
                    } else if n % 24 == 0 {
                        dir | SPECIAL
                    } else if n % 12 == 0 {
                        dir | NORMAL
                    } else if (a.x - b.x).abs() > 65 * SCALE {
                        dir
                    } else {
                        0
                    };
                    tick(&mut g, input, 1);
                    if g.world.winner != 0 {
                        break;
                    }
                }
                assert!(g.world.stats.hits.iter().sum::<u32>() > 0, "{name}/{rival}");
                assert!(g
                    .world
                    .actors
                    .iter()
                    .all(|a| a.damage >= 0 && a.stocks <= STOCKS));
                assert!(g.verify_replay().is_ok());
                count += 1;
            }
        }
        assert_eq!(count, 16);
    }
}
