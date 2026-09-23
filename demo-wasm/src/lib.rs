//! A small game consumer. Combat policy stays here, not in the helper crates.
use framesmith_fspack::{payload::ValueView, OwnedPack};
use framesmith_runtime::{
    aabb_overlap, check_hits, check_pushbox, init_resources, next_frame, report_block, report_hit,
    Aabb, CharacterState, FrameInput, HitResult,
};
use serde::Serialize;
const LEFT: u8 = 1;
const RIGHT: u8 = 2;
const LIGHT: u8 = 4;
const MEDIUM: u8 = 8;
const HEAVY: u8 = 16;
const SPECIAL: u8 = 32;
const UP: u8 = 64;
const DOWN: u8 = 128;
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
const IDS: [&str; 18] = [
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
];
const HZ: u32 = 60; // Demo policy, not a FrameSmith clock requirement.
const ROUND_FRAMES: u32 = HZ * 60;
const MATCH_FRAMES: u32 = ROUND_FRAMES * 3 + 180;
const SCALE: i32 = 256;
const WALL_L: i32 = 72 * SCALE;
const WALL_R: i32 = 928 * SCALE;
const PROJECTILES: usize = 8;
fn attacking(role: usize) -> bool {
    (JAB..=SUPER).contains(&role) || role == AIR_S
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
    chip: u16,
    grab: bool,
    knockdown: bool,
    projectile_invuln: bool,
}
struct Definition {
    pack: OwnedPack,
    ids: [u16; 18],
    rules: [MoveRule; 18],
    name: String,
    color: String,
    style: String,
    health: i32,
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
        let health = required_number(props, "health", 1., 60000.)? as i32;
        let speed = (required_number(props, "walk_speed", 0.25, 8.)? * SCALE as f64) as i32;
        let jump = (required_number(props, "jump_speed", 4., 14.)? * SCALE as f64) as i32;
        let gravity = (required_number(props, "gravity", 0.25, 1.)? * SCALE as f64) as i32;
        let width = required_number(props, "width", 20., 64.)? as i32;
        let height = required_number(props, "height", 60., 120.)? as i32;
        let counter_bonus = required_number(props, "counter_bonus", 0., 100.)? as i32;
        let states = view.states().ok_or("Missing compiled states")?;
        let mut ids = [0; 18];
        let mut rules = [MoveRule::default(); 18];
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
                chip: n("chip", 0., 100.)? as u16,
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
            health,
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
            health: self.health,
            facing,
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
    health: i32,
    previous_input: u8,
    control: u8,
    buffer: u8,
    buffer_age: u8,
    connected: bool,
    blocked_stun: bool,
    serial: u32,
    combo: u8,
    combo_damage: i32,
    juggles: u8,
    throw_immune: u8,
    knockdown: bool,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
struct Stats {
    hits: [u32; 2],
    blocks: [u32; 2],
    cancels: [u32; 2],
    spent: [u32; 2],
    signals: [u32; 2],
    shots: [u32; 2],
    throws: [u32; 2],
    air_hits: [u32; 2],
    max_combo: [u8; 2],
    clashes: u32,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
struct Contact {
    tick: u32,
    who: usize,
    damage: i32,
    blocked: bool,
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
    round_tick: u32,
    round: u8,
    wins: [u8; 2],
    round_winner: u8,
    next_round: u8,
    rng: u32,
    mode: u8,
    ai_wait: u16,
    ai_guard: u8,
    hitstop: u8,
    winner: u8,
    super_tick: u32,
    super_who: usize,
    stats: Stats,
    contact: Contact,
}
fn rng(w: &mut World) -> u32 {
    w.rng = w.rng.wrapping_mul(1664525).wrapping_add(1013904223);
    w.rng
}
fn axis(control: u8) -> i32 {
    i32::from(control & RIGHT != 0) - i32::from(control & LEFT != 0)
}
fn toward(a: &Actor) -> u8 {
    if a.facing == 1 {
        RIGHT
    } else {
        LEFT
    }
}
fn away(a: &Actor) -> u8 {
    if a.facing == 1 {
        LEFT
    } else {
        RIGHT
    }
}
fn bot(w: &mut World, d: &[Definition; 2]) -> u8 {
    let a = w.actors[1];
    let p = w.actors[0];
    let role = d[1].role(&a.state);
    let dist = (a.x - p.x).abs() / SCALE;
    if w.mode == 1 {
        return 0;
    }
    if w.mode >= 2 {
        return away(&a) | if w.mode == 3 { DOWN } else { 0 };
    }
    if w.ai_guard > 0 {
        w.ai_guard -= 1;
        return away(&a) | if p.control & DOWN != 0 { DOWN } else { 0 };
    }
    if a.state.hit_confirmed || a.state.block_confirmed {
        return match role {
            JAB | LOW | AIR_L => MEDIUM,
            MID | AIR_M => HEAVY,
            DRIVE | AIR_H => SPECIAL,
            LAUNCH => UP,
            _ => 0,
        };
    }
    if role == JUMP {
        return if dist < 120 && a.y > -110 * SCALE {
            HEAVY
        } else {
            0
        };
    }
    if role > CROUCH {
        return 0;
    }
    w.ai_wait = w.ai_wait.saturating_sub(1);
    let danger = w
        .projectiles
        .iter()
        .any(|q| q.live && q.owner == 0 && (q.x - a.x).abs() < 190 * SCALE);
    if (danger || attacking(d[0].role(&p.state)) && dist < 150)
        && w.tick.is_multiple_of(9)
        && !rng(w).is_multiple_of(3)
    {
        if danger && d[1].style != "zoner" && rng(w).is_multiple_of(3) {
            return toward(&a) | UP;
        }
        w.ai_guard = 12;
        return away(&a) | if p.control & DOWN != 0 { DOWN } else { 0 };
    }
    if p.y < -30 * SCALE && dist < 130 && w.ai_wait == 0 {
        w.ai_wait = 20;
        return DOWN | SPECIAL;
    }
    if w.ai_wait == 0 {
        let choice = rng(w);
        w.ai_wait = 12 + (choice % 16) as u16;
        if (150..320).contains(&dist) && d[1].style != "zoner" && choice.is_multiple_of(4) {
            return toward(&a) | UP;
        }
        if a.state.resources[d[1].meter] >= 50
            && dist < if d[1].style == "grappler" { 70 } else { 280 }
            && choice.is_multiple_of(5)
        {
            return HEAVY | SPECIAL;
        }
        match d[1].style.as_str() {
            "grappler" if dist < 72 && p.y == 0 && d[0].role(&p.state) != STUN => return SPECIAL,
            "zoner" if dist > 185 => return SPECIAL,
            "shoto" if dist > 170 && choice.is_multiple_of(2) => return SPECIAL,
            "rushdown" if dist > 105 && dist < 260 => return SPECIAL,
            _ => {}
        }
        if dist < 90 {
            return if choice.is_multiple_of(4) {
                DOWN | LIGHT
            } else {
                LIGHT
            };
        }
        if dist < 150 {
            return MEDIUM;
        }
    }
    if d[1].style == "zoner" && dist < 210 && a.x > WALL_L + 30 * SCALE && a.x < WALL_R - 30 * SCALE
    {
        return away(&a);
    }
    if dist > 65 {
        toward(&a)
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

fn requested(a: &Actor, pressed: u8) -> Option<usize> {
    if pressed & (HEAVY | SPECIAL) != 0
        && a.control & (HEAVY | SPECIAL) == HEAVY | SPECIAL
        && a.y == 0
    {
        return Some(SUPER);
    }
    if pressed & SPECIAL != 0 {
        return Some(if a.y < 0 {
            AIR_S
        } else if a.control & DOWN != 0 {
            ANTI
        } else {
            SKILL
        });
    }
    if pressed & HEAVY != 0 {
        return Some(if a.y < 0 {
            AIR_H
        } else if a.control & DOWN != 0 {
            LAUNCH
        } else {
            DRIVE
        });
    }
    if pressed & MEDIUM != 0 {
        return Some(if a.y < 0 { AIR_M } else { MID });
    }
    if pressed & LIGHT != 0 {
        return Some(if a.y < 0 {
            AIR_L
        } else if a.control & DOWN != 0 {
            LOW
        } else {
            JAB
        });
    }
    if pressed & UP != 0 && a.control & DOWN == 0 && a.y == 0 {
        return Some(JUMP);
    }
    None
}
fn neutral(a: &Actor) -> usize {
    if a.y < 0 {
        JUMP
    } else if a.control & DOWN != 0 {
        CROUCH
    } else if axis(a.control) == -a.facing {
        BLOCK
    } else {
        IDLE
    }
}
fn advance(a: &mut Actor, d: &Definition, stats: &mut Stats, i: usize, stationary: bool) -> bool {
    let role = d.role(&a.state);
    let request = if role == STUN {
        None
    } else if a.buffer_age > 0 {
        Some(d.ids[a.buffer as usize])
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
    let mut super_started = false;
    if a.state.current_state != before.current_state {
        let next = d.role(&a.state);
        a.connected = false;
        a.blocked_stun = false;
        if attacking(next) || next == JUMP {
            if attacking(role) {
                stats.cancels[i] += 1;
            }
            a.buffer_age = 0;
            a.serial += 1;
            stats.spent[i] +=
                u32::from(before.resources[d.meter].saturating_sub(a.state.resources[d.meter]));
            reward(a, d, a.state.current_state, 0);
            let lift = if next == JUMP {
                d.jump
            } else {
                d.rules[next].lift
            };
            if lift > 0 && a.y == 0 {
                a.vy = -lift;
                a.y = -1;
                a.vx = axis(a.control) * d.speed;
            }
            super_started = next == SUPER;
        }
    }
    if result.move_ended {
        if role == STUN && a.y < 0 {
            enter(a, d.ids[STUN], 2);
        } else {
            if role == STUN {
                a.throw_immune = 8;
                a.knockdown = false;
                a.combo = 0;
                a.combo_damage = 0;
                a.juggles = 0;
            }
            enter(a, d.ids[neutral(a)], 0);
            a.blocked_stun = false;
        }
    }
    a.buffer_age = a.buffer_age.saturating_sub(1);
    a.throw_immune = a.throw_immune.saturating_sub(1);
    let role = d.role(&a.state);
    let mv = d
        .pack
        .view()
        .states()
        .unwrap()
        .get(a.state.current_state as usize)
        .unwrap();
    if a.y < 0 {
        a.x += a.vx;
        a.y += a.vy;
        a.vy += d.gravity;
        if a.y >= 0 {
            a.y = 0;
            a.vy = 0;
            a.vx = 0;
            if role == STUN {
                enter(a, d.ids[STUN], if a.knockdown { 16 } else { 6 });
            } else {
                enter(a, d.ids[LAND], 0);
            }
        }
    } else if role < CROUCH && !stationary {
        a.x += axis(a.control) * d.speed;
    }
    if attacking(role) && a.state.frame < u16::from(mv.startup()) + u16::from(mv.active()) {
        a.x += d.rules[role].travel * a.facing;
    }
    a.x = a.x.clamp(WALL_L, WALL_R);
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
fn can_block(a: &Actor, d: &Definition, guard: u8) -> bool {
    let role = d.role(&a.state);
    a.y == 0
        && (role <= CROUCH || role == STUN && a.blocked_stun)
        && axis(a.control) == -a.facing
        && match guard {
            0 => a.control & DOWN == 0,
            1 => true,
            2 => a.control & DOWN != 0,
            _ => false,
        }
}
#[derive(Clone, Copy)]
struct ContactHit {
    hit: HitResult,
    owner: usize,
    serial: u32,
    role: usize,
    facing: i32,
    blocked: bool,
    counter: bool,
    kind: u8,
}
fn strike(w: &mut World, defs: &[Definition; 2], c: ContactHit) {
    let i = c.owner;
    let j = 1 - i;
    let rule = defs[i].rules[c.role];
    let hit = c.hit;
    let airborne = w.actors[j].y < 0;
    let combo = if c.blocked {
        0
    } else if defs[j].role(&w.actors[j].state) == STUN && !w.actors[j].blocked_stun {
        w.actors[j].combo.saturating_add(1)
    } else {
        1
    };
    let raw = i32::from(hit.damage) + if c.counter { defs[i].counter_bonus } else { 0 };
    let damage = if c.blocked {
        i32::from(hit.chip_damage.max(rule.chip))
    } else {
        raw * (100 - i32::from(combo.saturating_sub(1)) * 12).max(35) / 100
    };
    if w.actors[i].serial == c.serial && w.actors[i].state.current_state == hit.attacker_move {
        if c.blocked {
            report_block(&mut w.actors[i].state);
        } else {
            report_hit(&mut w.actors[i].state);
        }
    }
    if c.blocked {
        w.stats.blocks[j] += 1;
    } else {
        w.stats.hits[i] += 1;
        w.stats.max_combo[i] = w.stats.max_combo[i].max(combo);
        if airborne {
            w.stats.air_hits[i] += 1;
        }
        if rule.grab {
            w.stats.throws[i] += 1;
        }
    }
    reward(
        &mut w.actors[i],
        &defs[i],
        hit.attacker_move,
        if c.blocked { 2 } else { 1 },
    );
    let b = &mut w.actors[j];
    b.health = (b.health - damage).max(0);
    b.combo = combo;
    b.combo_damage = if combo > 1 {
        b.combo_damage + damage
    } else {
        damage
    };
    enter(
        b,
        defs[j].ids[STUN],
        u16::from(if c.blocked {
            hit.blockstun
        } else {
            hit.hitstun
        })
        .max(1),
    );
    b.blocked_stun = c.blocked;
    b.x = (b.x
        + if c.blocked {
            hit.block_pushback
        } else {
            hit.hit_pushback
        } * SCALE
            * c.facing)
        .clamp(WALL_L, WALL_R);
    if !c.blocked && (rule.launch < 0 || airborne) {
        b.juggles = b.juggles.saturating_add(1);
        b.y = b.y.min(-1);
        b.vy = if b.juggles >= 5 {
            3 * SCALE
        } else if rule.launch < 0 {
            rule.launch
        } else {
            -4 * SCALE
        };
        b.vx = 2 * SCALE * c.facing;
    }
    b.knockdown = !c.blocked && (rule.knockdown || airborne);
    w.hitstop = w.hitstop.max(hit.hitstop);
    w.contact = Contact {
        tick: w.tick,
        who: i,
        damage,
        blocked: c.blocked,
        counter: c.counter && !c.blocked,
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
        if a.connected || rule.projectile || b.juggles >= 5 && b.y < 0 {
            return None;
        }
        if rule.grab && (b.y < 0 || b.throw_immune > 0 || defs[1 - i].role(&b.state) == STUN) {
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
            blocked: can_block(&b, &defs[1 - i], hit.guard),
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
        if q.life == 0 || q.x < 0 || q.x > 1000 * SCALE || q.y < -400 * SCALE {
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
        if b.juggles >= 5 && b.y < 0 || defs[j].rules[role].projectile_invuln {
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
                    blocked: can_block(&b, &defs[j], hit.guard),
                    counter: attacking(role),
                    kind: 1,
                },
            );
        }
    }
}
fn separate(w: &mut World, defs: &[Definition; 2]) {
    if let Some(push) = check_pushbox(
        &w.actors[0].state,
        &defs[0].pack.view(),
        (w.actors[0].x / SCALE, w.actors[0].y / SCALE),
        &w.actors[1].state,
        &defs[1].pack.view(),
        (w.actors[1].x / SCALE, w.actors[1].y / SCALE),
    ) {
        w.actors[0].x += push.p1_dx * SCALE;
        w.actors[1].x += push.p2_dx * SCALE;
        for i in 0..2 {
            let spill = w.actors[i].x - w.actors[i].x.clamp(WALL_L, WALL_R);
            w.actors[i].x -= spill;
            w.actors[1 - i].x -= spill;
        }
    }
    for (i, def) in defs.iter().enumerate() {
        w.actors[i].x = w.actors[i].x.clamp(WALL_L, WALL_R);
        if w.actors[i].y == 0 && !attacking(def.role(&w.actors[i].state)) {
            w.actors[i].facing = if w.actors[i].x <= w.actors[1 - i].x {
                1
            } else {
                -1
            };
        }
    }
}
fn step_world(w: &mut World, defs: &[Definition; 2], input: u8) {
    if w.winner != 0 {
        return;
    }
    w.tick += 1;
    if w.next_round > 0 {
        w.next_round -= 1;
        if w.next_round == 0 {
            w.actors = [defs[0].actor(330, 1), defs[1].actor(670, -1)];
            w.projectiles = [Projectile::default(); PROJECTILES];
            w.round += 1;
            w.round_tick = 0;
            w.round_winner = 0;
            w.ai_wait = 25;
            w.ai_guard = 0;
            w.hitstop = 0;
        }
        return;
    }
    let cpu = if w.hitstop == 0 {
        bot(w, defs)
    } else {
        w.actors[1].control
    };
    for (a, control) in w.actors.iter_mut().zip([input, cpu]) {
        let pressed = control & !a.previous_input;
        a.control = control;
        if let Some(role) = requested(a, pressed) {
            a.buffer = role as u8;
            a.buffer_age = 8;
        }
        a.previous_input = control;
    }
    w.round_tick += 1;
    if w.hitstop > 0 {
        w.hitstop -= 1;
    } else {
        for (i, d) in defs.iter().enumerate() {
            if advance(&mut w.actors[i], d, &mut w.stats, i, i == 1 && w.mode > 0) {
                w.hitstop = 12;
                w.super_tick = w.tick;
                w.super_who = i;
            }
            spawn_notes(w, defs, i);
        }
        separate(w, defs);
        contacts(w, defs);
    }
    if w.actors.iter().any(|a| a.health == 0) || w.round_tick >= ROUND_FRAMES {
        let p = i64::from(w.actors[0].health) * i64::from(defs[1].health);
        let c = i64::from(w.actors[1].health) * i64::from(defs[0].health);
        w.round_winner = if p > c {
            1
        } else if c > p {
            2
        } else {
            3
        };
        if w.round_winner != 2 {
            w.wins[0] += 1;
        }
        if w.round_winner != 1 {
            w.wins[1] += 1;
        }
        if w.wins[0] >= 2 || w.wins[1] >= 2 {
            w.winner = if w.wins[0] == w.wins[1] {
                3
            } else if w.wins[0] > w.wins[1] {
                1
            } else {
                2
            };
        } else {
            w.next_round = 90;
        }
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
    hp: i32,
    max_hp: i32,
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
    blocking: bool,
    hitboxes: Vec<Rect>,
    hurtboxes: Vec<Rect>,
    reach: i32,
}
#[derive(Serialize)]
pub struct View<'a> {
    tick: u32,
    remaining: u32,
    round: u8,
    wins: [u8; 2],
    round_winner: u8,
    next_round: u8,
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
        hp: a.health,
        max_hp: def.health,
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
        blocking: role == BLOCK || role == STUN && a.blocked_stun,
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
    // ponytail: exact per-frame states, bounded by three one-minute rounds; stream traces for longer games.
    tape: Vec<(u8, World)>,
}
impl Game {
    pub fn new(player: &[u8], opponent: &[u8], seed: u32, mode: u8) -> Result<Self, String> {
        if mode > 3 {
            return Err("Unknown opponent mode".into());
        }
        let defs = [Definition::new(player)?, Definition::new(opponent)?];
        let world = World {
            actors: [defs[0].actor(330, 1), defs[1].actor(670, -1)],
            projectiles: [Projectile::default(); PROJECTILES],
            round_tick: 0,
            round: 1,
            wins: [0; 2],
            round_winner: 0,
            next_round: 0,
            super_tick: 0,
            super_who: 0,
            tick: 0,
            rng: seed,
            mode,
            ai_wait: 25,
            ai_guard: 0,
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
        if input > 255 {
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
            remaining: ROUND_FRAMES.saturating_sub(self.world.round_tick),
            round: self.world.round,
            wins: self.world.wins,
            round_winner: self.world.round_winner,
            next_round: self.world.next_round,
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
                integer(mode, 3)? as u8,
            )
            .map_err(|e| JsValue::from_str(&e))?;
            Ok(Self { game })
        }
        pub fn step(&mut self, input: f64) -> Result<JsValue, JsValue> {
            self.game
                .step(integer(input, 255)? as u16)
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
    fn close(g: &mut Game) {
        for _ in 0..200 {
            if (g.world.actors[1].x - g.world.actors[0].x).abs()
                <= ((g.defs[0].width + g.defs[1].width) / 2 + 2) * SCALE
            {
                break;
            }
            tick(g, RIGHT, 1);
        }
        tick(g, 0, 1);
    }
    fn policy(g: &Game) -> u8 {
        let a = g.world.actors[0];
        let b = g.world.actors[1];
        let role = g.defs[0].role(&a.state);
        if a.state.hit_confirmed || a.state.block_confirmed {
            return match role {
                JAB | LOW | AIR_L => MEDIUM,
                MID | AIR_M => HEAVY,
                DRIVE => {
                    if a.state.resources[g.defs[0].meter] >= 50 {
                        HEAVY | SPECIAL
                    } else {
                        SPECIAL
                    }
                }
                _ => 0,
            };
        }
        if (a.x - b.x).abs() > 58 * SCALE {
            toward(&a)
        } else if role <= CROUCH {
            LIGHT
        } else {
            0
        }
    }
    #[test]
    fn four_binary_archetypes_and_all_matchups_replay() {
        for a in ["relay", "bulwark", "sable", "zip"] {
            for b in ["relay", "bulwark", "sable", "zip"] {
                let mut g = game(a, b, 0);
                assert_eq!(g.pack_info()[0].2, 18);
                for _ in 0..1200 {
                    let p = policy(&g);
                    tick(&mut g, p, 1);
                    for a in g.world.actors {
                        assert!((WALL_L..=WALL_R).contains(&a.x));
                        assert!(a.y <= 0);
                        assert!(a.health >= 0);
                    }
                }
                assert!(g.world.stats.hits.iter().sum::<u32>() > 0, "{a}/{b}");
                assert_eq!(g.verify_replay().unwrap(), g.tape.len());
            }
        }
    }
    #[test]
    fn light_medium_heavy_special_chain_and_first_to_two() {
        let mut g = game("relay", "bulwark", 1);
        for _ in 0..MATCH_FRAMES {
            let p = policy(&g);
            tick(&mut g, p, 1);
            if g.world.winner != 0 {
                break;
            }
        }
        assert_eq!(g.world.winner, 1, "{:?}", g.world);
        assert_eq!(g.world.wins[0], 2);
        assert!(g.world.stats.cancels[0] > 0);
        assert!(g.world.stats.spent[0] >= 50);
        assert!(g.world.stats.max_combo[0] >= 3);
        assert!(g.verify_replay().is_ok());
        let terminal = g.world;
        tick(&mut g, 255, 1);
        assert_eq!(g.world, terminal);
    }
    #[test]
    fn back_block_low_block_and_grab_counterplay() {
        let mut g = game("relay", "bulwark", 2);
        close(&mut g);
        tick(&mut g, LIGHT, 1);
        tick(&mut g, 0, 14);
        assert!(g.world.stats.blocks[1] > 0);
        assert_eq!(g.world.actors[1].health, g.defs[1].health);
        tick(&mut g, 0, 30);
        tick(&mut g, DOWN | LIGHT, 1);
        tick(&mut g, 0, 20);
        assert!(g.world.actors[1].health < g.defs[1].health);
        let mut low = game("relay", "bulwark", 3);
        close(&mut low);
        tick(&mut low, DOWN | LIGHT, 1);
        tick(&mut low, 0, 20);
        assert_eq!(low.world.actors[1].health, low.defs[1].health);
        assert!(low.world.stats.blocks[1] > 0);
        let mut grab = game("bulwark", "relay", 2);
        close(&mut grab);
        tick(&mut grab, SPECIAL, 1);
        tick(&mut grab, 0, 30);
        assert_eq!(grab.world.stats.throws[0], 1);
        assert!(grab.world.actors[1].health < grab.defs[1].health);
        assert!(grab.verify_replay().is_ok());
    }
    #[test]
    fn projectiles_are_real_traveling_entities_and_snapshot_in_flight() {
        let mut g = game("sable", "relay", 1);
        tick(&mut g, SPECIAL, 1);
        tick(&mut g, 0, 18);
        assert!(g.world.projectiles.iter().any(|p| p.live));
        assert_eq!(g.world.stats.hits[0], 0);
        g.checkpoint();
        let saved = g.world;
        tick(&mut g, 0, 90);
        assert!(g.world.stats.hits[0] > 0);
        let end = g.world;
        assert!(g.verify_replay().is_ok());
        g.restore();
        assert_eq!(g.world, saved);
        tick(&mut g, 0, 90);
        assert_eq!(g.world, end);
    }
    #[test]
    fn launch_jump_cancel_air_chain_and_cross_up() {
        let mut g = game("zip", "bulwark", 1);
        close(&mut g);
        tick(&mut g, DOWN | HEAVY, 1);
        for _ in 0..30 {
            if g.world.actors[0].state.hit_confirmed {
                break;
            }
            tick(&mut g, 0, 1);
        }
        assert!(g.world.actors[1].y < 0);
        tick(&mut g, UP | RIGHT, 1);
        tick(&mut g, RIGHT, 9); // Jump-cancel is buffered through impact hitstop.
        assert!(g.world.actors[0].y < 0);
        for n in 0..90 {
            let role = g.defs[0].role(&g.world.actors[0].state);
            let input = match role {
                JUMP => LIGHT,
                AIR_L if g.world.actors[0].state.hit_confirmed => MEDIUM,
                AIR_M if g.world.actors[0].state.hit_confirmed => HEAVY,
                _ => 0,
            };
            tick(&mut g, if n == 0 { 0 } else { input }, 1);
        }
        assert!(g.world.stats.air_hits[0] >= 2, "{:?}", g.world);
        assert!(g.verify_replay().is_ok());
        let mut x = game("zip", "relay", 1);
        close(&mut x);
        tick(&mut x, UP | RIGHT, 1);
        tick(&mut x, RIGHT, 55);
        assert!(
            x.world.actors[0].x > x.world.actors[1].x,
            "cross-up positions {:?}",
            x.world.actors
        );
        assert_eq!(x.world.actors[0].facing, -1);
        assert!(x.verify_replay().is_ok());
    }
    #[test]
    fn jumping_clears_a_real_cpu_projectile() {
        let mut g = game("relay", "sable", 0);
        let mut jumped = false;
        let mut cleared = false;
        let mut serial = 0;
        for _ in 0..900 {
            let shot = g
                .world
                .projectiles
                .iter()
                .find(|p| p.live && p.owner == 1 && (serial == 0 || p.serial == serial));
            let go =
                shot.is_some_and(|p| !jumped && (p.x - g.world.actors[0].x).abs() < 130 * SCALE);
            if go {
                serial = shot.unwrap().serial;
                jumped = true;
            }
            if jumped && shot.is_some_and(|p| p.x < g.world.actors[0].x) {
                cleared = true;
                break;
            }
            g.step(if go { u16::from(UP) } else { 0 }).unwrap();
        }
        assert!(
            cleared && g.world.actors[0].health == g.defs[0].health,
            "{:?}",
            g.world
        );
        let mut grounded = game("relay", "sable", 0);
        tick(&mut grounded, 0, g.world.tick as usize + 4);
        assert!(grounded.world.actors[0].health < grounded.defs[0].health);
        assert!(g.verify_replay().is_ok());
    }
    #[test]
    fn anti_air_catches_an_approaching_cpu_jump() {
        let mut g = game("zip", "relay", 0);
        let mut caught = false;
        for _ in 0..2400 {
            let a = g.world.actors[0];
            let b = g.world.actors[1];
            let distance = (a.x - b.x).abs() / SCALE;
            let toward = if a.x < b.x { RIGHT } else { LEFT };
            let input = if a.y == 0 && distance < 200 {
                toward | UP
            } else {
                toward
            };
            let hp = a.health;
            g.step(input.into()).unwrap();
            if g.defs[1].role(&g.world.actors[1].state) == ANTI && g.world.actors[0].health < hp {
                caught = true;
                break;
            }
        }
        assert!(caught, "{:?}", g.world);
        assert!(g.verify_replay().is_ok());
    }

    #[test]
    fn cpu_can_win_and_invalid_input_is_atomic() {
        let mut g = game("relay", "bulwark", 0);
        let saved = g.world;
        assert!(g.step(256).is_err());
        assert_eq!(g.world, saved);
        for _ in 0..MATCH_FRAMES {
            tick(&mut g, 0, 1);
            if g.world.winner != 0 {
                break;
            }
        }
        assert_eq!(g.world.winner, 2);
        assert!(g.verify_replay().is_ok());
        assert!(Game::new(&[0; 16], &[0; 16], 0, 0).is_err());
        let p = pack("relay");
        assert!(Game::new(&p[..p.len() - 1], &p, 0, 0).is_err());
        assert!(Game::new(&p, &p, 0, 4).is_err());
    }
    #[test]
    fn whiff_cancels_are_denied_and_both_walls_separate() {
        let mut g = game("relay", "bulwark", 1);
        tick(&mut g, LIGHT, 1);
        tick(&mut g, 0, 6);
        tick(&mut g, MEDIUM, 1);
        assert_eq!(g.defs[0].role(&g.world.actors[0].state), JAB);
        assert_eq!(g.world.stats.cancels[0], 0);
        tick(&mut g, RIGHT, 500);
        let a = g.world.actors;
        assert!(
            (a[0].x - a[1].x).abs() >= 40 * SCALE,
            "positions {:?}, views {:?}/{:?}",
            (a[0].x, a[1].x),
            g.defs[0].width,
            g.defs[1].width
        );
        assert!(g.verify_replay().is_ok());
    }
}
