//! A bounded combat-design lab, not a second fighting-game engine.
#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
mod authoring;
use authoring::Settings;
use framesmith_authoring::schema::{CharacterData, EventEmit, GuardType, ResourceDelta};
use framesmith_fspack::OwnedPack;
use framesmith_runtime::{
    can_cancel_to, check_hits, check_pushbox, init_resources, next_frame, report_block, report_hit,
    CharacterState, FrameInput,
};
use serde::Serialize;
use serde_json::{json, Value};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

const ACTIONS: [&str; 7] = [
    "jab",
    "follow",
    "special",
    "finisher",
    "multi",
    "special~charged",
    "reload",
];
const MAX_FRAMES: usize = 3600;
const TRACE_LEN: usize = 48;
// Trace kinds: input, start, link attempt, cancel, hit, block, whiff, rejected,
// event, resource, gap, trial clear, trial fail, auto end.
const INPUT: u8 = 1;
const START: u8 = 2;
const LINK: u8 = 3;
const CANCEL: u8 = 4;
const HIT: u8 = 5;
const BLOCK: u8 = 6;
const WHIFF: u8 = 7;
const DENIED: u8 = 8;
const EVENT: u8 = 9;
const RESOURCE: u8 = 10;
const GAP: u8 = 11;
const CLEAR: u8 = 12;
const FAILED: u8 = 13;
const END: u8 = 14;
const PUSH: u8 = 15;

struct Definition {
    pack: OwnedPack,
    bytes: Vec<u8>,
    data: CharacterData,
    ids: [u16; 7],
    idle: u16,
    hit: u16,
    block: u16,
    energy: usize,
    ammo: usize,
}
impl Definition {
    fn new(bytes: Vec<u8>) -> Result<Self, String> {
        if bytes.len() > 1024 * 1024 {
            return Err("Lab pack exceeds 1 MiB".into());
        }
        let pack = OwnedPack::new(bytes.clone()).map_err(|e| format!("Invalid FSPK: {e:?}"))?;
        let p = pack.view();
        let payload = p.payload().ok_or("The lab requires FSPK v2 typed data")?;
        let data: CharacterData =
            serde_json::from_value(payload.root().to_json()).map_err(|e| e.to_string())?;
        if data.moves.len() > 64 || data.character.resources.len() > 8 {
            return Err("Lab limits: 64 states, 8 resources".into());
        }
        let lookup = |name: &str| -> Result<u16, String> {
            (0..data.moves.len())
                .find(|i| p.state_id(*i) == Some(name))
                .map(|i| i as u16)
                .ok_or_else(|| format!("Lab pack needs state {name}"))
        };
        let mut ids = [0; 7];
        for (i, name) in ACTIONS.iter().enumerate() {
            ids[i] = lookup(name)?;
        }
        let (idle, hit, block) = (lookup("idle")?, lookup("hitstun")?, lookup("blockstun")?);
        let index = |n: &str| {
            data.character
                .resources
                .iter()
                .position(|r| r.name == n)
                .ok_or_else(|| format!("Missing resource {n}"))
        };
        let (energy, ammo) = (index("energy")?, index("ammo")?);
        let mut state = CharacterState::default();
        if !init_resources(&mut state, &p) {
            return Err("Unsupported resources".into());
        }
        for (i, m) in data.moves.iter().enumerate() {
            if p.state_id(i) != Some(m.id.as_deref().unwrap_or(&m.input)) {
                return Err("Pack state ordering differs from payload".into());
            }
            let mv = p
                .states()
                .and_then(|s| s.get(i))
                .ok_or("Missing helper state")?;
            if mv.hit_windows_len() > 64
                || m.startup as u16 + m.active as u16 + m.recovery as u16 == 0
            {
                return Err("Invalid lab timing/window count".into());
            }
        }
        Ok(Self {
            pack,
            bytes,
            data,
            ids,
            idle,
            hit,
            block,
            energy,
            ammo,
        })
    }
    fn command(&self, id: u16) -> u8 {
        self.ids
            .iter()
            .position(|x| *x == id)
            .map_or(0, |i| i as u8 + 1)
    }
    fn state(&self, id: u16) -> &framesmith_authoring::schema::State {
        &self.data.moves[id as usize]
    }
    fn phase(&self, s: &CharacterState) -> u8 {
        if s.current_state == self.idle {
            0
        } else if s.current_state == self.hit {
            4
        } else if s.current_state == self.block {
            5
        } else {
            let m = self.state(s.current_state);
            if s.frame < u16::from(m.startup) {
                1
            } else if s.frame < u16::from(m.startup) + u16::from(m.active) {
                2
            } else {
                3
            }
        }
    }
    fn fresh(&self) -> CharacterState {
        let mut s = CharacterState {
            current_state: self.idle,
            ..Default::default()
        };
        assert!(init_resources(&mut s, &self.pack.view()));
        s
    }
    fn reason(&self, s: &CharacterState, target: u16) -> String {
        if can_cancel_to(s, &self.pack.view(), target) {
            return "Available now".into();
        }
        if self.pack.view().has_cancel_deny(s.current_state, target) {
            return "Explicit deny overrides the tag rule".into();
        }
        let m = self.state(target);
        for cost in m.costs.as_deref().unwrap_or(&[]) {
            if let framesmith_authoring::schema::Cost::Resource { name, amount } = cost {
                if let Some(i) = self
                    .data
                    .character
                    .resources
                    .iter()
                    .position(|r| r.name == *name)
                {
                    if s.resources[i] < *amount {
                        return format!(
                            "Needs {amount} {name}; you have {}. Nothing spent.",
                            s.resources[i]
                        );
                    }
                }
            }
        }
        let from = self.state(s.current_state);
        let matches = |tag: &str, state: &framesmith_authoring::schema::State| {
            tag == "any"
                || state.input == tag
                || state.id.as_deref() == Some(tag)
                || state.tags.iter().any(|t| t.as_str() == tag)
        };
        for rule in &self.data.cancel_table.tag_rules {
            if !matches(&rule.from, from) || !matches(&rule.to, m) {
                continue;
            }
            if s.frame < u16::from(rule.after_frame) {
                return format!(
                    "Cancel opens at frame {}; now {}",
                    rule.after_frame, s.frame
                );
            }
            if s.frame > u16::from(rule.before_frame) {
                return format!("Cancel window closed at frame {}", rule.before_frame);
            }
            if !rule.on.matches(s.hit_confirmed, s.block_confirmed) {
                return "Required hit/block/whiff confirmation is absent".into();
            }
        }
        "No matching tag rule here. Finish recovery to link.".into()
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
struct Notice {
    seq: u32,
    tick: u32,
    kind: u8,
    command: u8,
    value: i32,
    aux: i32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct World {
    tick: u32,
    p: CharacterState,
    d: CharacterState,
    denied: CharacterState,
    px: i32,
    dx: i32,
    behavior: u8,
    freeze: u8,
    consumed: u64,
    buffer: u8,
    buffer_age: u8,
    last_action: u8,
    transition: u8,
    combo: u16,
    max_combo: u16,
    damage: u32,
    hits: u16,
    blocks: u16,
    links: u16,
    cancels: u16,
    had_hit: bool,
    freed_at: u32,
    contact_tick: u32,
    contact_kind: u8,
    contact_damage: u16,
    trial: i8,
    trial_progress: u8,
    trial_failed: bool,
    trial_clear: bool,
    manual: bool,
    auto: bool,
    route: [u8; 4],
    route_len: u8,
    cursor: u8,
    notices: [Notice; TRACE_LEN],
    notice_count: u32,
    event_count: u16,
}
impl World {
    fn new(d: &Definition, behavior: u8, distance: i32, trial: i8) -> Self {
        Self {
            tick: 0,
            p: d.fresh(),
            d: d.fresh(),
            denied: d.fresh(),
            px: 0,
            dx: distance,
            behavior,
            freeze: 0,
            consumed: 0,
            buffer: 0,
            buffer_age: 0,
            last_action: 0,
            transition: START,
            combo: 0,
            max_combo: 0,
            damage: 0,
            hits: 0,
            blocks: 0,
            links: 0,
            cancels: 0,
            had_hit: false,
            freed_at: 0,
            contact_tick: 0,
            contact_kind: 0,
            contact_damage: 0,
            trial,
            trial_progress: 0,
            trial_failed: false,
            trial_clear: false,
            manual: true,
            auto: false,
            route: [0; 4],
            route_len: 0,
            cursor: 0,
            notices: [Notice::default(); TRACE_LEN],
            notice_count: 0,
            event_count: 0,
        }
    }
    fn note(&mut self, kind: u8, command: u8, value: i32, aux: i32) {
        let n = self.notice_count as usize % TRACE_LEN;
        self.notices[n] = Notice {
            seq: self.notice_count + 1,
            tick: self.tick,
            kind,
            command,
            value,
            aux,
        };
        self.notice_count += 1;
    }
}
#[derive(Clone, Copy, Debug, Serialize)]
struct Sample {
    tick: u32,
    player: u8,
    dummy: u8,
    command: u8,
    frame: u16,
    freeze: bool,
    contact: u8,
}
fn sample(w: &World, d: &Definition) -> Sample {
    Sample {
        tick: w.tick,
        player: d.phase(&w.p),
        dummy: d.phase(&w.d),
        command: d.command(w.p.current_state),
        frame: w.p.frame,
        freeze: w.freeze > 0,
        contact: if w.contact_tick == w.tick {
            w.contact_kind
        } else {
            0
        },
    }
}
fn enter(s: &mut CharacterState, id: u16, duration: u16) {
    s.current_state = id;
    s.frame = 0;
    s.instance_duration = duration;
    s.hit_confirmed = false;
    s.block_confirmed = false;
}
fn deltas(w: &mut World, d: &Definition, values: &[ResourceDelta], cmd: u8) {
    for delta in values {
        if let Some(i) = d
            .data
            .character
            .resources
            .iter()
            .position(|r| r.name == delta.name)
        {
            let before = w.p.resources[i];
            w.p.resources[i] = (i64::from(before) + i64::from(delta.delta))
                .clamp(0, i64::from(d.data.character.resources[i].max))
                as u16;
            w.note(
                RESOURCE,
                cmd,
                i32::from(w.p.resources[i]) - i32::from(before),
                i as i32,
            );
        }
    }
}
fn events(w: &mut World, values: &[EventEmit], cmd: u8) {
    for event in values {
        let kind = match event.id.as_str() {
            "spark" => 1,
            "charge" => 2,
            _ => 0,
        };
        let size = event
            .args
            .get("size")
            .and_then(|v| match v {
                framesmith_authoring::schema::EventArgValue::F32(n) => Some(*n as i32),
                framesmith_authoring::schema::EventArgValue::I64(n) => Some(*n as i32),
                _ => None,
            })
            .unwrap_or(12);
        w.event_count = w.event_count.saturating_add(1);
        w.note(EVENT, cmd, size, kind);
    }
}
fn route(trial: i8) -> (&'static [u8], &'static [u8]) {
    match trial {
        0 => (&[1, 2], &[START, LINK]),
        1 => (&[2, 3], &[START, CANCEL]),
        2 => (&[3, 4], &[START, CANCEL]),
        _ => (&[1, 2, 3, 4], &[START, LINK, CANCEL, CANCEL]),
    }
}
fn fail_trial(w: &mut World, cmd: u8, reason: i32) {
    if w.trial >= 0 && !w.trial_failed && !w.trial_clear && w.manual {
        w.trial_failed = true;
        w.note(FAILED, cmd, reason, 0);
    }
}
fn step_world(w: &mut World, d: &Definition, command: u8) {
    w.tick += 1;
    if command > 0 {
        w.buffer = command;
        w.buffer_age = 5;
        w.note(INPUT, command, 0, 0);
    }
    if w.freeze > 0 {
        w.freeze -= 1;
        return;
    }
    // Stun counts frames AFTER impact. +1 retains the impact sample at frame zero.
    let was_stunned = w.d.current_state == d.hit;
    let next = next_frame(&w.d, &d.pack.view(), &FrameInput::default());
    w.d = next.state;
    if next.move_ended {
        if was_stunned {
            w.freed_at = w.tick;
            w.combo = 0;
            if w.trial_progress > 0 && !w.trial_clear {
                fail_trial(w, 0, 1);
            }
        }
        enter(&mut w.d, d.idle, 0);
    }
    if w.auto && w.cursor < w.route_len {
        let next = w.route[w.cursor as usize];
        let first = w.cursor == 0;
        let link = w.cursor == 1 && w.route[0] == 1 && next == 2;
        let ready = w.p.current_state == d.idle;
        let m = d.state(w.p.current_state);
        let confirmed = w.p.hit_confirmed
            || w.p.block_confirmed
            || w.p.frame >= u16::from(m.startup) + u16::from(m.active);
        if first || link && ready || !link && confirmed && !ready {
            w.buffer = next;
            w.buffer_age = 2;
        }
        if !first && !link && ready {
            w.auto = false;
            w.denied = w.p;
            w.note(DENIED, next, 1, 0);
        }
    }
    let old = w.p;
    let target = (w.buffer_age > 0).then(|| d.ids[(w.buffer - 1) as usize]);
    let result = next_frame(
        &old,
        &d.pack.view(),
        &FrameInput {
            requested_state: target,
        },
    );
    w.p = result.state;
    let mut accepted = target == Some(w.p.current_state) && w.p.frame == 0;
    if result.move_ended {
        let cmd = d.command(old.current_state);
        if cmd > 0 && w.consumed == 0 && !d.state(old.current_state).hitboxes.is_empty() {
            w.note(WHIFF, cmd, 0, 0);
            fail_trial(w, cmd, 2);
        }
        enter(&mut w.p, d.idle, 0);
        let wanted_cancel =
            w.auto && w.cursor > 0 && !(w.cursor == 1 && w.route[0] == 1 && w.route[1] == 2);
        if wanted_cancel {
            w.auto = false;
            w.denied = old;
            w.note(DENIED, w.buffer, 0, 0);
            w.buffer = 0;
            w.buffer_age = 0;
        }
        // A manual buffered link may start at the recovery boundary. A scripted
        // cancel must not silently turn into a different (possibly valid) link.
        else if let Some(t) = target {
            let r = next_frame(
                &w.p,
                &d.pack.view(),
                &FrameInput {
                    requested_state: Some(t),
                },
            );
            if r.state.current_state == t && r.state.frame == 0 {
                w.p = r.state;
                accepted = true;
            }
        }
    }
    if accepted {
        let cmd = w.buffer;
        w.transition = if old.current_state == d.idle || result.move_ended {
            if w.last_action > 0 {
                LINK
            } else {
                START
            }
        } else {
            CANCEL
        };
        if w.transition == CANCEL {
            w.cancels += 1;
        }
        w.note(
            w.transition,
            cmd,
            i32::from(old.frame),
            i32::from(d.command(old.current_state)),
        );
        if w.trial >= 0 && w.manual && !w.trial_failed {
            let (expected, kinds) = route(w.trial);
            let i = w.trial_progress as usize;
            if i >= expected.len() || expected[i] != cmd || kinds[i] != w.transition {
                fail_trial(w, cmd, 3);
            }
        }
        w.last_action = cmd;
        w.consumed = 0;
        w.buffer = 0;
        w.buffer_age = 0;
        if w.auto {
            w.cursor += 1;
        }
        let m = d.state(w.p.current_state);
        if let Some(on) = &m.on_use {
            deltas(w, d, &on.resource_deltas, cmd);
            events(w, &on.events, cmd);
        }
        // One presentation freeze policy; the authored value comes from the v2 payload.
        if let Some(f) = &m.super_freeze {
            w.freeze = f.frames.min(12);
        }
    } else if w.buffer_age > 0 {
        w.buffer_age -= 1;
        if w.buffer_age == 0 {
            w.denied = w.p;
            w.note(DENIED, w.buffer, 0, 0);
            if w.trial >= 0 {
                fail_trial(w, w.buffer, 4);
            }
            w.buffer = 0;
        }
    }
    let cmd = d.command(w.p.current_state);
    let m = d.state(w.p.current_state);
    for n in &m.notifies {
        if w.p.frame == n.frame {
            events(w, &n.events, cmd);
        }
    }
    let hits = check_hits(
        &w.p,
        &d.pack.view(),
        (w.px, 0),
        &w.d,
        &d.pack.view(),
        (w.dx, 0),
    );
    for h in hits.iter() {
        let mask = 1u64 << h.window_index;
        if w.consumed & mask != 0 {
            continue;
        }
        let first_contact = w.consumed == 0;
        w.consumed |= mask;
        let advanced = m
            .hits
            .as_ref()
            .and_then(|hs| hs.get(h.window_index as usize));
        let (damage, stun, blockstun, stop, guard) = advanced.map_or(
            (h.damage, h.hitstun, h.blockstun, h.hitstop, &m.guard),
            |a| (a.damage, a.hitstun, a.blockstun, a.hitstop, &a.guard),
        );
        let in_stun = w.d.current_state == d.hit && w.d.frame < w.d.instance_duration;
        let guards = match w.behavior {
            1 => true,
            2 => !matches!(guard, GuardType::High),
            3 => w.had_hit && !in_stun,
            _ => false,
        };
        let blocked = guards && !matches!(guard, GuardType::Unblockable);
        w.contact_tick = w.tick;
        w.contact_kind = if blocked { BLOCK } else { HIT };
        w.contact_damage = if blocked {
            advanced
                .and_then(|h| h.chip_damage)
                .unwrap_or(h.chip_damage)
        } else {
            damage
        };
        w.freeze = w.freeze.max(stop.min(12));
        if blocked {
            report_block(&mut w.p);
            w.blocks += 1;
            w.combo = 0;
            enter(&mut w.d, d.block, u16::from(blockstun) + 1);
            w.note(BLOCK, cmd, i32::from(w.contact_damage), 0);
            fail_trial(w, cmd, 5);
            if let Some(on) = &m.on_block {
                deltas(w, d, &on.resource_deltas, cmd);
                events(w, &on.events, cmd);
            }
        } else {
            if w.had_hit && !in_stun {
                w.note(GAP, cmd, (w.tick - w.freed_at) as i32, 0);
            }
            report_hit(&mut w.p);
            w.hits += 1;
            w.damage += u32::from(damage);
            w.combo = if in_stun { w.combo + 1 } else { 1 };
            w.max_combo = w.max_combo.max(w.combo);
            w.had_hit = true;
            if in_stun && w.transition == LINK && first_contact {
                w.links += 1;
            }
            enter(&mut w.d, d.hit, u16::from(stun) + 1);
            w.note(HIT, cmd, i32::from(damage), i32::from(w.combo));
            if let Some(on) = &m.on_hit {
                deltas(w, d, &on.resource_deltas, cmd);
                events(w, &on.events, cmd);
            }
            if w.trial >= 0 && w.manual && !w.trial_failed && !w.trial_clear {
                let (expected, _) = route(w.trial);
                let index = w.trial_progress as usize;
                if expected.get(index) == Some(&cmd) && (index == 0 || in_stun) {
                    w.trial_progress += 1;
                    if w.trial_progress as usize == expected.len() {
                        w.trial_clear = true;
                        w.note(CLEAR, cmd, i32::from(w.combo), 0);
                    }
                } else {
                    fail_trial(w, cmd, 3);
                }
            }
        }
    }
    if let Some(push) = check_pushbox(
        &w.p,
        &d.pack.view(),
        (w.px, 0),
        &w.d,
        &d.pack.view(),
        (w.dx, 0),
    ) {
        w.px += push.p1_dx;
        w.dx += push.p2_dx;
        if push.p1_dx != 0 || push.p2_dx != 0 {
            w.note(PUSH, 0, w.dx - w.px, 0);
        }
    }
    if w.auto && w.cursor == w.route_len && w.p.current_state == d.idle {
        w.auto = false;
        w.note(END, 0, i32::from(w.max_combo), 0);
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub struct Lab {
    def: Definition,
    settings: Settings,
    editable: bool,
    w: World,
    initial: World,
    tape: Vec<(u8, World)>,
    samples: Vec<Sample>,
    checkpoint: Option<(World, usize)>,
}
impl Lab {
    fn create(bytes: Vec<u8>) -> Result<Self, String> {
        let def = Definition::new(bytes)?;
        let settings = Settings::default();
        let editable = authoring::compile(&settings.files())? == def.bytes;
        let w = World::new(&def, 3, 74, -1);
        Ok(Self {
            def,
            settings,
            editable,
            w,
            initial: w,
            tape: Vec::new(),
            samples: Vec::new(),
            checkpoint: None,
        })
    }
    fn reset_world(&mut self, behavior: u8, distance: i32, trial: i8) {
        self.w = World::new(&self.def, behavior, distance, trial);
        self.initial = self.w;
        self.tape.clear();
        self.samples.clear();
        self.checkpoint = None;
    }
    fn apply(&mut self, settings: Settings) -> Result<(), String> {
        let bytes = authoring::compile(&settings.files())?;
        let def = Definition::new(bytes)?;
        self.def = def;
        self.settings = settings;
        self.editable = true;
        self.reset_world(self.w.behavior, self.w.dx - self.w.px, -1);
        Ok(())
    }
    fn tick(&mut self, cmd: u8) -> Result<(), String> {
        if cmd > 7 {
            return Err("Unknown action".into());
        }
        if self.tape.len() >= MAX_FRAMES {
            return Err("60-second recording limit; reset to continue".into());
        }
        step_world(&mut self.w, &self.def, cmd);
        self.tape.push((cmd, self.w));
        self.samples.push(sample(&self.w, &self.def));
        Ok(())
    }
    fn replay(&self) -> Result<usize, String> {
        if self.tape.is_empty() {
            return Err("Record some frames first".into());
        }
        let mut w = self.initial;
        for (i, (cmd, expected)) in self.tape.iter().enumerate() {
            step_world(&mut w, &self.def, *cmd);
            if &w != expected {
                return Err(format!("Divergence at recorded frame {i}"));
            }
        }
        Ok(self.tape.len())
    }
    fn set_trial_inner(&mut self, id: i8) -> Result<(), String> {
        if !(-1..=3).contains(&id) {
            return Err("Unknown trial".into());
        }
        if id >= 0 {
            let mut s = Settings {
                recovery: 4,
                ..Default::default()
            };
            if id == 2 {
                s.energy = 50;
            }
            self.apply(s)?;
        }
        self.reset_world(3, 74, id);
        Ok(())
    }
    fn start_demo(&mut self, which: u8) -> Result<(), String> {
        if which > 4 {
            return Err("Unknown demonstration".into());
        }
        let trial = self.w.trial;
        self.reset_world(self.w.behavior, self.w.dx - self.w.px, trial);
        self.w.manual = false;
        // Reload is a resource-gain experiment: start this instance empty.
        if which == 3 {
            self.w.p.resources[self.def.ammo] = 0;
        }
        let commands: &[u8] = match which {
            0 => {
                if trial >= 0 {
                    route(trial).0
                } else {
                    &[1, 2, 3, 4]
                }
            }
            1 => &[5],
            2 => &[6],
            3 => &[7],
            _ => &[1],
        };
        self.w.route[..commands.len()].copy_from_slice(commands);
        self.w.route_len = commands.len() as u8;
        self.w.auto = true;
        self.initial = self.w;
        Ok(())
    }
    fn view_value(&self) -> Value {
        let d = &self.def;
        let w = &self.w;
        let actor = |s: &CharacterState, x: i32, dummy: bool| {
            let m = d.state(s.current_state);
            json!({"id":m.id.as_deref().unwrap_or(&m.input),"command":d.command(s.current_state),"name":if dummy{"DUMMY"}else{"RELAY"},"move_name":m.name,"frame":s.frame,"startup":m.startup,"active":m.active,"recovery":m.recovery,"phase":d.phase(s),"hit_confirmed":s.hit_confirmed,"block_confirmed":s.block_confirmed,"stun_remaining":s.instance_duration.saturating_sub(s.frame),"resources":s.resources[..d.data.character.resources.len()],"x":x,"y":0,"facing":if dummy{-1}else{1},"height":88,"width":36,"color":if dummy{"#ffa76b"}else{"#55e1ca"},"hitboxes":m.hitboxes.iter().filter(|b|s.frame>=u16::from(b.frames.0)&&s.frame<=u16::from(b.frames.1)).map(|b|&b.r#box).collect::<Vec<_>>(),"pushboxes":m.pushboxes.iter().filter(|b|s.frame>=u16::from(b.frames.0)&&s.frame<=u16::from(b.frames.1)).map(|b|&b.r#box).collect::<Vec<_>>(),"hurtboxes":m.hurtboxes.iter().filter(|b|s.frame>=u16::from(b.frames.0)&&s.frame<=u16::from(b.frames.1)).map(|b|&b.r#box).collect::<Vec<_>>()})
        };
        let start = w.notice_count.saturating_sub(TRACE_LEN as u32);
        let notices: Vec<_> = (start..w.notice_count)
            .map(|i| w.notices[i as usize % TRACE_LEN])
            .collect();
        let available:Vec<_>=d.ids.iter().enumerate().map(|(i,id)|json!({"command":i+1,"allowed":can_cancel_to(&w.p,&d.pack.view(),*id),"reason":d.reason(&w.p,*id)})).collect();
        let reason=notices.iter().rev().find(|n|[HIT,BLOCK,WHIFF,DENIED,GAP,CLEAR,FAILED,END,PUSH].contains(&n.kind)).map(|n|match n.kind {PUSH=>format!("Pushboxes separated overlapping bodies to {} pixels.",n.value),HIT=>if n.aux>1{format!("{}-hit true combo. {} damage.",n.aux,w.damage)}else{"Hit confirmed. The dummy is in hitstun.".into()},BLOCK=>"Blocked: the defender was able to guard. This is not a combo.".into(),WHIFF=>"Whiff: the active hitbox never reached the hurtbox.".into(),DENIED=>{let s=w.denied;if n.command>0{d.reason(&s,d.ids[n.command as usize-1])}else{"Move unavailable".into()}},GAP=>format!("Link broke: the dummy was free for {} frames.",n.value),CLEAR=>"TRIAL CLEAR — every required hit connected without a gap.".into(),FAILED=>match n.value{1=>"Trial failed: the defender recovered between hits.",2=>"Trial failed: an attack missed.",3=>"Trial failed: wrong move or transition (link vs cancel).",4=>"Too early: buffered input expired before the move became available.",_=>"Trial failed: the dummy blocked."}.into(),END=>format!("Demonstration finished: best true combo {} hits. Autoplay never clears a trial.",n.value),_=>String::new()}).unwrap_or_else(||"Try the sequence, change one rule, then run it again.".into());
        json!({"tick":w.tick,"actors":[actor(&w.p,w.px,false),actor(&w.d,w.dx,true)],"combo":w.combo,"max_combo":w.max_combo,"damage":w.damage,"hits":w.hits,"blocks":w.blocks,"links":w.links,"cancels":w.cancels,"freeze":w.freeze,"events":w.event_count,"energy":w.p.resources[d.energy],"ammo":w.p.resources[d.ammo],"available":available,"notices":notices,"reason":reason,"trial":w.trial,"trial_progress":w.trial_progress,"trial_failed":w.trial_failed,"trial_clear":w.trial_clear,"manual":w.manual,"auto":w.auto,"distance":w.dx-w.px,"dummy":w.behavior,"editable":self.editable&&w.trial<0,"recorded":self.tape.len(),"limit":MAX_FRAMES,"samples":&self.samples[self.samples.len().saturating_sub(180)..]})
    }
    fn metadata_value(&self) -> Value {
        let d = &self.def;
        let rows:Vec<_>=d.data.moves.iter().enumerate().map(|(i,m)| {
            let remaining=m.hitboxes.first().map(|h|i32::from(m.startup)+i32::from(m.active)+i32::from(m.recovery)-i32::from(h.frames.0)-1);
            let hit=m.hits.as_ref().and_then(|v|v.first());
            let on_hit=remaining.map(|n|i32::from(hit.map_or(m.hitstun,|h|h.hitstun))-n);
            let on_block=remaining.map(|n|i32::from(hit.map_or(m.blockstun,|h|h.blockstun))-n);
            json!({"id":m.id.as_deref().unwrap_or(&m.input),"input":m.input,"name":m.name,"command":d.command(i as u16),"startup":m.startup,"active":m.active,"recovery":m.recovery,"total":u16::from(m.startup)+u16::from(m.active)+u16::from(m.recovery),"damage":m.damage,"hitstun":m.hitstun,"blockstun":m.blockstun,"on_hit":on_hit,"on_block":on_block,"tags":m.tags,"animation":m.animation,"variant":m.id.as_deref().unwrap_or("").contains('~'),"resolved":m})}).collect();
        json!({"trace_kinds":{"input":INPUT,"hit":HIT,"block":BLOCK,"whiff":WHIFF,"event":EVENT},"event_kinds":{"swoosh":0,"spark":1,"charge":2},"settings":self.settings,"moves":rows,"cancel_table":d.data.cancel_table,"resources":d.data.character.resources,"character":d.data.character,"pack_bytes":d.bytes.len(),"format":"FSPK v2","rules":if self.editable{authoring::baseline()["framesmith.rules.json"].clone()}else{Value::Null},"trials":[{"name":"First link","route":[1,2]},{"name":"Hit-confirm cancel","route":[2,3]},{"name":"Spend it","route":[3,4]},{"name":"The full route","route":[1,2,3,4]}]})
    }
}
// Caller-supplied geometry queries, deliberately separate from the fighter's AABB windows.
fn geometry_data(kind: u8, gap: i32) -> Value {
    use framesmith_runtime::collision::{
        aabb_circle_overlap, capsule_overlap, circle_overlap, Aabb, Capsule, Circle,
    };
    let circle = Circle {
        x: gap,
        y: -50,
        r: 24,
    };
    let (overlap, shapes) = match kind {
        1 => {
            let a = Circle {
                x: 25,
                y: -50,
                r: 32,
            };
            (
                circle_overlap(&a, &circle),
                json!([{"kind":"circle","x":a.x,"y":a.y,"r":a.r},{"kind":"circle","x":circle.x,"y":circle.y,"r":circle.r}]),
            )
        }
        2 => {
            let a = Capsule {
                x1: 10,
                y1: -50,
                x2: 85,
                y2: -50,
                r: 10,
            };
            let b = Capsule {
                x1: gap,
                y1: -80,
                x2: gap,
                y2: -20,
                r: 16,
            };
            (
                capsule_overlap(&a, &b),
                json!([{"kind":"capsule","x1":a.x1,"y1":a.y1,"x2":a.x2,"y2":a.y2,"r":a.r},{"kind":"capsule","x1":b.x1,"y1":b.y1,"x2":b.x2,"y2":b.y2,"r":b.r}]),
            )
        }
        _ => {
            let a = Aabb {
                x: 10,
                y: -70,
                w: 70,
                h: 40,
            };
            (
                aabb_circle_overlap(&a, &circle),
                json!([{"kind":"aabb","x":a.x,"y":a.y,"w":a.w,"h":a.h},{"kind":"circle","x":circle.x,"y":circle.y,"r":circle.r}]),
            )
        }
    };
    json!({"overlap":overlap,"shapes":shapes,"owner":"Caller inputs → FrameSmith geometry helper; not a fighter combat run"})
}

fn integer(value: f64, min: i32, max: i32) -> Result<i32, String> {
    if !value.is_finite() || value.fract() != 0. || value < min as f64 || value > max as f64 {
        Err(format!("Expected an integer in {min}..={max}"))
    } else {
        Ok(value as i32)
    }
}
#[cfg(target_arch = "wasm32")]
fn js(value: Value) -> Result<JsValue, JsError> {
    value
        .serialize(&serde_wasm_bindgen::Serializer::json_compatible())
        .map_err(|e| JsError::new(&e.to_string()))
}
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl Lab {
    #[wasm_bindgen(constructor)]
    pub fn new(bytes: &[u8]) -> Result<Lab, JsError> {
        Self::create(bytes.to_vec()).map_err(|e| JsError::new(&e))
    }
    pub fn view(&self) -> Result<JsValue, JsError> {
        js(self.view_value())
    }
    pub fn metadata(&self) -> Result<JsValue, JsError> {
        js(self.metadata_value())
    }
    pub fn step(&mut self, command: f64) -> Result<JsValue, JsError> {
        let c = integer(command, 0, 7).map_err(|e| JsError::new(&e))?;
        self.tick(c as u8).map_err(|e| JsError::new(&e))?;
        self.view()
    }
    pub fn reset(&mut self) -> Result<JsValue, JsError> {
        self.reset_world(self.w.behavior, self.w.dx - self.w.px, self.w.trial);
        self.view()
    }
    pub fn edit(&mut self, key: &str, value: &str) -> Result<JsValue, JsError> {
        if !self.editable || self.w.trial >= 0 {
            return Err(JsError::new(
                "Rules are locked: leave the trial or reload the editable example",
            ));
        }
        let mut s = self.settings.clone();
        s.edit(key, value).map_err(|e| JsError::new(&e))?;
        self.apply(s).map_err(|e| JsError::new(&e))?;
        self.metadata()
    }
    pub fn dummy(&mut self, mode: f64, distance: f64) -> Result<JsValue, JsError> {
        if self.w.trial >= 0 {
            return Err(JsError::new("Trial spacing and dummy are locked"));
        }
        let m = integer(mode, 0, 3).map_err(|e| JsError::new(&e))?;
        let x = integer(distance, 20, 220).map_err(|e| JsError::new(&e))?;
        self.reset_world(m as u8, x, -1);
        self.view()
    }
    pub fn trial(&mut self, id: f64) -> Result<JsValue, JsError> {
        let i = integer(id, -1, 3).map_err(|e| JsError::new(&e))?;
        self.set_trial_inner(i as i8)
            .map_err(|e| JsError::new(&e))?;
        self.view()
    }
    pub fn demonstrate(&mut self, kind: f64) -> Result<JsValue, JsError> {
        let k = integer(kind, 0, 4).map_err(|e| JsError::new(&e))?;
        self.start_demo(k as u8).map_err(|e| JsError::new(&e))?;
        self.view()
    }
    pub fn checkpoint(&mut self) -> Result<JsValue, JsError> {
        self.checkpoint = Some((self.w, self.tape.len()));
        self.view()
    }
    pub fn restore(&mut self) -> Result<JsValue, JsError> {
        let (w, n) = self
            .checkpoint
            .ok_or_else(|| JsError::new("Save a checkpoint first"))?;
        self.w = w;
        self.tape.truncate(n);
        self.samples.truncate(n);
        self.view()
    }
    pub fn back(&mut self) -> Result<JsValue, JsError> {
        self.tape.pop();
        self.samples.pop();
        self.w = self.tape.last().map_or(self.initial, |(_, w)| *w);
        self.checkpoint = None;
        self.view()
    }
    pub fn verify(&self) -> Result<usize, JsError> {
        self.replay().map_err(|e| JsError::new(&e))
    }
    pub fn export_pack(&self) -> Vec<u8> {
        self.def.bytes.clone()
    }
    pub fn export_project(&self) -> Result<String, JsError> {
        if !self.editable {
            return Err(JsError::new(
                "Imported pack has no authoring overlay source; export its binary instead",
            ));
        }
        // JSON numbers are typed event arguments. Do not let JS parse/stringify
        // turn f32 18.0 into i64 18 before the project reaches the CLI.
        let files: std::collections::BTreeMap<_, _> = self
            .settings
            .files()
            .into_iter()
            .map(|(path, value)| serde_json::to_string_pretty(&value).map(|text| (path, text)))
            .collect::<Result<_, _>>()
            .map_err(|e| JsError::new(&e.to_string()))?;
        serde_json::to_string(&files).map_err(|e| JsError::new(&e.to_string()))
    }
    pub fn resolved_json(&self) -> String {
        serde_json::to_string_pretty(&self.def.pack.view().payload().unwrap().root().to_json())
            .unwrap()
    }
    pub fn geometry(&self, kind: f64, gap: f64) -> Result<JsValue, JsError> {
        let k = integer(kind, 1, 3).map_err(|e| JsError::new(&e))?;
        let x = integer(gap, 20, 220).map_err(|e| JsError::new(&e))?;
        js(geometry_data(k as u8, x))
    }
    pub fn validation_example(&self) -> String {
        let mut files = self.settings.files();
        files.get_mut("characters/relay/states/jab.json").unwrap()["on_hit"]["resource_deltas"]
            [0]["name"] = "undefined_resource".into();
        match authoring::compile(&files) {
            Ok(_) => "ERROR: validation accepted an undefined resource".into(),
            Err(e) => format!("REJECTED by the shared validator: {e}. Current pack unchanged."),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn lab() -> Lab {
        Lab::create(include_bytes!("../dist/packs/lab.fspk").to_vec()).unwrap()
    }
    fn run(g: &mut Lab, n: usize) {
        for _ in 0..n {
            g.tick(0).unwrap();
        }
    }
    fn demo(g: &mut Lab) {
        g.start_demo(0).unwrap();
        run(g, 180);
    }
    #[test]
    fn cli_browser_export_parity_and_atomic_edits() {
        let mut g = lab();
        assert!(g.editable);
        assert_eq!(
            authoring::compile(&authoring::baseline()).unwrap(),
            g.def.bytes
        );
        let bytes = g.def.bytes.clone();
        let w = g.w;
        let mut s = g.settings.clone();
        assert!(s.edit("recovery", "-1").is_err());
        assert_eq!(g.w, w);
        assert_eq!(g.def.bytes, bytes);
        s = g.settings.clone();
        s.recovery = 4;
        g.apply(s).unwrap();
        assert_ne!(g.def.bytes, bytes);
        assert!(g
            .def
            .data
            .moves
            .iter()
            .any(|m| m.id.as_deref() == Some("special~charged")));
        assert_eq!(g.def.state(g.def.ids[0]).recovery, 4);
    }
    #[test]
    fn actual_links_cancels_resources_and_no_false_combo() {
        let mut g = lab();
        demo(&mut g);
        assert!(
            g.w.blocks > 0,
            "bad link must let the dummy guard: {:?}",
            g.view_value()
        );
        assert_eq!(g.w.max_combo, 1);
        let mut s = g.settings.clone();
        s.recovery = 4;
        g.apply(s).unwrap();
        demo(&mut g);
        assert_eq!(g.w.max_combo, 4, "{}", g.view_value());
        assert!(g.w.links > 0 && g.w.cancels >= 2);
        assert_eq!(g.w.p.resources[g.def.energy], 10);
        assert_eq!(g.w.p.resources[g.def.ammo], 1);
        assert!(g.w.event_count > 0);
        assert_eq!(g.replay().unwrap(), 180);
        g.reset_world(1, 74, -1);
        demo(&mut g);
        assert_eq!(g.w.max_combo, 0);
        assert_eq!(g.w.hits, 0);
        assert!(g.w.blocks > 0);
    }
    #[test]
    fn trial_credit_is_hits_not_buttons_or_autoplay() {
        let mut g = lab();
        g.set_trial_inner(3).unwrap();
        demo(&mut g);
        assert_eq!(g.w.max_combo, 4);
        assert!(!g.w.trial_clear);
        g.set_trial_inner(3).unwrap();
        let mut at = 0;
        for _ in 0..180 {
            let targets = route(3).0;
            let cmd = if at < targets.len() {
                let c = targets[at];
                let allow = if at == 1 {
                    g.w.p.current_state == g.def.idle
                } else {
                    can_cancel_to(&g.w.p, &g.def.pack.view(), g.def.ids[c as usize - 1])
                        && (at == 0 || g.w.p.hit_confirmed)
                };
                if allow {
                    at += 1;
                    c
                } else {
                    0
                }
            } else {
                0
            };
            g.tick(cmd).unwrap();
        }
        assert!(g.w.trial_clear, "{}", g.view_value());
        assert_eq!(g.w.trial_progress, 4);
        g.set_trial_inner(0).unwrap();
        g.tick(2).unwrap();
        run(&mut g, 80);
        assert!(g.w.trial_failed);
        assert!(!g.w.trial_clear);
        g.set_trial_inner(0).unwrap();
        g.tick(1).unwrap();
        run(&mut g, 80);
        g.tick(2).unwrap();
        run(&mut g, 40);
        assert!(!g.w.trial_clear);
    }
    #[test]
    fn measured_advantage_geometry_and_reload() {
        for recovery in [0, 4, 12, 30] {
            let mut g = lab();
            let mut settings = g.settings.clone();
            settings.recovery = recovery;
            g.apply(settings).unwrap();
            g.reset_world(0, 74, -1);
            g.start_demo(4).unwrap();
            run(&mut g, 120);
            let p = g
                .tape
                .iter()
                .find(|(_, w)| w.last_action == 1 && w.p.current_state == g.def.idle)
                .unwrap()
                .1
                .tick;
            let q = g
                .tape
                .iter()
                .find(|(_, w)| w.hits > 0 && w.d.current_state == g.def.idle)
                .unwrap()
                .1
                .tick;
            let row = g.metadata_value()["moves"]
                .as_array()
                .unwrap()
                .iter()
                .find(|m| m["id"] == "jab")
                .unwrap()
                .clone();
            assert_eq!(i64::from(q) - i64::from(p), row["on_hit"].as_i64().unwrap());
        }
        let mut close = lab();
        close.reset_world(3, 20, -1);
        close.tick(0).unwrap();
        assert!(close.w.dx - close.w.px >= 36);
        assert!(close.w.px <= 0);
        assert_eq!(close.replay().unwrap(), 1);
        for kind in 1..=3 {
            assert_eq!(geometry_data(kind, 74)["overlap"], true);
            assert_eq!(geometry_data(kind, 220)["overlap"], false);
        }
        let mut g = lab();
        g.start_demo(3).unwrap();
        assert_eq!(g.w.p.resources[g.def.ammo], 0);
        run(&mut g, 60);
        assert_eq!(g.w.p.resources[g.def.ammo], 3);
        assert_eq!(g.w.hits, 0);
        assert!(!g.w.notices.iter().any(|n| n.kind == WHIFF));
        assert_eq!(g.replay().unwrap(), 60);
    }

    #[test]
    fn windows_tags_costs_multihit_and_pack_reimport() {
        let mut g = lab();
        let mut s = g.settings.clone();
        s.recovery = 4;
        s.deny = true;
        g.apply(s.clone()).unwrap();
        demo(&mut g);
        assert!(g.w.max_combo < 4);
        s.deny = false;
        s.tagged = false;
        g.apply(s.clone()).unwrap();
        demo(&mut g);
        assert!(g.w.max_combo < 4);
        s.tagged = true;
        s.ammo = 0;
        g.apply(s.clone()).unwrap();
        let before = g.w.p.resources;
        g.tick(4).unwrap();
        run(&mut g, 10);
        assert_eq!(g.w.p.resources, before);
        s.ammo = 3;
        s.gain = 0;
        g.apply(s).unwrap();
        demo(&mut g);
        assert!(g.w.max_combo < 4);
        g.reset_world(0, 74, -1);
        g.start_demo(1).unwrap();
        run(&mut g, 100);
        assert_eq!(g.w.hits, 2);
        assert_eq!(g.w.damage, 40);
        let mut imported = Lab::create(g.def.bytes.clone()).unwrap();
        assert!(!imported.editable);
        imported.reset_world(0, 74, -1);
        imported.start_demo(1).unwrap();
        run(&mut imported, 100);
        assert_eq!(g.w, imported.w);
        assert!(integer(f64::NAN, 0, 7).is_err());
        assert!(integer(0.5, 0, 7).is_err());
    }
}
