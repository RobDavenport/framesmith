import init, { Arena } from './pkg/framesmith_arena.js';

const $ = (id) => document.getElementById(id);
const canvas = $('arena');
const ctx = canvas.getContext('2d');
const keys = new Map([
  ['KeyA', 1], ['ArrowLeft', 1], ['KeyD', 2], ['ArrowRight', 2],
  ['KeyJ', 4], ['KeyK', 8], ['KeyL', 16], ['KeyU', 32],
  ['KeyW', 64], ['ArrowUp', 64], ['Space', 64], ['KeyS', 128], ['ArrowDown', 128],
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
  game = new Arena(packs[$('fighter').value], packs[$('rival').value], seed, Number($('mode').value));
  state = game.view();
  paused = !run; started = run; accumulator = 0; lastContact = 0;
  $('replay').textContent = 'Ready to compare the complete recorded match, one Rust state at a time.';
  if (run) $('overlay').hidden = true;
  else overlay(`${state.actors[0].name} vs ${state.actors[1].name}`, `${state.actors[0].style.toUpperCase()} — ${styleTip(state.actors[0].style)} First to two rounds.`, 'CHOOSE ABOVE. SETTLE IT BELOW.', 'FIGHT');
  const a = state.actors[0];
  $('tip').textContent = `S: ${a.special} · ↓+S: ${a.anti} · ↓+H: launch · H+S: ${a.super_name} (50)`;
  $('status').textContent = 'L → M → H → S on contact · Jump-ins beat crouch block · Grabs beat guard';
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
    overlay(['', 'You win.', `${state.actors[1].name} wins.`, 'A dead heat.'][state.winner],
      `${state.wins[0]} — ${state.wins[1]} · ${state.actors[0].name} vs ${state.actors[1].name}. Switch styles above or run it back.`,
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
    $(`${p}-charge`).textContent = `${a.meter} / ${a.max_meter}${a.meter >= 50 ? ' · SUPER' : ''}`;
  }
  for (let i=0;i<2;i++) $(`p${i+1}-wins`).textContent = [0,1].map(n => state.wins[i]>n?'◆':'◇').join(' ');
  $('round-number').textContent = `ROUND ${state.round}`;
  $('clock').textContent = Math.ceil(state.remaining / 60).toString().padStart(2, '0');
  $('bout-state').textContent = state.winner ? 'BOUT COMPLETE' : paused ? 'PAUSED' : Number($('mode').value) ? 'LAB SESSION' : 'FIRST TO TWO';
  $('pause').textContent = paused ? 'Resume · P' : 'Pause · P';
  document.querySelector('.burst').classList.toggle('ready', state.actors[0].meter >= 50);
  $('debug').textContent = state.actors.map((a, i) => `${i ? 'CPU' : 'YOU'}  ${a.name_of_move} [${a.id}] · ${a.phase} ${a.frame}/${a.total}`).join('\n') +
    `\nTick ${state.tick} · hitstop ${state.hitstop} · checkpoint ${state.checkpoint}\n` +
    `Hits ${state.stats.hits.join('/')} · blocks ${state.stats.blocks.join('/')} · cancels ${state.stats.cancels.join('/')}\nMeter spent ${state.stats.spent.join('/')} · authored signals ${state.stats.signals.join('/')}`;
  const c = state.contact, visible = c.serial && state.tick - c.tick < 24;
  $('impact').textContent = visible ? c.blocked ? 'GUARDED' : `${c.counter ? 'COUNTER · ' : ''}${c.damage} DAMAGE` : '';
  $('impact').style.color = c.blocked ? '#bed7ff' : state.actors[c.who].color;
  const combo = state.actors[1].combo;
  $('combo').textContent = combo > 1 ? `${combo} HIT / ${state.actors[1].combo_damage}` : '';
  const superFlash = state.super_tick && state.tick - state.super_tick < 25;
  $('round-call').textContent = state.next_round ? `${state.round_winner===3?'DOUBLE KO':state.actors[state.round_winner-1].name.toUpperCase()+' TAKES IT'}` : superFlash ? state.actors[state.super_who].super_name.toUpperCase() : !paused && state.remaining > 3530 ? `ROUND ${state.round} — FIGHT!` : '';
  if (c.kind===2 && visible) $('impact').textContent = `COMMAND GRAB / ${c.damage}`;
  draw();
}
function styleTip(style) { return {shoto:'Fireball space control. Rising anti-air. A tool for every range.',grappler:'Walk them down. Guard-breaking command grabs. Huge reward up close.',zoner:'Long pokes and rail shots. Own the screen; keep them out.',rushdown:'Fast chains and a lunging special. Get in and stay in.'}[style]; }

// The canvas is presentation only. Geometry and combat results are returned by Rust.
function line(x1, y1, x2, y2, color, width) {
  ctx.strokeStyle = color; ctx.lineWidth = width; ctx.lineCap = 'round';
  ctx.beginPath(); ctx.moveTo(x1, y1); ctx.lineTo(x2, y2); ctx.stroke();
}
function disk(x, y, radius, color) { ctx.fillStyle = color; ctx.beginPath(); ctx.arc(x, y, radius, 0, Math.PI * 2); ctx.fill(); }
function robot(a, index) {
  const attack = !['idle','guard','crouch','jump','stun','landing'].includes(a.id);
  const phase = a.phase, crouch = a.crouching, big = a.style === 'grappler', caster = a.style === 'zoner', fast = a.style === 'rushdown';
  const ext = !attack ? 0 : phase === 'startup' ? -.15 : phase === 'active' ? 1 : Math.max(0,1-(a.frame-a.startup-a.active)/9);
  const hurt = state.contact.serial && state.contact.who !== index && !state.contact.blocked && state.tick-state.contact.tick<8;
  const color = hurt ? '#fff6dc' : a.color;
  ctx.save(); ctx.translate(a.x,0);
  ctx.fillStyle='#03091488';ctx.beginPath();ctx.ellipse(0,3,a.width*.8,5,0,0,Math.PI*2);ctx.fill();
  ctx.translate(0,a.y);ctx.scale(a.facing,1);
  const sy=(crouch?52:a.height)/88, sx=a.width/36;
  const stride = !reduced && phase==='ready' && a.y===0 && (index?state.mode===0:input()&3) ? Math.sin(state.tick*.5)*9:0;
  const air=a.y<0, low=a.id==='low', kick=['heavy','air_medium','air_heavy','air_special'].includes(a.id);
  if (fast && attack && ext>0 && !reduced) for(let i=1;i<4;i++) line(-i*14,-40*sy,-i*14-22,-40*sy,a.color+'55',3);
  ctx.save();ctx.scale(sx,sy);
  if (caster) {ctx.fillStyle='#403454';ctx.beginPath();ctx.moveTo(-15,-64);ctx.lineTo(16,-64);ctx.lineTo(28,-16);ctx.lineTo(-26,-16);ctx.fill();line(-24,-18,24,-18,color,3);}
  line(-8,-32,-16+stride,air?-20:-6,'#43546f',big?13:9);
  const footX=kick&&ext>0?(a.reach-12)/sx:18-stride;
  const footY=kick&&ext>0?-47:air?-26:-6;
  line(8,-30,footX,footY,fast?color:'#7991ab',big?13:10);
  line(-23+stride,air?-18:-3,-9+stride,air?-18:-3,color,7);
  line(footX-5,footY+3,footX+11,footY+3,color,7);
  ctx.fillStyle=big?'#67462f':caster?'#2e2941':fast?'#53293d':'#d8e6df';ctx.fillRect(-17,-65,34,35);
  if (a.style==='shoto'){line(-15,-62,7,-37,color,6);line(15,-62,-7,-37,'#324e5b',6);line(-20,-32,20,-32,color,5);line(-6,-31,-17,-13,color,4);}
  else {ctx.fillStyle=color;ctx.fillRect(-18,-65,36,10);ctx.fillRect(-10,-52,20,12);}
  if(big){disk(-19,-58,12,color);disk(19,-58,12,color);line(-15,-33,15,-33,'#e8d39d',7);}
  if(fast){line(-10,-72,-31,-58,color,7);line(-30,-58,-37,-35,color,5);}
  ctx.fillStyle='#b6c1c7';ctx.fillRect(-6,-73,12,10);
  ctx.fillStyle=big?'#75605b':'#273c55';ctx.fillRect(-13,-88,27,20);
  ctx.fillStyle=color;ctx.fillRect(-15,-90,30,6);
  ctx.fillStyle='#f9f5d6';ctx.fillRect(1,-80,14,4);
  if(a.style==='shoto'){line(-14,-84,-32,-77,color,4);line(-31,-77,-39,-81,color,3);}
  if(caster){ctx.strokeStyle=color;ctx.lineWidth=2;ctx.beginPath();ctx.arc(0,-80,20,.4,5.8);ctx.stroke();}
  ctx.restore();
  let hx=26*sx,hy=-50*sy;
  if(a.blocking){hx=26*sx;hy=-70*sy;}
  else if(attack&&!kick){hx=26+ext*Math.max(15,a.reach-33);hy=low?-17:a.id==='anti_air'||a.id==='launch'?-105:-53*sy;}
  const glove=big?12:8;
  line(-14*sx,-59*sy,-24*sx,-37*sy,'#59718c',big?12:8);disk(-24*sx,-35*sy,glove,color);
  line(14*sx,-59*sy,(hx+14)/2,hy+10,'#7f99b1',big?12:9);line((hx+14)/2,hy+10,hx,hy,color,big?14:10);disk(hx,hy,glove,color);
  if(caster && a.id==='special')disk(hx+10,hy,12,'#d7bcff77');
  if(phase==='startup'&&attack){ctx.strokeStyle='#ffe4a6';ctx.lineWidth=2;ctx.beginPath();ctx.arc(hx,hy,14+a.frame/2,-.7,4.4);ctx.stroke();}
  if(phase==='active'&&attack){ctx.strokeStyle=a.id==='burst'?'#fff4b0':color;ctx.lineWidth=4;ctx.beginPath();ctx.arc(kick?a.reach-25:hx-10,kick?-45:hy,26,-1.1,1.1);ctx.stroke();}
  if(big&&a.id==='anti_air'&&phase==='active'){ctx.strokeStyle=color;ctx.lineWidth=4;ctx.beginPath();ctx.ellipse(0,-60,82,24,0,0,Math.PI*2);ctx.stroke();}
  if(a.blocking){ctx.strokeStyle='#b9dbff';ctx.lineWidth=3;ctx.beginPath();ctx.ellipse(28*sx,-50*sy,12,38*sy,0,-1.5,1.5);ctx.stroke();}
  ctx.restore();
  ctx.fillStyle=index?'#ffcfa9':'#c1fff1';ctx.font='bold 10px system-ui';ctx.textAlign='center';ctx.fillText(index?'CPU':'YOU',a.x,a.y-a.height-12);
}
function draw() {
  const bounds=canvas.getBoundingClientRect(),dpr=Math.min(devicePixelRatio||1,2),W=bounds.width,H=bounds.height;
  if(!W||!H)return;
  const pw=Math.round(W*dpr),ph=Math.round(H*dpr);if(canvas.width!==pw||canvas.height!==ph){canvas.width=pw;canvas.height=ph;}
  ctx.setTransform(dpr,0,0,dpr,0,0);
  const bg=ctx.createLinearGradient(0,0,0,H);bg.addColorStop(0,'#10172f');bg.addColorStop(.7,'#413355');bg.addColorStop(1,'#172c3b');ctx.fillStyle=bg;ctx.fillRect(0,0,W,H);
  const floor=H*.84;
  disk(W*.77,H*.21,Math.min(28,H*.1),'#efd6d099');
  for(let i=0;i<18;i++){
    const x=i*W/16-20,h=30+(i*37%89);ctx.fillStyle=i%2?'#17213b':'#1c2742';ctx.fillRect(x,floor*.8-h,W/14,h+H*.3);
    for(let yy=0;yy<h-12;yy+=15)for(let xx=6;xx<W/15;xx+=12){ctx.fillStyle=(i+xx+yy)%4?'#d5908160':'#75d6d770';ctx.fillRect(x+xx,floor*.8-h+yy+7,4,6);}
  }
  ctx.fillStyle='#111b2b';ctx.fillRect(0,floor-27,W,28);line(0,floor-26,W,floor-26,'#ea81ad',3);
  for(let x=0;x<W;x+=70)line(x,floor-25,x,floor,'#536078',3);
  ctx.fillStyle='#172836';ctx.fillRect(0,floor,W,H-floor);line(0,floor,W,floor,'#82c7cc',3);
  for(let x=-W;x<2*W;x+=120)line(W/2+(x-W/2)*.8,floor+3,x,H,'#2d4757',1);
  if(!state)return;
  const span=W<640?Math.max(330,Math.abs(state.actors[1].x-state.actors[0].x)+190):Math.max(730,Math.abs(state.actors[1].x-state.actors[0].x)+200);
  const ceiling=Math.max(...state.actors.map(a=>-a.y+a.height+36));
  const unit=Math.min(W/span,(floor-20)/ceiling),center=(state.actors[0].x+state.actors[1].x)/2;
  const c=state.contact,age=state.tick-c.tick;
  const shake=!reduced&&c.serial&&!c.blocked&&age<9?Math.sin(age*2.9)*(9-age)*.35:0;
  ctx.save();ctx.translate(W/2-center*unit+shake,floor);ctx.scale(unit,unit);
  robot(state.actors[0],0);robot(state.actors[1],1);
  for(const q of state.projectiles){
    const color=state.actors[q.owner].color,x=q.x+q.w/2,y=q.y+q.h/2,dir=state.actors[q.owner].facing;
    line(x-dir*40,y,x,y,color+'55',q.h*.6);ctx.fillStyle=color+'88';ctx.beginPath();ctx.ellipse(x,y,q.w*.65,q.h*.65,0,0,Math.PI*2);ctx.fill();
    ctx.fillStyle='#efffff';ctx.beginPath();ctx.ellipse(x,y,q.w*.35,q.h*.35,0,0,Math.PI*2);ctx.fill();
    if($('boxes').checked){ctx.strokeStyle='#ff8098';ctx.strokeRect(q.x,q.y,q.w,q.h);}
  }
  if(c.serial&&age<15){
    const color=c.blocked?'#b4d9ff':'#ffe7ac';for(let i=0;i<10;i++){const angle=i*Math.PI/5,r=8+age*(reduced?1:2.5);line(c.x+Math.cos(angle)*r,c.y+Math.sin(angle)*r,c.x+Math.cos(angle)*(r+13),c.y+Math.sin(angle)*(r+13),color,3);}
  }
  if($('boxes').checked)for(const a of state.actors)for(const [kind,boxes]of [['hurt',a.hurtboxes],['hit',a.hitboxes]]){ctx.fillStyle=kind==='hit'?'#ff677333':'#75d7ff16';ctx.strokeStyle=kind==='hit'?'#ff7682':'#75d7ff';ctx.lineWidth=1;for(const r of boxes){ctx.fillRect(r.x,r.y,r.w,r.h);ctx.strokeRect(r.x,r.y,r.w,r.h);}}
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
$('play').addEventListener('click', () => { enableSound(); if (state.winner) newMatch(true); else pause(false); canvas.focus({ preventScroll: true }); });
$('pause').addEventListener('click', () => pause());
$('reset').addEventListener('click', () => { newMatch(true); canvas.focus({ preventScroll: true }); });
for(const id of ['mode','fighter','rival']) $(id).addEventListener('change', () => newMatch(false));
$('boxes').addEventListener('change', draw);
async function enableSound() {
  try { if ($('sound').checked) { audio ||= new AudioContext(); await audio.resume(); } }
  catch { $('sound').checked = false; $('status').textContent = 'Audio unavailable; the game remains playable.'; }
}
$('sound').addEventListener('change', enableSound);
$('step').addEventListener('click', () => { pause(true); step(input()); update(); });
$('save').addEventListener('click', () => { pause(true); state = game.checkpoint(); $('replay').textContent = `Full-match checkpoint saved at tick ${state.tick}.`; update(); });
$('restore').addEventListener('click', () => { paused = true; clearInput(); state = game.restore(); lastContact = state.contact.serial; pause(true); $('replay').textContent = `Full-match checkpoint restored to tick ${state.tick}.`; update(); });
$('verify').addEventListener('click', () => {
  pause(true);
  try { const frames = game.verify_replay(); $('replay').textContent = frames ? `PASS · ${frames} frames replayed. Every complete Rust match state is identical.` : 'Record some frames first; an empty replay is not validation.'; }
  catch (error) { $('replay').textContent = `FAIL · ${error}`; }
});
for (const b of document.querySelectorAll('[data-input]')) {
  b.addEventListener('pointerdown', e => { if (!game || paused) return; e.preventDefault(); b.setPointerCapture(e.pointerId); pointers.set(e.pointerId, Number(b.dataset.input)); queued |= Number(b.dataset.input) & 124; showHeld(); });
  b.addEventListener('click', e => { if (e.detail === 0 && game && !paused) queued |= Number(b.dataset.input); });
  const release = e => { pointers.delete(e.pointerId); showHeld(); };
  b.addEventListener('pointerup', release); b.addEventListener('pointercancel', release); b.addEventListener('lostpointercapture', release);
}
document.addEventListener('keydown', e => {
  if (e.target.closest('input,select,summary,button:not([data-input])')) return;
  if (keys.has(e.code)) { e.preventDefault(); if (!paused) { if (!down.has(e.code)) queued |= keys.get(e.code) & 124; down.add(e.code); showHeld(); } }
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
  const roster = ['relay','bulwark','sable','zip'];
  const loaded = await Promise.all([init(), ...roster.map(n=>get(`./packs/${n}.fspk`)), get('./build-info.json', 'json')]);
  packs = Object.fromEntries(roster.map((name,i)=>[name,loaded[i+1]])); build = loaded[5];
  newMatch();
  for (const id of ['play', 'pause', 'reset', 'step', 'save', 'restore', 'verify', 'mode', 'fighter', 'rival']) $(id).disabled = false;
  $('runtime').textContent = 'RUST / WASM / FSPK v2';
  const info = game.pack_info();
  $('provenance').textContent = `${info.map(p => `${p[2]} states / ${p[0]} B / FSPK v${p[1]}`).join(' + ')}\nSeed ${seed} · source ${build.commit.slice(0, 12)}${build.dirty ? ' + local edits' : ''}`;
  $('status').textContent = 'Chain L → M → H → S · Hold away to block · Low beats standing guard';
  if (!build.dirty && /^[0-9a-f]{40}$/.test(build.commit)) $('source').href = `https://github.com/RobDavenport/framesmith/tree/${build.commit}/demo-wasm`;
  requestAnimationFrame(loop);
} catch (error) { fail(error); }
