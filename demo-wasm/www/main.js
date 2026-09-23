import init, { Arena } from './pkg/framesmith_arena.js';

const $ = (id) => document.getElementById(id);
const canvas = $('arena');
const ctx = canvas.getContext('2d');
const keys = new Map([
  ['KeyA', 1], ['ArrowLeft', 1], ['KeyD', 2], ['ArrowRight', 2],
  ['KeyJ', 32], ['KeyZ', 32], ['KeyK', 64], ['KeyX', 64],
  ['KeyW', 4], ['ArrowUp', 4], ['Space', 16], ['KeyS', 8], ['ArrowDown', 8],
]);
const down = new Set();
const pointers = new Map();
const seed = 0xf5a17;
const reduced = matchMedia('(prefers-reduced-motion: reduce)').matches;
let game, packs, state, build, audio;
let queued = 0, queuedDirection = 0;
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
function queuePress(bit) {
  if (bit & 112) { queued |= bit; queuedDirection = input() & 15; }
}
function consumeInput() {
  // A quick directional attack must keep the modifier even if released before RAF.
  const mask = queued ? (input() & 112) | queued | queuedDirection : input();
  queued = queuedDirection = 0;
  return mask;
}
function clearInput() { down.clear(); pointers.clear(); queued = queuedDirection = 0; showHeld(); }
function showHeld() {
  const mask = input();
  document.querySelectorAll('[data-input]').forEach(b => b.setAttribute('aria-pressed', String(!!(mask & Number(b.dataset.input)))));
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
  else overlay(`${state.actors[0].name} vs ${state.actors[1].name}`, `${state.actors[0].style.toUpperCase()} — ${styleTip(state.actors[0].style)} Three stocks. Build damage, then knock your rival out.`, 'CHOOSE ABOVE. SETTLE IT BELOW.', 'FIGHT');
  const a = state.actors[0];
  $('tip').textContent = `K: ${a.special} · ↑+K: recover · ↓+K: ${a.super_name} (50 energy)`;
  $('status').textContent = 'Build % to launch farther · Space twice: double jump · ↓+Space: drop through';
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
  oscillator.type = 'triangle';
  oscillator.frequency.setValueAtTime(160, audio.currentTime);
  oscillator.frequency.exponentialRampToValueAtTime(42, audio.currentTime + .09);
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
      `${state.actors[0].stocks} — ${state.actors[1].stocks} stocks · Switch fighters above or run it back.`,
      state.remaining ? 'KNOCKOUT' : 'TIME LIMIT', 'Rematch');
    $('status').textContent = `Bout complete · ${state.tick} simulated frames · replay available`;
  }
}
function update() {
  if (!state) return;
  for (const [i,a] of state.actors.entries()) {
    const p=`p${i+1}`;
    $(`${p}-name`).textContent=a.name.toUpperCase();
    $(`${p}-damage`).textContent=`${a.damage}%`;
    $(`${p}-damage`).classList.toggle('danger',a.damage>=100);
    $(`${p}-stocks`).textContent='●'.repeat(a.stocks)+'○'.repeat(3-a.stocks);
    $(`${p}-stocks`).setAttribute('aria-label',`${a.stocks} stocks remaining`);
    $(`${p}-meter`).max=a.max_meter;$(`${p}-meter`).value=a.meter;
    $(`${p}-charge`).textContent=`${a.meter} ENERGY${a.meter>=50?' · BURST READY':''}`;
  }
  const seconds=Math.ceil(state.remaining/60);
  $('clock').textContent=`${Math.floor(seconds/60)}:${String(seconds%60).padStart(2,'0')}`;
  $('bout-state').textContent=state.winner?'MATCH COMPLETE':paused?'PAUSED':state.mode?'PRACTICE':'THREE STOCKS';
  $('pause').textContent=paused?'Resume · P':'Pause · P';
  document.querySelector('.special').classList.toggle('ready',state.actors[0].meter>=50);
  $('debug').textContent=state.actors.map((a,i)=>`${i?'CPU':'YOU'} ${a.id} ${a.frame}/${a.total} · platform ${a.platform} · jumps ${a.jumps_remaining} · recovery ${a.recovery_ready}`).join('\n')+
    `\nTick ${state.tick} · hitstop ${state.hitstop} · checkpoint ${state.checkpoint}\nHits ${state.stats.hits.join('/')} · cancels ${state.stats.cancels.join('/')} · ring-outs ${state.stats.kos.join('/')}\nEnergy spent ${state.stats.spent.join('/')} · authored signals ${state.stats.signals.join('/')}`;
  const c=state.contact,visible=c.serial&&state.tick-c.tick<24;
  $('impact').textContent=visible?`${c.kind===2?'THROW · ':c.counter?'COUNTER · ':''}+${c.damage}%`:'';
  $('impact').style.color=state.actors[c.who].color;
  $('combo').textContent=state.actors[1].combo>1?`${state.actors[1].combo} HIT`:'';
  const burst=state.super_tick&&state.tick-state.super_tick<25;
  $('round-call').textContent=state.ko_tick&&state.tick-state.ko_tick<65?'RING OUT!':burst?state.actors[state.super_who].super_name.toUpperCase():!paused&&state.tick<65?'GO!':'';
  draw();
}
function styleTip(style) { return {shoto:'Fireballs and rising strikes. The all-rounder.',grappler:'Heavyweight throws. Get close and send them flying.',zoner:'Long pokes and rail shots. Build damage from a distance.',rushdown:'Fast movement, quick chains and a lunging special.'}[style]; }

// The canvas is presentation only. Geometry and combat results are returned by Rust.
function line(x1, y1, x2, y2, color, width) {
  ctx.strokeStyle = color; ctx.lineWidth = width; ctx.lineCap = 'round';
  ctx.beginPath(); ctx.moveTo(x1, y1); ctx.lineTo(x2, y2); ctx.stroke();
}
function disk(x, y, radius, color) { ctx.fillStyle = color; ctx.beginPath(); ctx.arc(x, y, radius, 0, Math.PI * 2); ctx.fill(); }
function robot(a, index, unit) {
  if(a.respawn>0)return;
  const attack = !['idle','guard','crouch','jump','stun','landing'].includes(a.id);
  const phase = a.phase, crouch = a.crouching, big = a.style === 'grappler', caster = a.style === 'zoner', fast = a.style === 'rushdown';
  const ext = !attack ? 0 : phase === 'startup' ? -.15 : phase === 'active' ? 1 : Math.max(0,1-(a.frame-a.startup-a.active)/9);
  const hurt = state.contact.serial && state.contact.who !== index && state.tick-state.contact.tick<8;
  const color = hurt ? '#fff6dc' : a.color;
  ctx.save(); ctx.translate(a.x,a.y);ctx.scale(a.facing,1);
  if(a.invulnerable){ctx.strokeStyle='#e6eaff88';ctx.lineWidth=2;ctx.beginPath();ctx.ellipse(0,-a.height/2,a.width*.9,a.height*.65,0,0,Math.PI*2);ctx.stroke();}
  const sy=(crouch?52:a.height)/88, sx=a.width/36;
  const stride = !reduced && phase==='ready' && a.platform>=0 && Math.abs(a.vx)>0 ? Math.sin(state.tick*.5)*9:0;
  const air=a.platform<0, low=a.id==='low', kick=['heavy','air_medium','air_heavy','air_special'].includes(a.id);
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
  if(attack&&!kick){hx=26+ext*Math.max(15,a.reach-33);hy=low?-17:['anti_air','launch','air_up'].includes(a.id)?-105:-53*sy;}
  const glove=big?12:8;
  line(-14*sx,-59*sy,-24*sx,-37*sy,'#59718c',big?12:8);disk(-24*sx,-35*sy,glove,color);
  line(14*sx,-59*sy,(hx+14)/2,hy+10,'#7f99b1',big?12:9);line((hx+14)/2,hy+10,hx,hy,color,big?14:10);disk(hx,hy,glove,color);
  if(caster && a.id==='special')disk(hx+10,hy,12,'#d7bcff77');
  if(phase==='startup'&&attack){ctx.strokeStyle='#ffe4a6';ctx.lineWidth=2;ctx.beginPath();ctx.arc(hx,hy,14+a.frame/2,-.7,4.4);ctx.stroke();}
  if(phase==='active'&&attack){ctx.strokeStyle=a.id==='burst'?'#fff4b0':color;ctx.lineWidth=4;ctx.beginPath();ctx.arc(kick?a.reach-25:hx-10,kick?-45:hy,26,-1.1,1.1);ctx.stroke();}
  if(big&&a.id==='anti_air'&&phase==='active'){ctx.strokeStyle=color;ctx.lineWidth=4;ctx.beginPath();ctx.ellipse(0,-60,82,24,0,0,Math.PI*2);ctx.stroke();}
  ctx.restore();
  ctx.fillStyle=index?'#ffcfa9':'#c1fff1';ctx.font=`bold ${Math.max(10,10/unit)}px system-ui`;ctx.textAlign='center';ctx.fillText(index?'CPU':'YOU',a.x,a.y-a.height-12);
}
function draw() {
  const bounds=canvas.getBoundingClientRect(),dpr=Math.min(devicePixelRatio||1,2),W=bounds.width,H=bounds.height;
  if(!W||!H)return;
  const pw=Math.round(W*dpr),ph=Math.round(H*dpr);if(canvas.width!==pw||canvas.height!==ph){canvas.width=pw;canvas.height=ph;}
  ctx.setTransform(dpr,0,0,dpr,0,0);
  const bg=ctx.createLinearGradient(0,0,0,H);bg.addColorStop(0,'#10172f');bg.addColorStop(.7,'#413355');bg.addColorStop(1,'#172c3b');ctx.fillStyle=bg;ctx.fillRect(0,0,W,H);
  const floor=H*.84;
  disk(W*.78,H*.22,Math.min(35,H*.12),'#efd6d099');
  for(let i=0;i<18;i++){
    const x=i*W/16-20,h=30+(i*37%89);ctx.fillStyle=i%2?'#17213b':'#1c2742';ctx.fillRect(x,floor*.8-h,W/14,h+H*.3);
    for(let yy=0;yy<h-12;yy+=15)for(let xx=6;xx<W/15;xx+=12){ctx.fillStyle=(i+xx+yy)%4?'#d5908140':'#75d6d740';ctx.fillRect(x+xx,floor*.8-h+yy+7,4,6);}
  }
  if(!state)return;
  // Fixed arena overview: fighters never drag or zoom one another's camera.
  const unit=Math.min((W-24)/800,(H-40)/760),originX=(W-800*unit)/2,originY=(H-760*unit)/2+530*unit;
  const c=state.contact,age=state.tick-c.tick;
  ctx.save();ctx.translate(originX,originY);ctx.scale(unit,unit);
  ctx.strokeStyle='#ff9ea044';ctx.lineWidth=2;ctx.setLineDash([10,12]);ctx.strokeRect(state.blast[0],state.blast[1],state.blast[2]-state.blast[0],state.blast[3]-state.blast[1]);ctx.setLineDash([]);
  for(const [i,p] of state.platforms.entries()){
    const width=p.right-p.left,depth=i?10:20;
    ctx.fillStyle=i?'#274957':'#2e3d57';ctx.fillRect(p.left,p.top,width,depth);
    line(p.left,p.top,p.right,p.top,i?'#91f1dd':'#e5c0ff',4);
    line(p.left+6,p.top+depth,p.right-6,p.top+depth,'#182439',4);
    for(let x=p.left+12;x<p.right-10;x+=30)line(x,p.top+5,x+11,p.top+depth-2,'#567589',2);
    if(!i){ctx.fillStyle='#18253e';ctx.beginPath();ctx.moveTo(p.left+20,p.top+22);ctx.lineTo(p.right-20,p.top+22);ctx.lineTo(p.right-95,p.top+65);ctx.lineTo(p.left+95,p.top+65);ctx.fill();
      for(const x of [p.left+115,p.right-115]){disk(x,p.top+63,11,'#94c9ff44');line(x-10,p.top+68,x+10,p.top+68,'#92e8ff',3);}}
  }
  robot(state.actors[0],0,unit);robot(state.actors[1],1,unit);
  for(const q of state.projectiles){
    const color=state.actors[q.owner].color,x=q.x+q.w/2,y=q.y+q.h/2,dir=state.actors[q.owner].facing;
    line(x-dir*40,y,x,y,color+'55',q.h*.6);ctx.fillStyle=color+'88';ctx.beginPath();ctx.ellipse(x,y,q.w*.65,q.h*.65,0,0,Math.PI*2);ctx.fill();
    ctx.fillStyle='#efffff';ctx.beginPath();ctx.ellipse(x,y,q.w*.35,q.h*.35,0,0,Math.PI*2);ctx.fill();
    if($('boxes').checked){ctx.strokeStyle='#ff8098';ctx.strokeRect(q.x,q.y,q.w,q.h);}
  }
  if(c.serial&&age<15){
    const color='#ffe7ac';for(let i=0;i<10;i++){const angle=i*Math.PI/5,r=8+age*(reduced?1:2.5);line(c.x+Math.cos(angle)*r,c.y+Math.sin(angle)*r,c.x+Math.cos(angle)*(r+13),c.y+Math.sin(angle)*(r+13),color,3);}
  }
  if($('boxes').checked)for(const a of state.actors)for(const [kind,boxes]of [['hurt',a.hurtboxes],['hit',a.hitboxes]]){ctx.fillStyle=kind==='hit'?'#ff677333':'#75d7ff16';ctx.strokeStyle=kind==='hit'?'#ff7682':'#75d7ff';ctx.lineWidth=1;for(const r of boxes){ctx.fillRect(r.x,r.y,r.w,r.h);ctx.strokeRect(r.x,r.y,r.w,r.h);}}
  ctx.restore();
}
function loop(now) {
  try {
    const elapsed = previous ? Math.min(now - previous, 100) : 0; previous = now;
    if (game && started && !paused) {
      accumulator += elapsed;
      while (accumulator >= 1000 / 60 && !paused) { step(consumeInput()); accumulator -= 1000 / 60; }
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
  b.addEventListener('pointerdown', e => { if (!game || paused) return; e.preventDefault(); b.setPointerCapture(e.pointerId); pointers.set(e.pointerId, Number(b.dataset.input)); queuePress(Number(b.dataset.input)); showHeld(); });
  b.addEventListener('click', e => { if (e.detail === 0 && game && !paused) { queued |= Number(b.dataset.input); queuedDirection = input() & 15; } });
  const release = e => { pointers.delete(e.pointerId); showHeld(); };
  b.addEventListener('pointerup', release); b.addEventListener('pointercancel', release); b.addEventListener('lostpointercapture', release);
}
document.addEventListener('keydown', e => {
  if (e.target.closest('input,select,summary,button:not([data-input])')) return;
  if (e.target.closest('button[data-input]') && (e.code === 'Space' || e.code === 'Enter')) return;
  if (keys.has(e.code)) { e.preventDefault(); if (!paused) { const fresh=!down.has(e.code); down.add(e.code); if(fresh)queuePress(keys.get(e.code)); showHeld(); } }
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
  $('status').textContent = 'Build % to launch farther · Direction + J/K changes attacks · Stay on the platforms';
  if (!build.dirty && /^[0-9a-f]{40}$/.test(build.commit)) $('source').href = `https://github.com/RobDavenport/framesmith/tree/${build.commit}/demo-wasm`;
  requestAnimationFrame(loop);
} catch (error) { fail(error); }
