//! A deliberately small game consumer. Combat policy lives here, never in the helper crates.
use framesmith_fspack::{payload::ValueView, OwnedPack};
use framesmith_runtime::{
    check_hits, check_pushbox, init_resources, next_frame, report_block, report_hit, Aabb,
    CharacterState, FrameInput,
};
use serde::Serialize;

const LEFT: u8 = 1;
const RIGHT: u8 = 2;
const LIGHT: u8 = 4;
const HEAVY: u8 = 8;
const BURST: u8 = 16;
const GUARD: u8 = 32;
const IDLE: usize = 0;
const BLOCK: usize = 1;
const JAB: usize = 2;
const DRIVE: usize = 3;
const SUPER: usize = 4;
const STUN: usize = 5;
const IDS: [&str; 6] = ["idle", "guard", "light", "heavy", "burst", "stun"];
const HZ: u32 = 60; // Demo policy, not a FrameSmith clock requirement.
const ROUND_FRAMES: u32 = HZ * 60;
const SCALE: i32 = 256;

fn number(value: ValueView<'_>) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_i64().map(|v| v as f64))
        .or_else(|| value.as_u64().map(|v| v as f64))
}
fn required_number(value: ValueView<'_>, key: &str, min: f64, max: f64) -> Result<f64, String> {
    let n = value
        .get(key)
        .and_then(number)
        .ok_or_else(|| format!("Missing numeric payload: {key}"))?;
    if !n.is_finite() || n < min || n > max {
        return Err(format!("Out-of-range payload: {key}"));
    }
    Ok(n)
}

struct Definition {
    pack: OwnedPack,
    ids: [u16; 6],
    name: String,
    color: String,
    health: i32,
    speed: i32,
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
        let name = character
            .get("name")
            .and_then(ValueView::as_str)
            .ok_or("Missing name")?
            .to_owned();
        let color = props
            .get("color")
            .and_then(ValueView::as_str)
            .ok_or("Missing color")?
            .to_owned();
        let health = required_number(props, "health", 1.0, 60000.0)? as i32;
        let speed = required_number(props, "walk_speed", 0.25, 8.0)?;
        if (speed * f64::from(SCALE)).fract() != 0.0 {
            return Err("Demo movement requires 1/256-pixel steps".into());
        }
        let counter_bonus = required_number(props, "counter_bonus", 0.0, 100.0)? as i32;
        let states = view.states().ok_or("Missing compiled states")?;
        let mut ids = [0; 6];
        for (slot, name) in ids.iter_mut().zip(IDS) {
            *slot = (0..states.len())
                .find(|&i| view.state_id(i) == Some(name))
                .ok_or_else(|| format!("Demo requires authored state id '{name}'"))?
                as u16;
        }
        let resources = view.resource_defs().ok_or("Missing resource definitions")?;
        let meter = (0..resources.len())
            .find(|&i| {
                resources
                    .get(i)
                    .is_some_and(|d| view.string(d.name_off(), d.name_len()) == Some("meter"))
            })
            .ok_or("Missing meter pool")?;
        let meter_max = resources.get(meter).ok_or("Invalid meter pool")?.max();
        let mut test_state = CharacterState::default();
        if !init_resources(&mut test_state, &view) {
            return Err("Unsupported resource capacity".into());
        }
        Ok(Self {
            pack,
            ids,
            name,
            color,
            health,
            speed: (speed * f64::from(SCALE)) as i32,
            counter_bonus,
            meter,
            meter_max,
            bytes: bytes.len(),
        })
    }
    fn role(&self, state: &CharacterState) -> usize {
        self.ids
            .iter()
            .position(|&id| id == state.current_state)
            .unwrap_or(STUN)
    }
    fn actor(&self, x: i32) -> Actor {
        let mut state = CharacterState {
            current_state: self.ids[IDLE],
            ..Default::default()
        };
        assert!(init_resources(&mut state, &self.pack.view()));
        Actor {
            state,
            x: x * SCALE,
            health: self.health,
            ..Default::default()
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Actor {
    state: CharacterState,
    x: i32,
    health: i32,
    previous_input: u8,
    control: u8,
    buffer: u8,
    buffer_age: u8,
    connected: bool,
    blocked_stun: bool,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
struct Stats {
    hits: [u32; 2],
    blocks: [u32; 2],
    cancels: [u32; 2],
    spent: [u32; 2],
    signals: [u32; 2],
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
struct Contact {
    tick: u32,
    who: usize,
    damage: i32,
    blocked: bool,
    counter: bool,
    serial: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct World {
    actors: [Actor; 2],
    tick: u32,
    rng: u32,
    mode: u8,
    ai_wait: u16,
    ai_guard: u8,
    hitstop: u8,
    winner: u8,
    stats: Stats,
    contact: Contact,
}
fn rng(world: &mut World) -> u32 {
    world.rng = world.rng.wrapping_mul(1664525).wrapping_add(1013904223);
    world.rng
}
fn bot(world: &mut World, defs: &[Definition; 2]) -> u8 {
    if world.mode == 1 {
        return 0;
    }
    if world.mode == 2 {
        return GUARD;
    }
    if world.ai_guard > 0 {
        world.ai_guard -= 1;
        return GUARD;
    }
    let cpu = world.actors[1];
    let distance = (cpu.x - world.actors[0].x) / SCALE;
    if defs[1].role(&cpu.state) > BLOCK {
        return 0;
    }
    let player = world.actors[0];
    if defs[0].role(&player.state) >= JAB
        && defs[0].role(&player.state) <= SUPER
        && player.state.frame >= 6
        && distance < 155
        && world.tick.is_multiple_of(12)
        && rng(world).is_multiple_of(3)
    {
        world.ai_guard = 18;
        return GUARD;
    }
    if world.ai_wait > 0 {
        world.ai_wait -= 1;
    }
    if distance > 112 {
        return LEFT;
    }
    if world.ai_wait == 0 {
        let choice = rng(world);
        world.ai_wait = 20 + (choice % 18) as u16;
        if cpu.state.resources[defs[1].meter] >= 50 && choice.is_multiple_of(4) {
            return BURST;
        }
        return if distance < 78 && choice.is_multiple_of(3) {
            LIGHT
        } else {
            HEAVY
        };
    }
    if distance < 56 {
        RIGHT
    } else {
        0
    }
}
fn enter(actor: &mut Actor, id: u16, duration: u16) {
    actor.state.current_state = id;
    actor.state.frame = 0;
    actor.state.instance_duration = duration;
    actor.state.hit_confirmed = false;
    actor.state.block_confirmed = false;
    actor.connected = false;
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
fn step_world(world: &mut World, defs: &[Definition; 2], input: u8) {
    if world.winner != 0 {
        return;
    }
    let cpu = if world.hitstop == 0 {
        bot(world, defs)
    } else {
        world.actors[1].control & GUARD
    };
    world.tick += 1;
    for (actor, control) in world.actors.iter_mut().zip([input, cpu]) {
        let pressed = control & !actor.previous_input;
        let attack = if pressed & BURST != 0 {
            BURST
        } else if pressed & HEAVY != 0 {
            HEAVY
        } else {
            pressed & LIGHT
        };
        if attack != 0 {
            actor.buffer = attack;
            actor.buffer_age = 8;
        }
        actor.previous_input = control;
        actor.control = control;
    }
    if world.hitstop > 0 {
        world.hitstop -= 1;
    } else {
        for (i, def) in defs.iter().enumerate() {
            let actor = &mut world.actors[i];
            let role = def.role(&actor.state);
            let requested = if role == STUN {
                None
            } else if actor.buffer_age > 0 {
                Some(
                    def.ids[match actor.buffer {
                        LIGHT => JAB,
                        HEAVY => DRIVE,
                        _ => SUPER,
                    }],
                )
            } else if role <= BLOCK {
                let neutral = if actor.control & GUARD != 0 {
                    BLOCK
                } else {
                    IDLE
                };
                (role != neutral).then_some(def.ids[neutral])
            } else {
                None
            };
            let before = actor.state;
            let result = next_frame(
                &actor.state,
                &def.pack.view(),
                &FrameInput {
                    requested_state: requested,
                },
            );
            actor.state = result.state;
            if actor.state.current_state != before.current_state {
                let next_role = def.role(&actor.state);
                if (JAB..=SUPER).contains(&next_role) {
                    if (JAB..=SUPER).contains(&role) {
                        world.stats.cancels[i] += 1;
                    }
                    world.stats.spent[i] += u32::from(
                        before.resources[def.meter]
                            .saturating_sub(actor.state.resources[def.meter]),
                    );
                    actor.buffer_age = 0;
                    let lunge = def
                        .pack
                        .view()
                        .state_data(actor.state.current_state as usize)
                        .and_then(|v| v.get("properties")?.get("lunge_px"))
                        .and_then(number)
                        .unwrap_or(0.0) as i32;
                    actor.x += lunge.clamp(0, 32) * SCALE * if i == 0 { 1 } else { -1 };
                    reward(actor, def, actor.state.current_state, 0);
                }
                actor.connected = false;
                actor.blocked_stun = false;
            }
            if result.move_ended {
                enter(
                    actor,
                    def.ids[if actor.control & GUARD != 0 {
                        BLOCK
                    } else {
                        IDLE
                    }],
                    0,
                );
                actor.blocked_stun = false;
            }
            if actor.buffer_age > 0 {
                actor.buffer_age -= 1;
            }
            if def.role(&actor.state) <= BLOCK {
                let axis =
                    i32::from(actor.control & RIGHT != 0) - i32::from(actor.control & LEFT != 0);
                actor.x += axis * def.speed / if actor.control & GUARD != 0 { 2 } else { 1 };
            }
            actor.x = actor.x.clamp(72 * SCALE, 928 * SCALE);
            let pack = def.pack.view();
            if let (Some(extra), Some(notes)) = (
                pack.state_extras()
                    .and_then(|e| e.get(actor.state.current_state as usize)),
                pack.move_notifies(),
            ) {
                let (offset, count) = extra.notifies();
                for j in 0..usize::from(count) {
                    if let Some(note) = notes.get_at(offset, j) {
                        if note.frame() == actor.state.frame {
                            world.stats.signals[i] += u32::from(note.emits().1);
                        }
                    }
                }
            }
        }
        // ponytail: grounded, side-locked duel. Add facing/cross-up state only with jumping/cross-ups.
        if let Some(push) = check_pushbox(
            &world.actors[0].state,
            &defs[0].pack.view(),
            (world.actors[0].x / SCALE, 0),
            &world.actors[1].state,
            &defs[1].pack.view(),
            (world.actors[1].x / SCALE, 0),
        ) {
            world.actors[0].x += push.p1_dx * SCALE;
            world.actors[1].x += push.p2_dx * SCALE;
            // A wall cannot absorb its half of a separation: transfer that correction to the other actor.
            let left_overflow = (72 * SCALE - world.actors[0].x).max(0);
            let right_overflow = (world.actors[1].x - 928 * SCALE).max(0);
            world.actors[1].x += left_overflow;
            world.actors[0].x -= right_overflow;
        }
        for actor in &mut world.actors {
            actor.x = actor.x.clamp(72 * SCALE, 928 * SCALE);
        }
        let hits = std::array::from_fn::<_, 2, _>(|i| {
            let j = 1 - i;
            if world.actors[i].connected {
                return None;
            }
            // In attack-local space both authored packs face +X; symmetric hurtboxes mirror exactly.
            let facing = if i == 0 { 1 } else { -1 };
            let a = world.actors[i];
            let b = world.actors[j];
            let hit = check_hits(
                &a.state,
                &defs[i].pack.view(),
                (a.x / SCALE * facing, 0),
                &b.state,
                &defs[j].pack.view(),
                (b.x / SCALE * facing, 0),
            )
            .get(0)
            .copied()?;
            let role = defs[j].role(&b.state);
            Some((
                hit,
                role == BLOCK || role == STUN && b.blocked_stun,
                (JAB..=SUPER).contains(&role),
            ))
        });
        // Compute both contacts before applying either, so trades are not entity-order wins.
        for (i, contact) in hits.into_iter().enumerate() {
            if let Some((hit, blocked, counter)) = contact {
                let j = 1 - i;
                let damage = if blocked {
                    i32::from(hit.chip_damage)
                } else {
                    i32::from(hit.damage) + if counter { defs[i].counter_bonus } else { 0 }
                };
                world.actors[i].connected = true;
                if blocked {
                    report_block(&mut world.actors[i].state);
                    world.stats.blocks[j] += 1;
                } else {
                    report_hit(&mut world.actors[i].state);
                    world.stats.hits[i] += 1;
                }
                reward(
                    &mut world.actors[i],
                    &defs[i],
                    hit.attacker_move,
                    if blocked { 2 } else { 1 },
                );
                world.actors[j].health = (world.actors[j].health - damage).max(0);
                enter(
                    &mut world.actors[j],
                    defs[j].ids[STUN],
                    u16::from(if blocked { hit.blockstun } else { hit.hitstun }).max(1),
                );
                world.actors[j].blocked_stun = blocked;
                let push = if blocked {
                    hit.block_pushback
                } else {
                    hit.hit_pushback
                };
                world.actors[j].x = (world.actors[j].x
                    + push * SCALE * if i == 0 { 1 } else { -1 })
                .clamp(72 * SCALE, 928 * SCALE);
                world.hitstop = world.hitstop.max(hit.hitstop);
                world.contact = Contact {
                    tick: world.tick,
                    who: i,
                    damage,
                    blocked,
                    counter: counter && !blocked,
                    serial: world.contact.serial + 1,
                };
            }
        }
    }
    let [player, cpu] = world.actors;
    if player.health == 0 || cpu.health == 0 || world.tick >= ROUND_FRAMES {
        let p = i64::from(player.health) * i64::from(defs[1].health);
        let c = i64::from(cpu.health) * i64::from(defs[0].health);
        world.winner = if p > c {
            1
        } else if c > p {
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
fn rectangle(shape: framesmith_fspack::ShapeView<'_>, x: i32, facing: i32) -> Rect {
    let r = Aabb::from_shape(&shape, 0, 0);
    Rect {
        x: x + if facing == 1 { r.x } else { -r.x - r.w as i32 },
        y: r.y,
        w: r.w,
        h: r.h,
    }
}
#[derive(Serialize)]
pub struct FighterView<'a> {
    name: &'a str,
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
    let phase = if role == IDLE {
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
                {
                    hitboxes.push(rectangle(shape, a.x / SCALE, facing));
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
                        facing,
                    ));
                }
            }
        }
    }
    FighterView {
        name: &def.name,
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

pub struct Game {
    defs: [Definition; 2],
    world: World,
    base: World,
    // ponytail: exact per-frame states, bounded by the one-minute round; stream traces for longer games.
    tape: Vec<(u8, World)>,
}
impl Game {
    pub fn new(player: &[u8], opponent: &[u8], seed: u32, mode: u8) -> Result<Self, String> {
        if mode > 2 {
            return Err("Unknown opponent mode".into());
        }
        let defs = [Definition::new(player)?, Definition::new(opponent)?];
        let world = World {
            actors: [defs[0].actor(310), defs[1].actor(650)],
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
            tape: Vec::with_capacity(ROUND_FRAMES as usize),
        })
    }
    pub fn step(&mut self, input: u8) -> Result<(), String> {
        if input > 63 {
            return Err("Unknown input bits".into());
        }
        if self.world.winner == 0 {
            step_world(&mut self.world, &self.defs, input);
            self.tape.push((input, self.world));
        }
        Ok(())
    }
    pub fn view(&self) -> View<'_> {
        View {
            tick: self.world.tick,
            remaining: ROUND_FRAMES.saturating_sub(self.world.tick),
            winner: self.world.winner,
            hitstop: self.world.hitstop,
            rng: self.world.rng,
            mode: self.world.mode,
            actors: [
                fighter_view(&self.world.actors[0], &self.defs[0], 1),
                fighter_view(&self.world.actors[1], &self.defs[1], -1),
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
                integer(mode, 2)? as u8,
            )
            .map_err(|e| JsValue::from_str(&e))?;
            Ok(Self { game })
        }
        pub fn step(&mut self, input: f64) -> Result<JsValue, JsValue> {
            self.game
                .step(integer(input, 63)? as u8)
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
    fn game(mode: u8) -> Game {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("dist/packs");
        Game::new(
            &std::fs::read(root.join("relay.fspk")).expect("run build.py --packs-only"),
            &std::fs::read(root.join("bulwark.fspk")).unwrap(),
            0xf5a17,
            mode,
        )
        .unwrap()
    }
    fn approach(g: &mut Game) {
        for _ in 0..100 {
            g.step(RIGHT).unwrap();
        }
    }
    fn combo_input(g: &Game) -> u8 {
        let a = g.world.actors[0];
        match g.defs[0].role(&a.state) {
            IDLE | BLOCK => LIGHT,
            JAB if a.state.hit_confirmed => HEAVY,
            DRIVE if a.state.hit_confirmed && a.state.resources[g.defs[0].meter] >= 50 => BURST,
            _ => 0,
        }
    }
    #[test]
    fn real_packs_play_to_ko_with_cancels_costs_events_and_exact_replay() {
        let mut g = game(1);
        assert_eq!(g.world.actors[0].health, 600);
        assert_eq!(g.world.actors[1].health, 720);
        assert_eq!(g.defs[0].speed, 832); // 3.25 survives binary payload, not a JS sidecar.
        assert_ne!(g.defs[0].ids[IDLE], 0);
        approach(&mut g);
        for _ in 0..ROUND_FRAMES {
            let input = if g.world.actors[1].x - g.world.actors[0].x > 64 * SCALE {
                RIGHT
            } else {
                combo_input(&g)
            };
            g.step(input).unwrap();
            if g.world.winner != 0 {
                break;
            }
        }
        assert_eq!(g.world.winner, 1, "{:?}", g.world);
        assert_eq!(g.world.actors[1].health, 0);
        assert!(
            g.world.stats.cancels[0] > 0
                && g.world.stats.spent[0] >= 50
                && g.world.stats.signals[0] > 0
        );
        assert_eq!(g.verify_replay().unwrap(), g.tape.len());
        let terminal = g.world;
        g.step(RIGHT | LIGHT).unwrap();
        assert_eq!(terminal, g.world);
    }
    #[test]
    fn blocks_do_not_become_hit_confirms_and_whiff_cancels_are_denied() {
        let mut g = game(2);
        approach(&mut g);
        g.step(LIGHT).unwrap();
        for _ in 0..14 {
            g.step(0).unwrap();
        }
        assert!(g.world.actors[0].state.block_confirmed);
        assert!(!g.world.actors[0].state.hit_confirmed);
        g.step(HEAVY).unwrap();
        assert_eq!(g.world.stats.cancels[0], 0);
        assert_eq!(g.world.actors[1].health, 720);
        assert!(g.world.stats.blocks[1] > 0);
        assert!(g.verify_replay().is_ok());
        let mut whiff = game(1);
        whiff.step(LIGHT).unwrap();
        for _ in 0..8 {
            whiff.step(0).unwrap();
        }
        whiff.step(HEAVY).unwrap();
        assert_eq!(whiff.defs[0].role(&whiff.world.actors[0].state), JAB);
    }
    #[test]
    fn bot_can_win_and_mid_match_restore_reproduces_all_state() {
        let mut g = game(0);
        for _ in 0..173 {
            g.step(0).unwrap();
        }
        g.checkpoint();
        let saved = g.world;
        for i in 0..150 {
            g.step(if i % 30 < 12 { GUARD } else { LIGHT }).unwrap();
        }
        let expected = g.world;
        assert_eq!(g.verify_replay().unwrap(), 150);
        g.restore();
        assert_eq!(g.world, saved);
        for i in 0..150 {
            g.step(if i % 30 < 12 { GUARD } else { LIGHT }).unwrap();
        }
        assert_eq!(g.world, expected);
        for _ in 0..ROUND_FRAMES {
            g.step(0).unwrap();
            if g.world.winner != 0 {
                break;
            }
        }
        assert_eq!(g.world.winner, 2);
        assert_eq!(g.world.actors[0].health, 0);
        assert!(g.verify_replay().is_ok());
    }
    #[test]
    fn player_can_beat_the_live_seeded_opponent() {
        let mut g = game(0);
        for _ in 0..ROUND_FRAMES {
            let input = if g.world.actors[1].x - g.world.actors[0].x > 64 * SCALE {
                RIGHT
            } else {
                combo_input(&g)
            };
            g.step(input).unwrap();
            if g.world.winner != 0 {
                break;
            }
        }
        assert_eq!(g.world.winner, 1, "{:?}", g.world);
        assert_eq!(g.world.actors[1].health, 0);
        assert!(g.verify_replay().is_ok());
    }
    #[test]
    fn pushing_into_the_wall_does_not_leave_penetrating_pushboxes() {
        let mut g = game(1);
        for _ in 0..ROUND_FRAMES {
            g.step(RIGHT).unwrap();
            if let Some(push) = check_pushbox(
                &g.world.actors[0].state,
                &g.defs[0].pack.view(),
                (g.world.actors[0].x / SCALE, 0),
                &g.world.actors[1].state,
                &g.defs[1].pack.view(),
                (g.world.actors[1].x / SCALE, 0),
            ) {
                assert_eq!((push.p1_dx, push.p2_dx), (0, 0), "tick {}", g.world.tick);
            }
        }
        assert!(g.verify_replay().is_ok());
    }
    #[test]
    fn invalid_data_and_input_fail_without_mutation() {
        assert!(Game::new(&[0; 16], &[0; 16], 0, 0).is_err());
        let mut g = game(0);
        let before = g.world;
        assert!(g.step(255).is_err());
        assert_eq!(before, g.world);
        let bytes = std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("dist/packs/relay.fspk"),
        )
        .unwrap();
        assert!(Game::new(&bytes[..bytes.len() - 1], &bytes, 0, 0).is_err());
    }
}
