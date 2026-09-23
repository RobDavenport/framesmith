import init, { Arena } from './pkg/framesmith_arena.js';

const $ = (id) => document.getElementById(id);
const canvas = $('arena');
const ctx = canvas.getContext('2d');
const keys = new Map([
  ['KeyA', 1], ['ArrowLeft', 1], ['KeyD', 2], ['ArrowRight', 2],
  ['KeyJ', 4], ['KeyK', 8], ['KeyL', 16], ['Space', 32],
]);
const down = new Set();
const pointers = new Map();
const seed = 0xf5a17;
const reduced = matchMedia('(prefers-reduced-motion: reduce)').matches;
let game, packs, state, build, audio;
let queued = 0;
let paused = true, started = false, accumulator = 0, previous = 0, lastContact = 0;

function fail(error) {
  paused = true;
  $('error').hidden = false;
  $('error').textContent = `Arena could not run: ${error?.message || error}. Reload to retry; details are in the browser console.`;
  $('status').textContent = 'Runtime error — no simulated fallback.';
  console.error(error);
}
function input() {
  let bits = 0;
  for (const code of down) bits |= keys.get(code) || 0;
  for (const bit of pointers.values()) bits |= bit;
  return bits;
}
function clearInput() { down.clear(); pointers.clear(); queued = 0; showHeld(); }
function showHeld() {
  const mask = input();
  document.querySelectorAll('[data-input]').forEach(b => b.classList.toggle('held', !!(mask & Number(b.dataset.input))));
}
function overlay(title, text, label, action) {
  $('overlay').hidden = false;
  $('overlay-title').textContent = title;
  $('overlay-text').textContent = text;
  $('overlay-label').textContent = label;
  $('play').textContent = action;
}
function newMatch(run = false) {
  clearInput();
  game?.free();
  game = new Arena(packs[0], packs[1], seed, Number($('mode').value));
  state = game.view();
  paused = !run; started = run; accumulator = 0; lastContact = 0;
  $('replay').textContent = 'Ready to compare the complete recorded match, one Rust state at a time.';
  if (run) $('overlay').hidden = true;
  else overlay('Make every frame count.', 'Beat Bulwark. Close the gap, guard the big windup, then land a jab → heavy → burst.', 'YOU ARE RELAY / CYAN', 'Enter arena');
  update();
}
function pause(value = !paused) {
  if (!game || state.winner) return;
  paused = value; accumulator = 0; clearInput();
  if (paused) overlay('Paused', 'The simulation clock is stopped. Open Runtime lab to step, checkpoint or verify.', 'TIME TO THINK', 'Resume bout');
  else { started = true; $('overlay').hidden = true; canvas.focus({ preventScroll: true }); }
  update();
}
function sound(contact) {
  if (!$('sound').checked || !audio || audio.state !== 'running') return;
  const oscillator = audio.createOscillator(), gain = audio.createGain();
  oscillator.type = contact.blocked ? 'sine' : 'triangle';
  oscillator.frequency.setValueAtTime(contact.blocked ? 700 : 160, audio.currentTime);
  oscillator.frequency.exponentialRampToValueAtTime(contact.blocked ? 400 : 42, audio.currentTime + .09);
  gain.gain.setValueAtTime(.035, audio.currentTime);
  gain.gain.exponentialRampToValueAtTime(.0001, audio.currentTime + .11);
  oscillator.connect(gain); gain.connect(audio.destination);
  oscillator.start(); oscillator.stop(audio.currentTime + .12);
}
function step(mask) {
  state = game.step(mask);
  if (state.contact.serial !== lastContact) { sound(state.contact); lastContact = state.contact.serial; }
  if (state.winner) {
    paused = true; clearInput();
    overlay(['', 'You win.', 'Bulwark wins.', 'A dead heat.'][state.winner],
      state.remaining ? 'Knockout. Try a rematch, or verify the complete recorded fight in Runtime lab.' : 'Time is up. Remaining health percentage decides the result.',
      state.remaining ? 'KNOCKOUT' : 'TIME LIMIT', 'Rematch');
    $('status').textContent = `Bout complete · ${state.tick} simulated frames · replay available`;
  }
}
function update() {
  if (!state) return;
  for (const [i, a] of state.actors.entries()) {
    const p = `p${i + 1}`;
    $(`${p}-name`).textContent = a.name.toUpperCase();
    $(`${p}-hp`).textContent = `${a.hp} / ${a.max_hp}`;
    $(`${p}-health`).max = a.max_hp; $(`${p}-health`).value = a.hp;
    $(`${p}-meter`).max = a.max_meter; $(`${p}-meter`).value = a.meter;
    $(`${p}-charge`).textContent = `${a.meter} / ${a.max_meter}${a.meter >= 50 ? ' · BURST' : ''}`;
  }
  $('clock').textContent = Math.ceil(state.remaining / 60).toString().padStart(2, '0');
  $('bout-state').textContent = state.winner ? 'BOUT COMPLETE' : paused ? 'PAUSED' : Number($('mode').value) ? 'LAB SESSION' : 'LIVE DUEL';
  $('pause').textContent = paused ? 'Resume · P' : 'Pause · P';
  document.querySelector('.burst').classList.toggle('ready', state.actors[0].meter >= 50);
  $('debug').textContent = state.actors.map((a, i) => `${i ? 'CPU' : 'YOU'}  ${a.name_of_move} [${a.id}] · ${a.phase} ${a.frame}/${a.total}`).join('\n') +
    `\nTick ${state.tick} · hitstop ${state.hitstop} · checkpoint ${state.checkpoint}\n` +
    `Hits ${state.stats.hits.join('/')} · blocks ${state.stats.blocks.join('/')} · cancels ${state.stats.cancels.join('/')}\nMeter spent ${state.stats.spent.join('/')} · authored signals ${state.stats.signals.join('/')}`;
  const c = state.contact, visible = c.serial && state.tick - c.tick < 24;
  $('impact').textContent = visible ? c.blocked ? 'GUARDED' : `${c.counter ? 'COUNTER · ' : ''}${c.damage} DAMAGE` : '';
  $('impact').style.color = c.blocked ? '#bed7ff' : state.actors[c.who].color;
  draw();
}

// The canvas is presentation only. Geometry and combat results are returned by Rust.
function line(x1, y1, x2, y2, color, width) {
  ctx.strokeStyle = color; ctx.lineWidth = width; ctx.lineCap = 'round';
  ctx.beginPath(); ctx.moveTo(x1, y1); ctx.lineTo(x2, y2); ctx.stroke();
}
function disk(x, y, radius, color) { ctx.fillStyle = color; ctx.beginPath(); ctx.arc(x, y, radius, 0, Math.PI * 2); ctx.fill(); }
function robot(a, index) {
  const direction = index ? -1 : 1;
  const attacking = ['light', 'heavy', 'burst'].includes(a.id);
  const phase = a.phase;
  let extension = 0;
  if (attacking) extension = phase === 'startup' ? -.2 * a.frame / Math.max(1, a.startup) : phase === 'active' ? 1 : Math.max(0, 1 - (a.frame - a.startup - a.active) / 8);
  const recoil = phase === 'stun' ? -6 : extension < 0 ? -3 : extension * 5;
  const isHurt = state.contact.serial && state.contact.who !== index && !state.contact.blocked && state.tick - state.contact.tick < 9;
  const metal = isHurt ? '#f7f1df' : a.color;
  ctx.save(); ctx.translate(a.x, 0); ctx.scale(direction, 1);
  ctx.fillStyle = '#02080c88'; ctx.beginPath(); ctx.ellipse(0, 2, 30, 5, 0, 0, Math.PI * 2); ctx.fill();
  const walk = phase === 'ready' && (index ? !paused && state.mode === 0 : !!(input() & 3));
  const stride = walk && !reduced ? Math.sin(state.tick * .45) * 8 : 0;
  line(-8, -31, -16 + stride, -7, '#536975', 10); line(8, -31, 18 - stride, -7, '#80939a', 10);
  line(-22 + stride, -3, -9 + stride, -3, metal, 6); line(12 - stride, -3, 28 - stride, -3, metal, 6);
  ctx.translate(recoil, 0);
  ctx.fillStyle = '#253845'; ctx.fillRect(-18, -62, 33, 34);
  ctx.fillStyle = metal; ctx.fillRect(-19, -65, 38, 9); ctx.fillRect(-14, -53, 27, 19);
  ctx.fillStyle = '#0b252b'; ctx.fillRect(-10, -48, 19, 8);
  disk(0, -44, 3, '#edf7ef');
  ctx.fillStyle = '#627b86'; ctx.fillRect(-7, -71, 13, 8);
  ctx.fillStyle = '#20323d'; ctx.fillRect(-16, -89, 33, 22);
  ctx.fillStyle = metal; ctx.fillRect(-15, -89, 30, 5);
  ctx.fillStyle = '#b8f3ed'; ctx.fillRect(1, -81, 16, 5);
  if (index) { ctx.fillStyle = metal; ctx.fillRect(-20, -62, 9, 26); ctx.fillRect(-13, -95, 7, 7); }
  line(-15, -59, -24, -38, '#526e78', 9); disk(-24, -35, 7, metal);
  let handX = 28, handY = -49;
  if (a.blocking) { handX = 24; handY = -73; }
  else if (attacking) { handX = 25 + extension * Math.max(15, a.reach - 32); handY = -54; }
  line(13, -59, (handX + 12) / 2, handY + 9, '#80969e', 10);
  line((handX + 12) / 2, handY + 9, handX, handY, metal, 10);
  disk(handX, handY, a.id === 'heavy' ? 10 : 8, metal);
  if (phase === 'startup') {
    ctx.strokeStyle = '#ffe09b'; ctx.lineWidth = 2;
    ctx.beginPath(); ctx.arc(handX, handY, 14 + 6 * a.frame / Math.max(1, a.startup), -.7, 4.4); ctx.stroke();
  }
  if (phase === 'active') {
    ctx.strokeStyle = a.id === 'burst' ? '#f9f4bd' : metal; ctx.lineWidth = a.id === 'burst' ? 6 : 3;
    ctx.beginPath(); ctx.arc(handX - 10, handY, a.id === 'burst' ? 28 : 18, -.8, .8); ctx.stroke();
    line(handX - 30, handY + 5, handX - 5, handY + 5, '#ffffff88', 2);
  }
  if (a.blocking) {
    ctx.strokeStyle = '#9ac4ee'; ctx.fillStyle = '#9ac4ee18'; ctx.lineWidth = 2;
    ctx.beginPath(); ctx.ellipse(27, -52, 10, 36, 0, -1.5, 1.5); ctx.stroke();
  }
  ctx.restore();
  if (['startup', 'active', 'recovery'].includes(phase)) {
    ctx.fillStyle = phase === 'startup' ? '#ffe09b' : phase === 'active' ? '#fff5dd' : '#a7b4b9';
    ctx.font = 'bold 11px ui-monospace, monospace'; ctx.textAlign = 'center';
    ctx.fillText(phase === 'startup' ? 'WINDUP' : phase === 'active' ? 'STRIKE' : 'RECOVERY', a.x, -110 - index * 16);
  }
}
function draw() {
  const bounds = canvas.getBoundingClientRect(), dpr = Math.min(devicePixelRatio || 1, 2);
  const W = bounds.width, H = bounds.height;
  if (!W || !H) return;
  const pw = Math.round(W * dpr), ph = Math.round(H * dpr);
  if (canvas.width !== pw || canvas.height !== ph) { canvas.width = pw; canvas.height = ph; }
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  const bg = ctx.createLinearGradient(0, 0, 0, H); bg.addColorStop(0, '#142831'); bg.addColorStop(1, '#101a22');
  ctx.fillStyle = bg; ctx.fillRect(0, 0, W, H);
  const floor = H * .79;
  ctx.strokeStyle = '#2e444d'; ctx.lineWidth = 1;
  for (let x = 20; x < W; x += 80) { ctx.beginPath(); ctx.moveTo(x, 0); ctx.lineTo(x, floor); ctx.stroke(); }
  for (let y = 50; y < floor; y += 60) { ctx.beginPath(); ctx.moveTo(0, y); ctx.lineTo(W, y); ctx.stroke(); }
  ctx.fillStyle = '#173238'; ctx.fillRect(0, floor - 5, W, 5);
  ctx.fillStyle = '#243c43'; ctx.fillRect(0, floor, W, 3);
  for (let x = -W; x < W * 2; x += 90) line(W / 2 + (x - W / 2) * .7, floor + 3, x, H, '#24373e', 1);
  ctx.fillStyle = '#79999818'; ctx.font = `900 ${Math.min(70, W / 11)}px system-ui`; ctx.textAlign = 'center';
  ctx.fillText('FRAME / SMITH', W / 2, floor - H * .34);
  if (!state) return;
  const span = W < 640 ? Math.max(360, state.actors[1].x - state.actors[0].x + 210) : 1000;
  const center = Math.max(span / 2, Math.min(1000 - span / 2, (state.actors[0].x + state.actors[1].x) / 2));
  const unit = W / span;
  ctx.save(); ctx.translate(W / 2 - center * unit, floor); ctx.scale(unit, unit);
  robot(state.actors[0], 0); robot(state.actors[1], 1);
  const c = state.contact, age = state.tick - c.tick;
  if (c.serial && age < 12) {
    const x = state.actors[1 - c.who].x, color = c.blocked ? '#aedaff' : '#ffe6b4';
    for (let i = 0; i < 7; i++) {
      const angle = i * Math.PI * 2 / 7, r = 10 + age * 2;
      line(x + Math.cos(angle) * r, -55 + Math.sin(angle) * r, x + Math.cos(angle) * (r + 8), -55 + Math.sin(angle) * (r + 8), color, 2);
    }
  }
  if ($('boxes').checked) for (const a of state.actors) for (const [kind, boxes] of [['hurt', a.hurtboxes], ['hit', a.hitboxes]]) {
    ctx.fillStyle = kind === 'hit' ? '#ff677333' : '#75d7ff16'; ctx.strokeStyle = kind === 'hit' ? '#ff7682' : '#75d7ff'; ctx.lineWidth = 1;
    for (const r of boxes) { ctx.fillRect(r.x, r.y, r.w, r.h); ctx.strokeRect(r.x, r.y, r.w, r.h); }
  }
  ctx.restore();
}
function loop(now) {
  try {
    const elapsed = previous ? Math.min(now - previous, 100) : 0; previous = now;
    if (game && started && !paused) {
      accumulator += elapsed;
      while (accumulator >= 1000 / 60 && !paused) { const mask = input() | queued; queued = 0; step(mask); accumulator -= 1000 / 60; }
      update();
    }
  } catch (error) { fail(error); }
  requestAnimationFrame(loop);
}
$('play').addEventListener('click', () => { if (state.winner) newMatch(true); else pause(false); canvas.focus({ preventScroll: true }); });
$('pause').addEventListener('click', () => pause());
$('reset').addEventListener('click', () => { newMatch(true); canvas.focus({ preventScroll: true }); });
$('mode').addEventListener('change', () => newMatch(false));
$('boxes').addEventListener('change', draw);
$('sound').addEventListener('change', async () => {
  try { if ($('sound').checked) { audio ||= new AudioContext(); await audio.resume(); } }
  catch { $('sound').checked = false; $('status').textContent = 'Audio unavailable; the game remains playable.'; }
});
$('step').addEventListener('click', () => { pause(true); step(input()); update(); });
$('save').addEventListener('click', () => { pause(true); state = game.checkpoint(); $('replay').textContent = `Full-match checkpoint saved at tick ${state.tick}.`; update(); });
$('restore').addEventListener('click', () => { paused = true; clearInput(); state = game.restore(); lastContact = state.contact.serial; pause(true); $('replay').textContent = `Full-match checkpoint restored to tick ${state.tick}.`; update(); });
$('verify').addEventListener('click', () => {
  pause(true);
  try { const frames = game.verify_replay(); $('replay').textContent = frames ? `PASS · ${frames} frames replayed. Every complete Rust match state is identical.` : 'Record some frames first; an empty replay is not validation.'; }
  catch (error) { $('replay').textContent = `FAIL · ${error}`; }
});
for (const b of document.querySelectorAll('[data-input]')) {
  b.addEventListener('pointerdown', e => { if (!game || paused) return; e.preventDefault(); b.setPointerCapture(e.pointerId); pointers.set(e.pointerId, Number(b.dataset.input)); queued |= Number(b.dataset.input) & 28; showHeld(); });
  b.addEventListener('click', e => { if (e.detail === 0 && game && !paused) queued |= Number(b.dataset.input); });
  const release = e => { pointers.delete(e.pointerId); showHeld(); };
  b.addEventListener('pointerup', release); b.addEventListener('pointercancel', release); b.addEventListener('lostpointercapture', release);
}
document.addEventListener('keydown', e => {
  if (e.target.closest('input,select,button,summary')) return;
  if (keys.has(e.code)) { e.preventDefault(); if (!paused) { if (!down.has(e.code)) queued |= keys.get(e.code) & 28; down.add(e.code); showHeld(); } }
  if (!e.repeat && game) {
    if (e.code === 'KeyP' || e.code === 'Escape') { e.preventDefault(); pause(); }
    if (e.code === 'KeyR') { e.preventDefault(); newMatch(true); }
  }
});
document.addEventListener('keyup', e => { down.delete(e.code); showHeld(); });
window.addEventListener('blur', () => { if (started && game && !paused) pause(true); else clearInput(); });
document.addEventListener('visibilitychange', () => { if (document.hidden && game && !paused) pause(true); });
new ResizeObserver(draw).observe(canvas);
Object.defineProperty(window, 'framesmith', { value: Object.freeze({ snapshot: () => structuredClone(state), paused: () => paused }) });

try {
  const get = async (url, type) => { const r = await fetch(url, { cache: 'no-store' }); if (!r.ok) throw new Error(`${url}: HTTP ${r.status}`); return type === 'json' ? r.json() : new Uint8Array(await r.arrayBuffer()); };
  const loaded = await Promise.all([init(), get('./packs/relay.fspk'), get('./packs/bulwark.fspk'), get('./build-info.json', 'json')]);
  packs = loaded.slice(1, 3); build = loaded[3];
  newMatch();
  for (const id of ['play', 'pause', 'reset', 'step', 'save', 'restore', 'verify', 'mode']) $(id).disabled = false;
  $('runtime').textContent = 'RUST / WASM / FSPK v2';
  const info = game.pack_info();
  $('provenance').textContent = `${info.map(p => `${p[2]} states / ${p[0]} B / FSPK v${p[1]}`).join(' + ')}\nSeed ${seed} · source ${build.commit.slice(0, 12)}${build.dirty ? ' + local edits' : ''}`;
  $('status').textContent = `Binary-only runtime · source ${build.commit.slice(0, 7)}${build.dirty ? ' + local edits' : ''} · seed ${seed}`;
  if (!build.dirty && /^[0-9a-f]{40}$/.test(build.commit)) $('source').href = `https://github.com/RobDavenport/framesmith/tree/${build.commit}/demo-wasm`;
  requestAnimationFrame(loop);
} catch (error) { fail(error); }
