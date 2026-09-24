import init, { Lab } from './pkg/framesmith_arena.js';
const $ = id => document.getElementById(id), canvas=$('arena'), ctx=canvas.getContext('2d');
const reduced=matchMedia('(prefers-reduced-motion: reduce)').matches;
const names=['','Jab','Follow-up','Arc','Finisher','Twin Pulse','Charged Arc','Reload'];
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
let lab, workshop, basePack, build, state, meta, mode='guided', lesson=0, trial=0, paused=true, demonstrating=false;
let geometryProbe=null;
let queue=[],previous=0,accumulator=0,lastHeard=0,audio,files={},clears=new Set();
const setText=(id,value)=>{if($(id).textContent!==String(value))$(id).textContent=value;};
function safe(fn){if(!lab)return;try{return fn();}catch(e){paused=true;setText('edit-message',e.message||String(e));sync();}}
function pause(value=!paused){paused=value;accumulator=0;queue=[];sync();}
function refreshMetadata(){meta=lab.metadata();try{files=JSON.parse(lab.export_project());}catch{files={};}state=lab.view();renderKnobs();updateInspector();if($('data-dialog').open)renderData();}
function cleanPlayback(){geometryProbe=null;$('geometry').value='0';queue=[];accumulator=0;lastHeard=0;demonstrating=false;paused=true;setText('replay','Records the entire consumer state, not just move IDs.');setText('edit-message','');}
function retry(){state=lab.reset();cleanPlayback();sync();}
function changeMode(next){
 if(next===mode)return;
 cleanPlayback();
 if(next==='trials'){workshop=lab;lab=new Lab(basePack);state=lab.trial(trial);$('speed').value='0.25';}
 else if(mode==='trials'){lab.free();lab=workshop;workshop=null;}
 mode=next;refreshMetadata();renderExperiment();sync();
}
function selectExperiment(index){cleanPlayback();if(mode==='trials'){trial=index;state=lab.trial(trial);}else{lesson=index;state=lab.reset();if(lesson>0&&state.editable&&meta.settings.recovery===12)lab.edit('recovery','4');if(lesson===4)$('boxes').checked=true;}refreshMetadata();renderExperiment();sync();}
function renderExperiment(){
 const options=mode==='trials'?meta.trials.map((t,i)=>`${clears.has(i)?'✓ ':''}${i+1}. ${t.name}`):lessons.map((l,i)=>`${i+1}. ${l.title}`);
 $('experiment').replaceChildren(...options.map((x,i)=>new Option(x,i)));$('experiment').value=mode==='trials'?trial:lesson;
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
 setText('pause',paused?'▶ Play':'Ⅱ Pause');
 setText('energy',`ENERGY ${state.energy} / ${meta.resources.find(r=>r.name==='energy')?.max??100}`);$('meter').value=state.energy;
 setText('ammo',`AMMO ${'●'.repeat(state.ammo)}${'○'.repeat(Math.max(0,3-state.ammo))}`);
 setText('combo',state.max_combo);setText('damage',`${state.damage} damage · ${state.links} links / ${state.cancels} cancels`);
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
 $('next-trial').hidden=!state.trial_clear||trial===3;
 setText('inputs','Inputs: '+(state.notices.filter(n=>n.kind===meta.trace_kinds.input).slice(-10).map(n=>`${names[n.command]} @${n.tick}`).join(' · ')||'—'));
 draw();drawTimeline();
}
function startDemo(){cleanPlayback();state=lab.demonstrate(mode==='trials'?0:lessons[lesson].variants?Number($('variant').value):lessons[lesson].demo??0);demonstrating=true;paused=false;enableSound();sync();canvas.focus({preventScroll:true});}
function attack(command){geometryProbe=null;$('geometry').value='0';if(!state.manual){state=lab.reset();lastHeard=0;}demonstrating=false;queue.push(command);if(queue.length>8)queue.shift();paused=false;enableSound();sync();}
function tick(){state=lab.step(queue.shift()??0);playEvents();if(state.trial_clear||state.trial_failed){paused=true;queue=[];}if(demonstrating&&!state.auto&&state.actors.every(a=>a.phase===0)){paused=true;demonstrating=false;}if(state.recorded>=state.limit){paused=true;setText('edit-message','60-second recording limit reached. Retry starts a fresh buffer.');}}
function loop(now){const dt=previous?Math.min(100,now-previous):0;previous=now;if(lab&&!paused){try{accumulator+=dt*Number($('speed').value);while(accumulator>=1000/60&&!paused){tick();accumulator-=1000/60;}sync();}catch(e){paused=true;queue=[];setText('error',e.message||String(e));$('error').hidden=false;}}requestAnimationFrame(loop);}
// The original demo's procedural mannequin; pose follows authoritative phase/frame.
function line(x1,y1,x2,y2,color,width){ctx.strokeStyle=color;ctx.lineWidth=width;ctx.lineCap='round';ctx.beginPath();ctx.moveTo(x1,y1);ctx.lineTo(x2,y2);ctx.stroke();}
function disk(x,y,r,color){ctx.fillStyle=color;ctx.beginPath();ctx.arc(x,y,r,0,Math.PI*2);ctx.fill();}
function robot(a,index){
 const phase=phases[a.phase],attack=a.command>0&&a.command!==7,ext=!attack?0:phase==='startup'?-.15:phase==='active'?1:Math.max(0,1-(a.frame-a.startup-a.active)/9);
 const boxes=a.hitboxes,reach=boxes.length?Math.max(...boxes.map(b=>b.x+b.w)):a.command===3||a.command===6?115:76;
 const color=a.color,hurt=a.phase===4,guard=a.phase===5||index&&state.dummy>0&&a.phase===0&&state.hits>0;
 ctx.save();ctx.translate(a.x,a.y);ctx.scale(a.facing,1);if(hurt)ctx.rotate(-.09);
 line(-8,-32,-16,-6,'#43546f',9);line(8,-30,18,-6,'#7991ab',10);line(-23,-3,-9,-3,color,7);line(13,-3,29,-3,color,7);
 ctx.fillStyle=index?'#67513f':'#d8e6df';ctx.fillRect(-17,-65,34,35);line(-15,-62,7,-37,color,6);line(15,-62,-7,-37,'#324e5b',6);line(-20,-32,20,-32,color,5);
 ctx.fillStyle='#b6c1c7';ctx.fillRect(-6,-73,12,10);ctx.fillStyle='#273c55';ctx.fillRect(-13,-88,27,20);ctx.fillStyle=color;ctx.fillRect(-15,-90,30,6);ctx.fillStyle='#f9f5d6';ctx.fillRect(1,-80,14,4);line(-14,-84,-32,-77,color,4);line(-31,-77,-39,-81,color,3);
 let hx=guard?23:26+ext*Math.max(15,reach-33),hy=guard?-73:-53;
 line(-14,-59,-24,-37,'#59718c',8);disk(-24,-35,8,color);line(14,-59,(hx+14)/2,hy+10,'#7f99b1',9);line((hx+14)/2,hy+10,hx,hy,color,10);disk(hx,hy,8,color);
 if(attack&&phase==='startup'){ctx.strokeStyle='#f1cb76';ctx.lineWidth=2;ctx.beginPath();ctx.arc(hx,hy,14,-.7,4.4);ctx.stroke();}
 if(attack&&phase==='active'){ctx.strokeStyle=a.command===4?'#ffdf99':color;ctx.lineWidth=4;ctx.beginPath();ctx.arc(hx-10,hy,26,-1.1,1.1);ctx.stroke();if([3,4,5,6].includes(a.command)){line(hx,hy,reach,hy,color+'66',18);disk(reach-10,hy,13,color+'88');}}
 if(guard){ctx.strokeStyle='#eeb98b99';ctx.lineWidth=2;ctx.beginPath();ctx.arc(10,-49,42,-1.2,1.2);ctx.stroke();}
 ctx.restore();
}
function sizeCanvas(element){const r=element.getBoundingClientRect(),ratio=Math.min(devicePixelRatio||1,2);if(!r.width||!r.height)return null;const w=Math.round(r.width*ratio),h=Math.round(r.height*ratio);if(element.width!==w||element.height!==h){element.width=w;element.height=h;}const c=element.getContext('2d');c.setTransform(ratio,0,0,ratio,0,0);return[c,r.width,r.height];}
function draw(){
 const sized=sizeCanvas(canvas);if(!sized)return;const[,W,H]=sized;const bg=ctx.createLinearGradient(0,0,0,H);bg.addColorStop(0,'#132235');bg.addColorStop(1,'#263954');ctx.fillStyle=bg;ctx.fillRect(0,0,W,H);
 for(let x=0;x<W;x+=36)line(x,70,x,H,'#b1d4e507',1);for(let y=82;y<H;y+=30)line(0,y,W,y,'#b1d4e507',1);
 if(!state)return;
 const unit=Math.min((W-24)/310,(H-90)/112),originX=W*.31,originY=H-27;
 ctx.save();ctx.translate(originX,originY);ctx.scale(unit,unit);
 line(-150,3,310,3,'#769fb866',2);for(let x=-120;x<300;x+=30)line(x,3,x,8,'#769fb84d',1);
 for(const a of state.actors){ctx.fillStyle='#07132166';ctx.beginPath();ctx.ellipse(a.x,4,28,6,0,0,Math.PI*2);ctx.fill();}robot(state.actors[0],0);robot(state.actors[1],1);
 const spark=state.notices.slice().reverse().find(n=>n.kind===meta.trace_kinds.event&&n.aux===meta.event_kinds.spark),age=spark?state.tick-spark.tick:999;
 if(spark&&age<11){const r=Math.max(4,spark.value)*Math.max(4,Math.min(48,Number(meta.character.properties.spark_size)||18))/18+(reduced?0:age),x=state.actors[1].x-18,y=-51;for(let i=0;i<8;i++){const a=i*Math.PI/4;line(x+Math.cos(a)*r*.35,y+Math.sin(a)*r*.35,x+Math.cos(a)*r,y+Math.sin(a)*r,'#ffe5ae',2);}}
 if($('boxes').checked)for(const a of state.actors)for(const[kind,boxes]of [['hurt',a.hurtboxes],['hit',a.hitboxes],['push',a.pushboxes]])for(const b of boxes){ctx.fillStyle=kind==='hit'?'#ff67732a':kind==='push'?'#ffd27d08':'#75d7ff16';ctx.strokeStyle=kind==='hit'?'#ff929e':kind==='push'?'#ffd27d':'#80d5ff';ctx.lineWidth=1;ctx.setLineDash(kind==='push'?[3,3]:[]);ctx.fillRect(a.x+b.x,a.y+b.y,b.w,b.h);ctx.strokeRect(a.x+b.x,a.y+b.y,b.w,b.h);}ctx.setLineDash([]);
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
function playEvents(){for(const n of state.notices){if(n.seq<=lastHeard)continue;lastHeard=n.seq;if(n.kind!==meta.trace_kinds.event||!audio||!$('sound').checked)continue;const o=audio.createOscillator(),g=audio.createGain(),t=audio.currentTime;o.type='triangle';o.frequency.setValueAtTime(n.aux===meta.event_kinds.spark?680:n.aux===meta.event_kinds.charge?310:470,t);o.frequency.exponentialRampToValueAtTime(140,t+.055);g.gain.setValueAtTime(.035,t);g.gain.exponentialRampToValueAtTime(.001,t+.07);o.connect(g);g.connect(audio.destination);o.start(t);o.stop(t+.075);}}
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
for(const b of document.querySelectorAll('[data-mode]'))b.addEventListener('click',()=>safe(()=>changeMode(b.dataset.mode)));
for(const b of document.querySelectorAll('[data-command]'))b.addEventListener('click',()=>safe(()=>attack(Number(b.dataset.command))));
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
document.addEventListener('keydown',e=>{if(!lab||e.repeat||document.querySelector('dialog[open]')||e.target.closest('input,select,textarea,summary'))return;const cmd=/^(?:Digit|Numpad)([1-4])$/.exec(e.code);if(cmd){e.preventDefault();safe(()=>attack(Number(cmd[1])));}else if(e.code==='KeyP'){e.preventDefault();pause();}else if(e.code==='KeyR'){e.preventDefault();safe(retry);}else if(e.code==='Period'){e.preventDefault();safe(()=>{pause(true);tick();sync();});}});
window.addEventListener('blur',()=>{if(lab)pause(true);});document.addEventListener('visibilitychange',()=>{if(document.hidden&&lab)pause(true);});
new ResizeObserver(()=>{draw();drawTimeline();}).observe(canvas);
Object.defineProperty(window,'framesmith',{value:Object.freeze({snapshot:()=>structuredClone(state),metadata:()=>structuredClone(meta),paused:()=>paused,mode:()=>mode})});
try{
 const get=async(url,binary)=>{const response=await fetch(url,{cache:'no-store',signal:AbortSignal.timeout(20000)});if(!response.ok)throw new Error(`${url}: HTTP ${response.status}`);return binary?new Uint8Array(await response.arrayBuffer()):response.json();};
 const loaded=await Promise.all([init(),get('./packs/lab.fspk',true),get('./build-info.json')]);basePack=loaded[1];build=loaded[2];lab=new Lab(basePack);refreshMetadata();renderExperiment();
 for(const b of document.querySelectorAll('#moves button,#run,#retry,#pause,#experiment'))b.disabled=false;
 $('app').setAttribute('aria-busy','false');setText('status',`Build ${build.commit.slice(0,10)}${build.dirty?' + local edits':''} · editable source → real binary → real behavior`);
 if(!build.dirty&&/^[0-9a-f]{40}$/.test(build.commit))$('source').href=`https://github.com/RobDavenport/framesmith/tree/${build.commit}/demo-wasm`;
 sync();requestAnimationFrame(loop);
}catch(e){setText('error',`The lab could not load: ${e.message||e}. Reload to retry.`);$('error').hidden=false;setText('mission-text','The real runtime is unavailable. No substitute simulation is running.');$('app').setAttribute('aria-busy','false');}
