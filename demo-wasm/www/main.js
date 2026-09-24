import init, { Lab } from './pkg/framesmith_arena.js';
const $ = id => document.getElementById(id), canvas=$('arena'), ctx=canvas.getContext('2d');
const reduced=matchMedia('(prefers-reduced-motion: reduce)').matches;
const names=['','Light','Heavy','Arc','Overdrive','Twin Pulse','Charged Arc','Reload'];
const phases=['ready','startup','active','recovery','hitstun','blockstun'];
const colors=['#55e1ca','#f1cb76','#ff8e94','#76a8df','#c4a6ff','#dfb176'];
const lessons=[
 {title:'Make this combo work.',text:'The dummy guards the first gap. Run it, then shorten Jab recovery until the follow-up links.',copy:'A link waits for recovery to finish. No cancel rule is involved.',fields:['recovery','follow_startup']},
 {title:'Cut the recovery short.',text:'Hit-confirm into Arc instead of waiting. Try a different condition or close the cancel window.',copy:'A cancel leaves a move early. These controls edit the actual tag rule.',fields:['cancel','condition','window_start','window_end']},
 {title:'Earn it. Then spend it.',text:'Normal hits build energy. Arc and the finisher spend ammo; the finisher also needs energy.',copy:'Named pools, caps, requirements and atomic costs. A denied move spends nothing.',fields:['energy','gain','cost','ammo']},
 {title:'One group rule. Many moves.',text:'Remove the chainable tag or add a deny. Watch the available route change.',copy:'Move types carry category tags too. “chainable” is a custom tag, shared by the two normals.',fields:['tagged','deny']},
 {title:'A missed hit is not a bad cancel.',text:'Change spacing, guard policy or Jab reach. Actual collision queries decide the contact.',copy:'Red = active hitbox. Blue = hurtbox. The dummy only guards when it can act.',fields:['distance','dummy','reach']},
 {title:'Data asks. The game responds.',text:'Move the notify; change spark size. See event markers, hitstop, and an optional sound.',copy:'The pack supplies typed events and properties. This host draws the spark and applies resource gains.',fields:['notify_frame','spark_size'],demo:4},
 {title:'Author once. Reuse the result.',text:'Try two distinct hits, an inherited charged move, or an on-use ammo refill. Inspect the real source.',copy:'Shared idle comes from globals with a character override. Charged Arc inherits Arc; Twin Pulse has per-hit stats.',fields:[],variants:true},
 {title:'Take the rules into your game.',text:'Download the current project and binary. Reload the pack: the same data produces the same behavior.',copy:'Every successful edit ran the shared Rust validator and exporter. Invalid drafts leave the current build intact.',fields:[],exports:true}
];
const fields={
 recovery:['Jab recovery',0,30],follow_startup:['Follow startup',2,20],cancel:['Enable cancel','check'],
 condition:['Condition','select',[['hit','On hit'],['block','On block'],['whiff','On whiff'],['always','Always']]],
 window_start:['Window opens',0,60],window_end:['Window closes','number',0,255],energy:['Start energy',0,100],gain:['Energy per hit',0,50],cost:['Finisher cost',0,100],ammo:['Start ammo',0,3],tagged:['Chainable tags','check'],deny:['Deny Follow → Arc','check'],distance:['Dummy distance',20,220],
 dummy:['Guard policy','select',[[0,'Never guard'],[1,'Guard high'],[2,'Guard low'],[3,'Guard after first hit']]],reach:['Jab reach',20,140],notify_frame:['Notify frame',0,30],spark_size:['Spark size',4,48]
};
let lab, workshop, basePack, build, state, meta, mode='sandbox', lesson=0, trial=0, paused=false, demonstrating=false;
let geometryProbe=null, jumpQueued=false;
const held=new Set(), pointers=new Map(), sprites={};
function releaseMotion(){held.clear();pointers.clear();jumpQueued=false;}
function inspection(open){$('workbench').hidden=!open;$('app').classList.toggle('inspecting',open);pause(open);if(!open){geometryProbe=null;$('geometry').value='0';canvas.focus({preventScroll:true});}}
function moving(){return Number([...held].some(c=>direction(c)==='right')||[...pointers.values()].includes('right'))-Number([...held].some(c=>direction(c)==='left')||[...pointers.values()].includes('left'));}
let queue=[],previous=0,accumulator=0,lastHeard=0,audio,files={},clears=new Set();
const setText=(id,value)=>{if($(id).textContent!==String(value))$(id).textContent=value;};
function safe(fn){if(!lab)return;try{return fn();}catch(e){paused=true;setText('edit-message',e.message||String(e));sync();}}
function pause(value=!paused){paused=value;accumulator=0;queue=[];releaseMotion();sync();}
function refreshMetadata(){meta=lab.metadata();try{files=JSON.parse(lab.export_project());}catch{files={};}state=lab.view();renderKnobs();updateInspector();if($('data-dialog').open)renderData();}
function cleanPlayback(){releaseMotion();geometryProbe=null;$('geometry').value='0';queue=[];accumulator=0;lastHeard=0;demonstrating=false;paused=true;setText('replay','Records the entire consumer state, not just move IDs.');setText('edit-message','');}
function retry(){state=lab.reset();cleanPlayback();paused=false;sync();}
function changeMode(next){
 if(next===mode){if(next==='guided')inspection($('workbench').hidden);return;}
 cleanPlayback();
 if(next==='trials'){workshop=lab;lab=new Lab(basePack);state=lab.trial(trial);$('speed').value='1';}
 else if(mode==='trials'){lab.free();lab=workshop;workshop=null;}
 mode=next;refreshMetadata();renderExperiment();inspection(next==='guided');sync();
}
function selectExperiment(index){cleanPlayback();if(mode==='trials'){trial=index;state=lab.trial(trial);}else{lesson=index;state=lab.reset();if(lesson>0&&state.editable&&meta.settings.recovery===12)lab.edit('recovery','4');if(lesson===4)$('boxes').checked=true;}refreshMetadata();renderExperiment();if(mode==='trials')paused=false;sync();}
function renderExperiment(){
 const options=mode==='trials'?meta.trials.map((t,i)=>`${clears.has(i)?'✓ ':''}${i+1}. ${t.name}`):lessons.map((l,i)=>`${i+1}. ${l.title}`);
 $('experiment').replaceChildren(...options.map((x,i)=>new Option(x,i)));$('experiment').value=mode==='trials'?trial:lesson;
 $('trial-select').replaceChildren(...meta.trials.map((t,i)=>new Option(`${i+1}. ${t.name}`,i)));$('trial-select').value=trial;$('trial-ui').hidden=mode!=='trials';
 setText('experiment-label',mode==='trials'?'TRIAL':'EXPERIMENT');
 setText('mission-title',mode==='trials'?meta.trials[trial].name:mode==='sandbox'?'Your data. Your experiment.':lessons[lesson].title);
 setText('mission-text',mode==='trials'?'Watch once, then land the route yourself. Try ¼ speed; the dummy guards every gap.':mode==='sandbox'?'Press 1–4 or tap a move. Experiment settings stay editable; nothing runs a second JS combat model.':lessons[lesson].text);
 for(const b of document.querySelectorAll('[data-mode]'))b.setAttribute('aria-pressed',String(b.dataset.mode===mode));
 $('trial-help').hidden=mode!=='trials';$('variant-tools').hidden=mode==='trials'||!lessons[lesson].variants;$('export-tools').hidden=mode==='trials'||!lessons[lesson].exports;$('geometry-tools').hidden=mode==='trials'||lesson!==4;
 setText('inspector-title',mode==='trials'?'FROZEN RULES. YOUR EXECUTION.':'ONE CHANGE. REAL DATA.');
 setText('inspector-copy',mode==='trials'?'Clear means real uninterrupted hits, using the specified link or cancel.':lessons[lesson].copy);
 setText('run',mode==='trials'?'▶ Watch demo':lessons[lesson].variants?'▶ Run example':'▶ Run sequence');
 $('next').disabled=mode==='trials'?trial===3:lesson===7;
 renderKnobs();updateInspector();
}
function renderKnobs(){
 $('knobs').replaceChildren();if(!meta||mode==='trials')return;
 for(const key of lessons[lesson].fields.filter(k=>state.editable||['distance','dummy'].includes(k))){
  const spec=fields[key],row=document.createElement('div');row.className='knob';
  const label=document.createElement('label');label.htmlFor=`edit-${key}`;label.textContent=spec[0];
  const input=document.createElement(spec[1]==='select'?'select':'input');input.id=`edit-${key}`;input.dataset.field=key;
  if(spec[1]==='select'){for(const [val,name] of spec[2])input.add(new Option(name,val));}
  else if(spec[1]==='check')input.type='checkbox';
  else {input.type=spec[1]==='number'?'number':'range';input.min=spec[1]==='number'?spec[2]:spec[1];input.max=spec[1]==='number'?spec[3]:spec[2];input.step='1';}
  const value=key==='distance'?state.distance:key==='dummy'?state.dummy:meta.settings[key];
  if(input.type==='checkbox')input.checked=value;else input.value=value;
  input.disabled=state.trial>=0||(!state.editable&&!['distance','dummy'].includes(key));
  row.append(label,input);
  if(input.type==='range'){const output=document.createElement('output');output.htmlFor=input.id;output.textContent=value;row.append(output);input.addEventListener('input',()=>output.textContent=input.value);}
  input.addEventListener('change',()=>safe(()=>{
   paused=true;queue=[];
   const value=input.type==='checkbox'?input.checked:key==='condition'?input.value:Number(input.value);
   try{if(key==='distance'||key==='dummy'){state=lab.dummy(key==='dummy'?value:state.dummy,key==='distance'?value:state.distance);}else{lab.edit(key,JSON.stringify(value));}}
   finally{refreshMetadata();sync();}
   lastHeard=0;demonstrating=false;if(Number($('geometry').value)){geometryProbe=lab.geometry(Number($('geometry').value),state.distance);sync();}setText('edit-message',['distance','dummy'].includes(key)?'Consumer setting changed; attempt reset.':`Compiled ${meta.pack_bytes.toLocaleString()} bytes of FSPK in WASM. Attempt reset; data is live.`);
  }));
  $('knobs').append(row);
 }
}
function updateInspector(){
 if(!meta)return;
 const jab=meta.moves.find(m=>m.input==='jab'),follow=meta.moves.find(m=>m.input==='follow');
 setText('mini-stats',`Jab ${jab.startup}/${jab.active}/${jab.recovery} · on hit ${signed(jab.on_hit)}f · Follow ${follow.startup}f startup`);
 setText('edit-state',state.editable?'LIVE FSPK COMPILER':state.trial>=0?'TRIAL LOCK':'IMPORTED BINARY');
 $('reset-edits').disabled=mode==='trials';$('download-project').disabled=!state.editable;
 setText('build-state',`${meta.moves.length} STATES · FSPK v2`);
 $('moves').querySelector('[data-command="4"] small').textContent=meta.moves.find(m=>m.input==='finisher').resolved.costs.map(c=>`${c.amount} ${c.name.toUpperCase()}`).join(' + ');
}
function signed(n){return n==null?'—':n>=0?`+${n}`:String(n);}
function sync(){
 if(!state)return;
 setText('pause',paused?'Resume P':'Pause P');
 setText('energy',`ENERGY ${state.energy} / ${meta.resources.find(r=>r.name==='energy')?.max??100}`);$('meter').value=state.energy;
 setText('ammo',`AMMO ${'●'.repeat(state.ammo)}${'○'.repeat(Math.max(0,3-state.ammo))}`);
 const lastHit=state.notices.findLast(n=>n.kind===meta.trace_kinds.hit),combo=state.combo||lastHit?.aux||0;
 setText('combo',combo);setText('damage',`${state.combo_damage} DAMAGE`);$('combo-result').hidden=combo<2||(!state.combo&&state.tick-(lastHit?.tick??0)>60);
 $('paused-label').hidden=!paused||!$('workbench').hidden||document.querySelector('dialog[open]')!==null;
 $('meter-panel').hidden=!$('frame-meter').checked;
 setText('dummy-caption',['NEVER GUARD','GUARD HIGH','GUARD LOW','GUARD AFTER FIRST HIT'][state.dummy]);
 const p=state.actors[0],d=state.actors[1];setText('dummy-state',d.stun_remaining?`${phases[d.phase].toUpperCase()} ${d.stun_remaining}f`:phases[d.phase].toUpperCase());
 setText('phase',`${p.move_name.toUpperCase()} · ${phases[p.phase].toUpperCase()} ${p.frame}${state.freeze?' · HITSTOP':''}`);setText('frame',`f ${state.tick}`);
 let reason=state.reason;
 if(state.trial<0&&state.blocks>0&&state.dummy===3&&state.links===0)reason='The follow-up got blocked. The dummy recovered first: shorten Jab recovery.';
 if(state.trial<0&&state.hits===0&&state.notices.some(n=>n.kind===meta.trace_kinds.whiff))reason='Whiff: the active boxes never reached the dummy. Try less distance or more reach.';
 if(geometryProbe){reason=`GEOMETRY QUERY: ${geometryProbe.overlap?'OVERLAP':'SEPARATE'}. These are helper inputs, not an attack. Run returns to combat.`;setText('phase','GEOMETRY HELPER · NO COMBAT TICK');}setText('reason',reason);
 const route=mode==='trials'?meta.trials[trial].route:lessons[lesson].variants?[Number($('variant').value)===1?5:Number($('variant').value)===2?6:Number($('variant').value)===3?7:1]:[1,2,3,4];
 const next=mode==='trials'&&!state.trial_failed&&!state.trial_clear?route[state.trial_progress]:0;
 const ready=next?state.available.find(a=>a.command===next):null;
 setText('hint',mode==='trials'?(state.manual?(state.trial_clear?'Clear earned. Retry, or take the next trial.':state.trial_failed?'Retry to start a fresh attempt.':`${ready?.allowed?'NOW':'WAIT'} → ${next} · ${names[next]}. ${ready?.reason||''}`):'DEMONSTRATION ONLY · press Retry or a move to begin your manual attempt.'):`${state.events} authored events · ${state.blocks} blocked contacts · ${paused?'Paused — step or run':'Live'} · no timer-based combo credit`);
 for(const b of $('moves').children){const command=Number(b.dataset.command),a=state.available.find(a=>a.command===command);b.classList.toggle('can',a.allowed);b.classList.toggle('next',command===next);b.title=`${names[command]}: ${a.reason}`;}
 const routeKey=JSON.stringify([route,state.trial_progress,state.trial_clear,mode]);
 if($('route').dataset.key!==routeKey){$('route').dataset.key=routeKey;$('route').replaceChildren();route.forEach((cmd,i)=>{if(i){const e=document.createElement('span');e.className='edge';e.textContent=route[i-1]===1&&cmd===2?'→ LINK →':'⇢ CANCEL ⇢';$('route').append(e);}const n=document.createElement('span');n.className='node'+(mode==='trials'&&i<state.trial_progress?' done':'')+(cmd===next?' expected':'');n.textContent=`${cmd>4?'':cmd+' · '}${names[cmd]}`;$('route').append(n);});}
 if(state.trial_clear){clears.add(trial);setText('clear-count',`${clears.size}/4`);}
 $('trial-call').hidden=mode!=='trials'||(!state.trial_clear&&!state.trial_failed);$('trial-call').classList.toggle('failed',state.trial_failed);setText('trial-call',state.trial_clear?'TRIAL CLEAR ✓':'TRY AGAIN · R / RETRY');
 $('next-trial').hidden=!state.trial_clear||trial===3;$('advance-trial').hidden=!state.trial_clear||trial===3;
 setText('trial-cue',state.trial_failed?'Resetting… R to retry now':state.trial_clear?'Route complete':!state.manual?'DEMONSTRATION · no trial credit':`${ready?.allowed?'NOW':'WAIT'}  ${['','J','K','L','I'][next]??''} · ${names[next]??''}`);
 setText('inputs','Inputs: '+(state.notices.filter(n=>n.kind===meta.trace_kinds.input).slice(-10).map(n=>`${names[n.command]} @${n.tick}`).join(' · ')||'—'));
 draw();if($('frame-meter').checked)drawTimeline();
}
function startDemo(){cleanPlayback();state=lab.demonstrate(mode==='trials'?0:lessons[lesson].variants?Number($('variant').value):lessons[lesson].demo??0);demonstrating=true;paused=false;enableSound();sync();canvas.focus({preventScroll:true});}
function attack(command){geometryProbe=null;$('geometry').value='0';if(!state.manual||state.trial_failed||state.trial_clear){state=lab.reset();lastHeard=0;}demonstrating=false;queue.push(command);if(queue.length>8)queue.shift();paused=false;enableSound();sync();}
function tick(){
 state=lab.step_input(queue.shift()??0,moving(),jumpQueued);jumpQueued=false;playEvents();
 if(state.trial_failed){const failure=state.notices.find(n=>n.kind===13);if(failure&&state.tick-failure.tick>=60&&$('workbench').hidden)retry();}
 if(demonstrating&&!state.auto&&state.actors.every(a=>a.phase===0)){paused=true;demonstrating=false;}
}
function loop(now){const dt=previous?Math.min(100,now-previous):0;previous=now;if(lab&&!paused){try{accumulator+=dt*Number($('speed').value);while(accumulator>=1000/60&&!paused){tick();accumulator-=1000/60;}sync();}catch(e){paused=true;queue=[];setText('error',e.message||String(e));$('error').hidden=false;}}requestAnimationFrame(loop);}
function line(x1,y1,x2,y2,color,width){ctx.strokeStyle=color;ctx.lineWidth=width;ctx.beginPath();ctx.moveTo(x1,y1);ctx.lineTo(x2,y2);ctx.stroke();}
// Consumer clip binding: native phase/frame gates contact, the art does not.
function fighter(a,index){
 let clip='Idle',frame=Math.floor(state.tick/7)%8;
 if(a.phase===4){clip='TakeHit';frame=1+Math.min(2,Math.floor(a.frame/6));}
 else if(a.command&&a.command!==7){
  clip=['follow','finisher','multi'].includes(a.animation)?'Attack2':'Attack1';
  frame=a.phase===1?Math.min(3,Math.floor(a.frame*4/Math.max(1,a.startup))):a.phase===2?4:5;
  if(a.phase===3&&a.frame-a.startup-a.active>=2){frame=Math.max(0,3-Math.floor((a.frame-a.startup-a.active-2)*4/Math.max(1,a.recovery-2)));}
 }else if(a.y<0){clip=a.vy<0?'Jump':'Fall';frame=Math.floor(state.tick/5)%2;}
 else if(a.walking){clip='Run';frame=Math.floor(state.tick/4)%8;}
 const image=sprites[clip];if(!image)return;
 ctx.save();ctx.translate(a.x,a.y);ctx.scale(a.facing,1);ctx.imageSmoothingEnabled=false;
 if(index)ctx.filter='hue-rotate(165deg) saturate(.65) brightness(1.15)';
 // Fixed feet pivot (95,122) for every original 200px cell, including effects.
 ctx.drawImage(image,frame*200,0,200,200,-95*1.7,-122*1.7,340,340);ctx.restore();
 if(a.phase===5){ctx.strokeStyle='#a9e8f4';ctx.lineWidth=2;ctx.beginPath();ctx.arc(a.x-a.facing*12,a.y-50,33,a.facing>0?Math.PI*.6:-Math.PI*.4,a.facing>0?Math.PI*1.4:Math.PI*.4);ctx.stroke();}
}
function sizeCanvas(element){const r=element.getBoundingClientRect(),ratio=Math.min(devicePixelRatio||1,2);if(!r.width||!r.height)return null;const w=Math.round(r.width*ratio),h=Math.round(r.height*ratio);if(element.width!==w||element.height!==h){element.width=w;element.height=h;}const c=element.getContext('2d');c.setTransform(ratio,0,0,ratio,0,0);return[c,r.width,r.height];}
function draw(){
 const sized=sizeCanvas(canvas);if(!sized)return;const[,W,H]=sized;
 const floor=H-62;ctx.fillStyle='#6d7774';ctx.fillRect(0,0,W,H);ctx.fillStyle='#737e7a';ctx.fillRect(0,86,W,floor-86);
 for(let x=0;x<W;x+=80)line(x,86,x,floor,'#64716b',1);
 for(let y=86;y<floor;y+=80)line(0,y,W,y,'#64716b',1);
 ctx.fillStyle='#87948a';ctx.fillRect(0,floor,W,H-floor);line(0,floor,W,floor,'#c2c8b8',3);
 for(let x=-W;x<W*2;x+=120)line(W/2+(x-W/2)*.8,floor,x,H,'#6b7b71',1);
 line(0,floor+22,W,floor+22,'#a1ad9f',1);ctx.fillStyle='#d9ddc722';ctx.fillRect(W/2-2,86,4,floor-86);
 if(!state)return;
 const narrow=W<600,span=narrow?Math.max(240,Math.abs(state.distance)+110):470;
 const unit=Math.min((W-36)/span,(H-118)/225),center=narrow?(state.actors[0].x+state.actors[1].x)/2:55,originX=W/2-center*unit,originY=floor;
 ctx.save();ctx.translate(originX,originY);ctx.scale(unit,unit);
 for(const a of state.actors){ctx.fillStyle='#27392f44';ctx.beginPath();ctx.ellipse(a.x,2,27,5,0,0,Math.PI*2);ctx.fill();}
 fighter(state.actors[0],0);fighter(state.actors[1],1);
 const spark=state.notices.slice().reverse().find(n=>n.kind===meta.trace_kinds.event&&n.aux===meta.event_kinds.spark),age=spark?state.tick-spark.tick:999;
 if(spark&&age<11){const r=Math.max(4,spark.value)*Math.max(4,Math.min(48,Number(meta.character.properties.spark_size)||18))/18+(reduced?0:age),x=state.actors[1].x-state.actors[0].facing*18,y=-51;for(let i=0;i<8;i++){const a=i*Math.PI/4;line(x+Math.cos(a)*r*.35,y+Math.sin(a)*r*.35,x+Math.cos(a)*r,y+Math.sin(a)*r,'#ffe5ae',2);}}
 if($('boxes').checked)for(const a of state.actors)for(const[kind,boxes]of [['hurt',a.hurtboxes],['hit',a.hitboxes],['push',a.pushboxes]])for(const b of boxes){ctx.fillStyle=kind==='hit'?'#ff67732a':kind==='push'?'#ffd27d08':'#75d7ff16';ctx.strokeStyle=kind==='hit'?'#ff929e':kind==='push'?'#ffd27d':'#80d5ff';ctx.lineWidth=1;ctx.setLineDash(kind==='push'?[3,3]:[]);ctx.fillRect(a.x+(a.facing===1?b.x:-b.x-b.w),a.y+b.y,b.w,b.h);ctx.strokeRect(a.x+(a.facing===1?b.x:-b.x-b.w),a.y+b.y,b.w,b.h);}ctx.setLineDash([]);
 if(geometryProbe)for(const [i,b]of geometryProbe.shapes.entries()){const color=i?'#ffa76b':'#55e1ca';ctx.strokeStyle=color;ctx.fillStyle=color+'33';ctx.lineWidth=2;if(b.kind==='circle'){ctx.beginPath();ctx.arc(b.x,b.y,b.r,0,Math.PI*2);ctx.fill();ctx.stroke();}else if(b.kind==='aabb'){ctx.fillRect(b.x,b.y,b.w,b.h);ctx.strokeRect(b.x,b.y,b.w,b.h);}else{line(b.x1,b.y1,b.x2,b.y2,color+'66',b.r*2);line(b.x1,b.y1,b.x2,b.y2,color,2);}}
 ctx.restore();
}
function drawTimeline(){
 const sized=sizeCanvas($('timeline'));if(!sized||!state)return;const[c,W,H]=sized;c.fillStyle='#0b1720';c.fillRect(0,0,W,H);const left=42,rows=[6,H/2+2],height=H/2-5,samples=state.samples,width=(W-left-3)/Math.max(80,samples.length);
 c.font='9px ui-monospace,monospace';c.fillStyle='#b9c9d4';c.fillText('RELAY',1,rows[0]+height-2);c.fillText('DUMMY',1,rows[1]+height-2);
 samples.forEach((s,i)=>{for(const[row,phase]of [[0,s.player],[1,s.dummy]]){c.fillStyle=colors[phase];c.globalAlpha=s.freeze?.38:.85;c.fillRect(left+i*width,rows[row],Math.max(1,width-.5),height);}c.globalAlpha=1;if(s.contact){c.fillStyle='#fff4ce';c.fillRect(left+i*width,0,Math.max(1,width),3);}});
 c.globalAlpha=1;c.strokeStyle='#ffffff88';c.beginPath();c.moveTo(left+samples.length*width,0);c.lineTo(left+samples.length*width,H);c.stroke();
}
async function enableSound(){try{if($('sound').checked){audio ||= new AudioContext();await audio.resume();}}catch{$('sound').checked=false;}}
let noise;
function playEvents(){
 for(const n of state.notices){
  if(n.seq<=lastHeard)continue;lastHeard=n.seq;
  if(n.kind!==meta.trace_kinds.event||!audio||!$('sound').checked)continue;
  if(!noise){noise=audio.createBuffer(1,Math.ceil(audio.sampleRate*.15),audio.sampleRate);const data=noise.getChannelData(0);let seed=19;for(let i=0;i<data.length;i++){seed=(Math.imul(seed,1664525)+1013904223)>>>0;data[i]=seed/2147483648-1;}}
  const hit=n.aux===meta.event_kinds.spark,t=audio.currentTime,source=audio.createBufferSource(),filter=audio.createBiquadFilter(),gain=audio.createGain();
  source.buffer=noise;filter.type='bandpass';filter.frequency.setValueAtTime(hit?2400:4200,t);filter.frequency.exponentialRampToValueAtTime(hit?800:1200,t+.1);filter.Q.value=.7;
  gain.gain.setValueAtTime(hit?.18:.07,t);gain.gain.exponentialRampToValueAtTime(.001,t+.1);source.connect(filter).connect(gain).connect(audio.destination);source.start(t);source.stop(t+.12);
  if(hit){const body=audio.createOscillator(),g=audio.createGain();body.frequency.setValueAtTime(140,t);body.frequency.exponentialRampToValueAtTime(45,t+.07);g.gain.setValueAtTime(.11,t);g.gain.exponentialRampToValueAtTime(.001,t+.085);body.connect(g).connect(audio.destination);body.start(t);body.stop(t+.09);}
 }
}
function renderData(){
 const query=$('filter').value.toLowerCase(),sort=$('sort').value;const moves=meta.moves.filter(m=>`${m.name} ${m.id} ${m.tags.join(' ')}`.toLowerCase().includes(query)).sort((a,b)=>sort==='name'?a.name.localeCompare(b.name):sort==='startup'?a[sort]-b[sort]:b[sort]-a[sort]);
 $('frame-rows').replaceChildren(...moves.map(m=>{const row=document.createElement('tr');for(const text of [m.name,`${m.startup} / ${m.active} / ${m.recovery}`,m.resolved.hits?.map(h=>h.damage).join(' + ')??m.damage,`${signed(m.on_hit)} / ${signed(m.on_block)}`,m.tags.join(' · ')]){const td=document.createElement('td');td.textContent=text;row.append(td);}return row;}));
 const rules=meta.cancel_table.tag_rules;setText('graph',rules.map(r=>`${r.from} → ${r.to} [${r.on}, frames ${r.after_frame??0}–${r.before_frame??255}]`).join('   |   ')+(Object.keys(meta.cancel_table.deny??{}).length?'   ·   Explicit deny: '+JSON.stringify(meta.cancel_table.deny):''));
 renderDataItems();
}
function renderDataItems(){const old=$('data-item').value,source=$('data-kind').value==='source';const options=source?Object.keys(files):meta.moves.map(m=>m.id);$('data-item').replaceChildren(...options.map(x=>new Option(x,x)));if(options.includes(old))$('data-item').value=old;else if(!source)$('data-item').value='jab';$('data-item').disabled=!['source','resolved'].includes($('data-kind').value);renderJson();}
function renderJson(){const kind=$('data-kind').value,item=$('data-item').value;let value=kind==='source'?files[item]:kind==='cancels'?meta.cancel_table:kind==='resources'?meta.character:kind==='registry'?meta.rules:kind==='geometry'?geometryProbe:meta.moves.find(m=>m.id===item)?.resolved;setText('json',typeof value==='string'?value:JSON.stringify(value??{note:'This imported runtime pack has no editable overlay source.'},null,2));setText('source-note',kind==='source'?'The project ZIP contains these exact source files, including globals and the variant overlay.':'Resolved fields are decoded from the current binary. Authoring support is not a promise that this host executes every field.');}
function download(name,bytes,type){const url=URL.createObjectURL(new Blob([bytes],{type})),a=document.createElement('a');a.href=url;a.download=name;document.body.append(a);a.click();a.remove();setTimeout(()=>URL.revokeObjectURL(url),1000);}
// Stored ZIP: tiny text-only project, no dependency or compression needed.
function projectZip(project){
 const parts=[],central=[],encoder=new TextEncoder();let offset=0;
 const crc=bytes=>{let c=0xffffffff;for(const b of bytes){c^=b;for(let n=0;n<8;n++)c=c&1?(c>>>1)^0xedb88320:c>>>1;}return(c^0xffffffff)>>>0;};
 for(const[path,value]of Object.entries(project)){
  if(!/^[a-zA-Z0-9_~./-]+$/.test(path)||path.startsWith('/')||path.split('/').includes('..'))throw new Error('Unsafe project filename');
  if(typeof value!=='string')throw new Error('Project files must be native-serialized text');
  const name=encoder.encode('framesmith-lab/'+path),bytes=encoder.encode(value+'\n'),sum=crc(bytes),header=new Uint8Array(30+name.length),v=new DataView(header.buffer);
  v.setUint32(0,0x04034b50,true);v.setUint16(4,20,true);v.setUint16(6,0x800,true);v.setUint16(12,33,true);v.setUint32(14,sum,true);v.setUint32(18,bytes.length,true);v.setUint32(22,bytes.length,true);v.setUint16(26,name.length,true);header.set(name,30);parts.push(header,bytes);
  const entry=new Uint8Array(46+name.length),c=new DataView(entry.buffer);c.setUint32(0,0x02014b50,true);c.setUint16(4,20,true);c.setUint16(6,20,true);c.setUint16(8,0x800,true);c.setUint16(14,33,true);c.setUint32(16,sum,true);c.setUint32(20,bytes.length,true);c.setUint32(24,bytes.length,true);c.setUint16(28,name.length,true);c.setUint32(42,offset,true);entry.set(name,46);central.push(entry);offset+=header.length+bytes.length;
 }
 const centralSize=central.reduce((s,b)=>s+b.length,0),end=new Uint8Array(22),e=new DataView(end.buffer);e.setUint32(0,0x06054b50,true);e.setUint16(8,central.length,true);e.setUint16(10,central.length,true);e.setUint32(12,centralSize,true);e.setUint32(16,offset,true);return new Blob([...parts,...central,end],{type:'application/zip'});
}
$('close-tools').onclick=()=>inspection(false);
$('trial-select').onchange=()=>safe(()=>selectExperiment(Number($('trial-select').value)));
$('advance-trial').onclick=()=>safe(()=>selectExperiment(Math.min(3,trial+1)));
$('frame-meter').onchange=sync;
for(const b of document.querySelectorAll('[data-motion]')){
 b.onpointerdown=e=>{e.preventDefault();b.setPointerCapture(e.pointerId);const dir=b.dataset.motion;if(dir==='jump')jumpQueued=true;else pointers.set(e.pointerId,dir);paused=false;};
 for(const type of ['pointerup','pointercancel','lostpointercapture'])b.addEventListener(type,e=>pointers.delete(e.pointerId));
 b.onclick=e=>{if(e.detail===0&&b.dataset.motion==='jump'){jumpQueued=true;paused=false;}};
}
canvas.onpointerdown=()=>{canvas.focus({preventScroll:true});pause(false);};
for(const b of document.querySelectorAll('[data-mode]'))b.addEventListener('click',()=>safe(()=>changeMode(b.dataset.mode)));
for(const b of document.querySelectorAll('[data-command]')){b.addEventListener('pointerdown',e=>{e.preventDefault();safe(()=>attack(Number(b.dataset.command)));});b.addEventListener('click',e=>{if(e.detail===0)safe(()=>attack(Number(b.dataset.command)));});}
$('experiment').addEventListener('change',()=>safe(()=>selectExperiment(Number($('experiment').value))));
$('next').addEventListener('click',()=>safe(()=>selectExperiment(Math.min(mode==='trials'?3:7,(mode==='trials'?trial:lesson)+1))));
$('next-trial').addEventListener('click',()=>safe(()=>selectExperiment(Math.min(3,trial+1))));
$('run').addEventListener('click',()=>safe(startDemo));$('retry').addEventListener('click',()=>safe(retry));$('pause').addEventListener('click',()=>safe(()=>pause()));
$('boxes').addEventListener('change',draw);$('sound').addEventListener('change',enableSound);$('speed').addEventListener('change',()=>accumulator=0);
$('geometry').addEventListener('change',()=>safe(()=>{pause(true);geometryProbe=Number($('geometry').value)?lab.geometry(Number($('geometry').value),state.distance):null;sync();}));
$('variant').addEventListener('change',()=>safe(()=>{state=lab.reset();cleanPlayback();sync();}));
$('step').addEventListener('click',()=>safe(()=>{pause(true);tick();sync();}));$('back').addEventListener('click',()=>safe(()=>{pause(true);state=lab.back();lastHeard=0;sync();}));
$('save').addEventListener('click',()=>safe(()=>{pause(true);state=lab.checkpoint();setText('replay',`Full checkpoint saved at frame ${state.tick}.`);}));
$('restore').addEventListener('click',()=>safe(()=>{pause(true);state=lab.restore();lastHeard=state.notices.at(-1)?.seq??0;setText('replay',`Full checkpoint restored to frame ${state.tick}.`);sync();}));
$('verify').addEventListener('click',()=>safe(()=>{pause(true);const n=lab.verify();setText('replay',`PASS · all ${n} complete world states replayed identically.`);}));
$('reset-edits').addEventListener('click',()=>safe(()=>{const next=new Lab(basePack);lab.free();lab=next;cleanPlayback();refreshMetadata();sync();setText('edit-message','Restored the original editable example.');}));
$('download-pack').addEventListener('click',()=>safe(()=>download('relay.fspk',lab.export_pack(),'application/octet-stream')));
$('download-project').addEventListener('click',()=>safe(()=>download('framesmith-lab.zip',projectZip(JSON.parse(lab.export_project())),'application/zip')));
$('import-pack').addEventListener('change',async()=>{const file=$('import-pack').files[0];if(!file)return;if(file.size>512*1024){setText('edit-message','Pack exceeds the 512 KiB lab limit.');$('import-pack').value='';return;}const bytes=new Uint8Array(await file.arrayBuffer());safe(()=>{const next=new Lab(bytes);lab.free();lab=next;cleanPlayback();refreshMetadata();sync();setText('edit-message',state.editable?'Loaded the editable baseline.':'Imported binary: simulation uses this pack; overlay editing is locked to prevent losing its unknown source.');});$('import-pack').value='';});
$('invalid').addEventListener('click',()=>safe(()=>setText('edit-message',lab.validation_example())));
$('inspect').addEventListener('click',()=>safe(()=>{pause(true);renderData();$('data-dialog').showModal();}));$('about').addEventListener('click',()=>{if(lab)pause(true);$('coverage-dialog').showModal();});
for(const b of document.querySelectorAll('[data-close]'))b.addEventListener('click',()=>$(b.dataset.close).close());
for(const id of ['filter','sort'])$(id).addEventListener('input',()=>safe(renderData));$('data-kind').addEventListener('change',()=>safe(renderDataItems));$('data-item').addEventListener('change',()=>safe(renderJson));
const direction=code=>['KeyA','ArrowLeft'].includes(code)?'left':['KeyD','ArrowRight'].includes(code)?'right':null;
document.addEventListener('keyup',e=>{const dir=direction(e.code);if(dir)held.delete(e.code);});
document.addEventListener('keydown',e=>{
 if(!lab||e.repeat||e.ctrlKey||e.metaKey||e.altKey||document.querySelector('dialog[open]')||e.target.closest('input,select,textarea,summary'))return;
 const dir=direction(e.code),cmd=/^(?:Digit|Numpad)([1-4])$/.exec(e.code),key={KeyJ:1,KeyK:2,KeyL:3,KeyI:4}[e.code];
 if(dir){e.preventDefault();held.add(e.code);paused=false;}
 else if(['KeyW','ArrowUp','Space'].includes(e.code)&&!e.target.closest('button')){e.preventDefault();jumpQueued=true;paused=false;}
 else if(cmd||key){e.preventDefault();safe(()=>attack(key||Number(cmd[1])));}
 else if(e.code==='KeyP'){e.preventDefault();pause();}
 else if(e.code==='KeyR'){e.preventDefault();safe(retry);}
 else if(e.code==='Escape'){e.preventDefault();if(!$('workbench').hidden)inspection(false);else pause();}
 else if(e.code==='F1'){e.preventDefault();inspection($('workbench').hidden);}
 else if(e.code==='F2'){e.preventDefault();$('frame-meter').checked=!$('frame-meter').checked;sync();}
 else if(e.code==='F3'){e.preventDefault();$('boxes').checked=!$('boxes').checked;draw();}
 else if(e.code==='Period'){e.preventDefault();safe(()=>{pause(true);tick();sync();});}
});
window.addEventListener('blur',()=>{if(lab)pause(true);});document.addEventListener('visibilitychange',()=>{if(document.hidden&&lab)pause(true);});
new ResizeObserver(()=>{draw();drawTimeline();}).observe(canvas);
Object.defineProperty(window,'framesmith',{value:Object.freeze({snapshot:()=>structuredClone(state),metadata:()=>structuredClone(meta),paused:()=>paused,mode:()=>mode})});
try{
 const get=async(url,binary)=>{const response=await fetch(url,{cache:'no-store',signal:AbortSignal.timeout(20000)});if(!response.ok)throw new Error(`${url}: HTTP ${response.status}`);return binary?new Uint8Array(await response.arrayBuffer()):response.json();};
 const art=Promise.all(['Idle','Run','Jump','Fall','Attack1','Attack2','TakeHit'].map(async name=>{const image=new Image();image.src=`./assets/${name}.png`;await image.decode();sprites[name]=image;}));
 const loaded=await Promise.all([init(),get('./packs/lab.fspk',true),get('./build-info.json'),art]);basePack=loaded[1];build=loaded[2];lab=new Lab(basePack);lab.edit('recovery','4');lab.dummy(0,130);refreshMetadata();renderExperiment();
 for(const b of document.querySelectorAll('#moves button,#run,#retry,#pause,#experiment'))b.disabled=false;
 $('app').setAttribute('aria-busy','false');setText('status',`${build.commit.slice(0,7)}${build.dirty?' · local preview':''}`);
 if(!build.dirty&&/^[0-9a-f]{40}$/.test(build.commit))$('source').href=`https://github.com/RobDavenport/framesmith/tree/${build.commit}/demo-wasm`;
 sync();requestAnimationFrame(loop);
}catch(e){setText('error',`The lab could not load: ${e.message||e}. Reload to retry.`);$('error').hidden=false;setText('mission-text','The real runtime is unavailable. No substitute simulation is running.');$('app').setAttribute('aria-busy','false');}
