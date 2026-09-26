import { test, expect, type Page } from '@playwright/test';
import { readFileSync, writeFileSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { join } from 'node:path';
const snapshot=(page:Page)=>page.evaluate(()=>(window as any).framesmith.snapshot());
const metadata=(page:Page)=>page.evaluate(()=>(window as any).framesmith.metadata());
async function ready(page:Page,tools=true){await page.goto('./');await expect(page.locator('#run')).toBeEnabled();await expect(page.locator('#error')).toBeHidden();if(tools){await page.locator('[data-mode=guided]').click();await page.locator('#reset-edits').click();}}
async function until(page:Page, fn:(s:any)=>boolean){await expect.poll(async()=>fn(await snapshot(page)),{intervals:[16],timeout:12000}).toBe(true);}
async function done(page:Page){await page.waitForFunction(()=>{const f=(window as any).framesmith,s=f.snapshot();return f.paused()&&s.tick>0&&!s.auto;});}
async function demo(page:Page){await page.locator('#run').click();await done(page);return snapshot(page);}
async function knob(page:Page,key:string,value:number){await page.locator(`#edit-${key}`).evaluate((el:any,v)=>{el.value=String(v);el.dispatchEvent(new Event('input',{bubbles:true}));el.dispatchEvent(new Event('change',{bubbles:true}));},value);}
async function fit(page:Page){expect(await page.evaluate(()=>{const d=document.documentElement;return{overflow:d.scrollWidth>innerWidth||d.scrollHeight>innerHeight,clipped:[...document.querySelectorAll('#arena,#retry,#moves button,[data-motion],#speed')].some(e=>{const r=e.getBoundingClientRect();return r.bottom>innerHeight||r.top<0||r.right>innerWidth||r.left<0;})};})).toEqual({overflow:false,clipped:false});}
async function pressMove(page:Page,command:number,touch=false){
 const button=({1:1,2:2,3:4,4:4,7:4,8:3,9:1,10:2,11:3,12:3} as Record<number,number>)[command];
 expect(button).toBeTruthy();
 const down=[7,9,10,11].includes(command),forward=command===4;
 const direction=down?'down':forward?((await snapshot(page)).actors[0].facing>0?'right':'left'):null;
 if(touch&&direction){
  const a=await page.locator(`[data-motion=${direction}]`).boundingBox(),b=await page.locator(`[data-command="${button}"]`).boundingBox();
  expect(a&&b).toBeTruthy();const cdp=await page.context().newCDPSession(page);
  try{const modifier={id:0,x:a!.x+a!.width/2,y:a!.y+a!.height/2};
   await cdp.send('Input.dispatchTouchEvent',{type:'touchStart',touchPoints:[modifier]});
   await cdp.send('Input.dispatchTouchEvent',{type:'touchStart',touchPoints:[modifier,{id:1,x:b!.x+b!.width/2,y:b!.y+b!.height/2}]});
   await cdp.send('Input.dispatchTouchEvent',{type:'touchEnd',touchPoints:[]});
  }finally{await cdp.detach();}
 }else if(touch)await page.locator(`[data-command="${button}"]`).tap();
 else{const key=direction==='down'?'KeyS':direction==='right'?'KeyD':direction==='left'?'KeyA':null;
  if(key)await page.keyboard.down(key);await page.keyboard.press(['','KeyJ','KeyK','KeyL','KeyI'][button]);if(key)await page.keyboard.up(key);
 }
}
async function manualRoute(page:Page,commands:number[],touch=false){
 await page.locator('#retry').click();await page.locator('#arena').focus();
 for(let i=0;i<commands.length;i++){
  const command=commands[i],needsLink=(await metadata(page)).trials[(await snapshot(page)).trial].kinds[i]===3;
  // Exercise the real five-tick buffer, not a late UI round-trip into a 2f link.
  await until(page,s=>{const p=s.actors[0];return s.trial_progress===i&&(needsLink?(p.phase===0||p.phase===3&&p.frame>=p.startup+p.active+p.recovery-3):s.available.find((a:any)=>a.command===command).allowed);});
  await pressMove(page,command,touch);
  await until(page,s=>s.trial_progress>i||s.trial_failed);expect((await snapshot(page)).trial_failed).toBe(false);
 }
 await until(page,s=>s.trial_clear);await expect(page.locator('#trial-call')).toContainText('TRIAL CLEAR');
}

test('practice, lab and trials preserve sourced timing instead of a hidden fast preset',async({page})=>{
 await ready(page,false);
 for(const mode of ['sandbox','guided','trials']){
  await page.locator(`[data-mode=${mode}]`).click();
  const m=await metadata(page),jab=m.moves.find((a:any)=>a.command===1),mp=m.moves.find((a:any)=>a.command===2);
  expect([m.settings.recovery,jab.startup,jab.active,jab.recovery]).toEqual([7,3,3,7]);
  expect([mp.startup,mp.active,mp.recovery,mp.on_hit,mp.on_block]).toEqual([5,4,11,7,-1]);
  expect(mp.resolved.properties.whiff_recovery).toBe(2);
  expect(await page.locator('#speed').inputValue()).toBe('1');
 }
 await page.locator('[data-mode=guided]').click();await page.locator('#inspect').click();
 await expect(page.locator('#data-dialog')).toContainText('SF6 Ryu-inspired timing subset');
 await expect(page.locator('#data-dialog')).toContainText('first active minus one');
 await expect(page.locator('a[href="https://www.streetfighter.com/6/en-us/character/ryu/frame"]')).toBeVisible();
});

test('all required moves follow native hit windows; target and three-hit super are not cosmetic',async({page})=>{
 test.setTimeout(120_000);
 await ready(page);await page.selectOption('#experiment','2');await page.selectOption('#speed','0.25');
 const art=await page.evaluate(async()=> (await import('./assets/blocks/clips.js')).default);
 const expected=['','jab','follow','special','finisher','multi','charged','reload','heavy','low_light','low_medium','low_heavy','target'];
 expect(Object.keys(art.clips)).toHaveLength(21);
 expect(art.clips.target.entry_from).toEqual({clip:"follow",frame:12});
 for(let command=1;command<=12;command++){
  await page.selectOption('#experiment','2');await knob(page,'energy',100);
  if(command===5||command===6){await page.selectOption('#experiment','6');await page.selectOption('#variant',command===5?'1':'2');}
  const move=(await metadata(page)).moves.find((m:any)=>m.command===command);
  await page.evaluate(({command,windows})=>{
   const w=window as any,bag={running:true,rows:[] as any[],images:{} as Record<string,string>};w.artProbe=bag;let last=-1;
   const capture=()=>{if(!bag.running)return;const s=w.framesmith.snapshot(),a=s.actors[0],p=w.framesmith.presentation();
    if(s.tick!==last&&a.command===command){last=s.tick;bag.rows.push({native:a.frame,clip:p[0].clip,art:p[0].frame,freeze:s.freeze,reaction:p[1].clip,hitboxes:a.hitboxes.length});
     windows.forEach((h:any,i:number)=>{if(a.frame>=h.frames[0]&&a.frame<=h.frames[1]&&!bag.images[i])bag.images[i]=(document.querySelector('#arena') as HTMLCanvasElement).toDataURL();});}
    requestAnimationFrame(capture);};requestAnimationFrame(capture);
  },{command,windows:move.resolved.hitboxes});
  await page.locator('#arena').focus();
  if(command===12){await pressMove(page,2);await until(page,s=>s.actors[0].command===2&&s.available.find((a:any)=>a.command===12).allowed);}
  if(command===5||command===6)await page.locator('#run').click();else await pressMove(page,command);
  await until(page,s=>s.actors[0].command===command);await until(page,s=>s.actors[0].command===0&&s.tick>5);if(!await page.evaluate(()=>(window as any).framesmith.paused()))await page.keyboard.press('KeyP');
  const probe=await page.evaluate(()=>{const b=(window as any).artProbe;b.running=false;return{rows:b.rows,images:b.images};});
  expect(probe.rows.length).toBeGreaterThan(5);
  expect([...new Set(probe.rows.map((r:any)=>r.clip))]).toEqual([expected[command]]);
  for(const r of probe.rows)expect(r.art).toBeGreaterThanOrEqual(0);
  for(const [i,h] of move.resolved.hitboxes.entries()){
   const rows=probe.rows.filter((r:any)=>r.native>=h.frames[0]&&r.native<=h.frames[1]);expect(rows.length).toBeGreaterThan(0);
   for(const r of rows){expect(r.art).toBeGreaterThanOrEqual(art.clips[expected[command]].contacts[i][0]);expect(r.art).toBeLessThanOrEqual(art.clips[expected[command]].contacts[i][1]);}
   writeFileSync(test.info().outputPath(`contact-${command}-${i}.png`),Buffer.from(probe.images[i].split(',')[1],'base64'));
  }
  for(let i=1;i<probe.rows.length;i++){const a=probe.rows[i-1],b=probe.rows[i];if(a.native===b.native&&b.freeze)expect(b.art).toBe(a.art);}
  if(command===5){const gap=probe.rows.filter((r:any)=>r.native>move.resolved.hitboxes[0].frames[1]&&r.native<move.resolved.hitboxes[1].frames[0]);expect(gap.length).toBeGreaterThan(0);expect(new Set(gap.map((r:any)=>r.art)).size).toBeGreaterThan(1);expect((await snapshot(page)).hits).toBe(2);}
  if(command!==7)expect(probe.rows.some((r:any)=>r.reaction==='hitstun')).toBe(true);
  if(command===4)expect((await snapshot(page)).hits).toBe(3);
  if(command===7)expect((await snapshot(page)).hits).toBe(0);
 }
 await page.selectOption('#experiment','4');await page.selectOption('#edit-dummy','1');await page.locator('#arena').focus();await page.keyboard.press('KeyJ');await until(page,s=>s.actors[1].phase===5);await page.keyboard.press('KeyP');
 expect(await page.evaluate(()=>(window as any).framesmith.presentation()[1].clip)).toBe('blockstun');
});

test('cold repository subpath, self-contained runtime, asset identity and offline authoring',async({page,baseURL})=>{
 const origin=new URL(baseURL!).origin;
 await page.route('**/*',route=>new URL(route.request().url()).origin===origin?route.continue():route.abort());
 const errors:string[]=[],requests:string[]=[];page.on('pageerror',e=>errors.push(e.message));page.on('console',m=>{if(m.type()==='error')errors.push(m.text());});page.on('request',r=>requests.push(r.url()));await ready(page,false);await fit(page);
 expect(new URL(page.url()).pathname).toBe('/framesmith/');expect(await page.evaluate(()=>(window as any).framesmith.paused())).toBe(false);await expect(page.locator('#workbench')).toBeHidden();
 const paths=requests.map(x=>new URL(x).pathname);expect(paths.filter(x=>x.endsWith('.fspk'))).toHaveLength(1);expect(paths.some(x=>x.endsWith('.wasm'))).toBe(true);expect(paths.filter(x=>x.endsWith('.json')).map(x=>x.split('/').at(-1))).toEqual(['build-info.json']);
 const checked=await page.evaluate(async()=>{const info=await(await fetch('./build-info.json',{cache:'no-store'})).json();const mismatches=[];for(const [name,expected]of Object.entries(info.files)){const response=await fetch('./'+name,{cache:'no-store'});const hash=[...new Uint8Array(await crypto.subtle.digest('SHA-256',await response.arrayBuffer()))].map(x=>x.toString(16).padStart(2,'0')).join('');if(!response.ok||hash!==expected)mismatches.push(name);}return{mismatches,count:Object.keys(info.files).length};});expect(checked.mismatches).toEqual([]);expect(checked.count).toBeGreaterThan(5);
 await page.context().setOffline(true);await page.locator('#arena').focus();await page.keyboard.press('KeyJ');await until(page,s=>s.hits===1);
 await page.locator('[data-mode=guided]').click();await knob(page,'recovery',4);const offline=await demo(page);expect([offline.max_combo,offline.links,offline.cancels]).toEqual([6,1,2]);
 expect([...new Set(requests.map(url=>new URL(url).origin))]).toEqual([origin]);expect(errors).toEqual([]);
});

test('broken route → real keyboard authoring edit → uninterrupted link and cancels',async({page})=>{
 const errors:string[]=[];page.on('pageerror',e=>errors.push(e.message));await ready(page);let s=await demo(page);expect(s.max_combo).toBe(1);expect(s.blocks).toBe(1);await expect(page.locator('#reason')).toContainText('recovered first');
 // Real range-key input, not a simulation-state mutation.
 await page.locator('#edit-recovery').press('Home');for(let i=0;i<4;i++)await page.locator('#edit-recovery').press('ArrowRight');await page.locator('#edit-recovery').press('Tab');expect((await metadata(page)).moves.find((m:any)=>m.id==='jab').recovery).toBe(4);
 s=await demo(page);expect([s.max_combo,s.links,s.cancels,s.energy,s.ammo]).toEqual([6,1,2,10,1]);expect(s.blocks).toBe(0);expect(s.events).toBeGreaterThan(0);
 await page.locator('#inspect').click();await expect(page.locator('#frame-rows tr')).toHaveCount(16);await page.locator('#filter').fill('chainable');await expect(page.locator('#frame-rows tr')).toHaveCount(7);await page.selectOption('#sort','startup');await expect(page.locator('#graph')).toContainText('chainable');await page.getByRole('button',{name:'Close data inspector'}).click();
 await page.screenshot({path:test.info().outputPath('desktop-solved.png')});expect(errors).toEqual([]);
});

test('real tag, deny, window, confirmation, resource, geometry and validation boundaries',async({page})=>{
 await ready(page);await knob(page,'recovery',4);await page.selectOption('#speed','1');await page.selectOption('#experiment','3');await page.locator('#edit-tagged').uncheck();let s=await demo(page);expect(s.cancels).toBe(0);expect(s.max_combo).toBe(2);
 await page.locator('#edit-tagged').check();await page.locator('#edit-deny').check();s=await demo(page);expect(s.cancels).toBe(0);await expect(page.locator('#reason')).toContainText('deny');await page.locator('#edit-deny').uncheck();
 await page.selectOption('#experiment','1');await page.locator('#edit-window_end').fill('0');await page.locator('#edit-window_end').press('Tab');s=await demo(page);expect(s.cancels).toBe(0);await expect(page.locator('#reason')).toContainText('window');await page.locator('#edit-window_end').fill('255');await page.locator('#edit-window_end').press('Tab');await page.selectOption('#edit-condition','block');s=await demo(page);expect(s.cancels).toBe(0);
 await page.selectOption('#experiment','4');await knob(page,'distance',20);await page.locator('#arena').focus();await page.keyboard.press('Period');expect((await snapshot(page)).distance).toBeGreaterThanOrEqual(36);await expect(page.locator('#reason')).toContainText('Pushboxes');await knob(page,'distance',48);await page.selectOption('#edit-dummy','1');s=await demo(page);expect(s.max_combo).toBe(0);expect(s.blocks).toBeGreaterThan(0);expect(s.cancels).toBe(1);
 for(const kind of ['1','2','3']){await page.selectOption('#geometry',kind);await expect(page.locator('#reason')).toContainText('OVERLAP');await knob(page,'distance',220);await expect(page.locator('#reason')).toContainText('SEPARATE');await knob(page,'distance',48);}
 await knob(page,'distance',220);s=await demo(page);expect(s.hits).toBe(0);await expect(page.locator('#reason')).toContainText('Whiff');
 await page.selectOption('#edit-dummy','3');await knob(page,'distance',48);await page.selectOption('#experiment','1');await page.selectOption('#edit-condition','hit');
 await page.selectOption('#experiment','2');await knob(page,'energy',100);await knob(page,'ammo',0);await page.locator('#arena').focus();await pressMove(page,4);await until(page,s=>s.tick>7);await page.keyboard.press('KeyP');s=await snapshot(page);expect([s.energy,s.ammo,s.hits]).toEqual([100,0,0]);
 await page.selectOption('#experiment','7');const before=await snapshot(page);await page.locator('#invalid').click();await expect(page.locator('#edit-message')).toContainText('REJECTED');expect(await snapshot(page)).toEqual(before);
});

test('seven actual manual structures; demo, wrong order and missed timing cannot earn credit',async({page})=>{
 test.setTimeout(120_000);
 await ready(page);await page.locator('[data-mode="trials"]').click();expect((await snapshot(page)).editable).toBe(false);
 const routes=[[2,2],[2,3],[3,4],[2,2,12,3,4],[9,10,11],[2,12],[10,7]];
 expect((await metadata(page)).trials.map((t:any)=>t.route)).toEqual(routes);
 const expectedHits=(route:number[],m:any)=>route.reduce((n,c)=>n+m.moves.find((a:any)=>a.command===c).resolved.hitboxes.length,0);
 for(let trial=0;trial<routes.length;trial++){
  await page.selectOption('#trial-select',String(trial));await page.keyboard.press('F1');await page.selectOption('#speed','1');let s=await demo(page);expect(s.trial_clear).toBe(false);expect(s.manual).toBe(false);expect(s.max_combo).toBe(expectedHits(routes[trial],await metadata(page)));
  await page.locator('#close-tools').click();await page.selectOption('#speed','0.25');await manualRoute(page,routes[trial]);s=await snapshot(page);expect(s.max_combo).toBe(expectedHits(routes[trial],await metadata(page)));expect(s.manual).toBe(true);
 }
 await expect(page.locator('#clear-count')).toHaveText('7/7');await page.screenshot({path:test.info().outputPath('trial-clear.png')});
 await page.selectOption('#trial-select','0');await page.locator('#arena').focus();await page.keyboard.press('Digit1');await until(page,s=>s.trial_failed);expect((await snapshot(page)).trial_clear).toBe(false);
 await page.locator('#retry').click();await page.selectOption('#speed','1');await page.locator('#arena').focus();await page.keyboard.press('Digit2');await until(page,s=>s.trial_progress===1&&s.actors[1].phase===0);await page.keyboard.press('Digit2');await until(page,s=>s.trial_failed);expect((await snapshot(page)).trial_clear).toBe(false);
 await page.locator('[data-mode="guided"]').click();expect((await metadata(page)).settings.recovery).toBe(7); // trial presets did not overwrite the design draft
});

test('multi-hit, inherited variant, refill, event property, checkpoint, rewind and exact replay',async({page})=>{
 await ready(page);await page.selectOption('#speed','1');await page.selectOption('#experiment','6');let s=await demo(page);expect([s.hits,s.max_combo,s.damage]).toEqual([2,2,40]);
 await page.locator('#tools summary').click();await page.locator('#save').click();const saved=await snapshot(page);for(let n=0;n<3;n++)await page.locator('#step').click();const advanced=await snapshot(page);expect(advanced.tick).toBe(saved.tick+3);await page.locator('#restore').click();expect(await snapshot(page)).toEqual(saved);for(let n=0;n<3;n++)await page.locator('#step').click();expect(await snapshot(page)).toEqual(advanced);await page.locator('#back').click();expect((await snapshot(page)).tick).toBe(advanced.tick-1);await page.locator('#verify').click();await expect(page.locator('#replay')).toContainText('PASS');await page.locator('#tools summary').click();
 await page.selectOption('#variant','2');s=await demo(page);expect(s.damage).toBe((await metadata(page)).moves.find((m:any)=>m.id==='special~charged').damage);
 await page.selectOption('#variant','3');s=await demo(page);expect(s.ammo).toBe(3);expect(s.notices.some((n:any)=>n.kind===10&&n.value===3)).toBe(true);expect(s.hits).toBe(0);
 await page.selectOption('#experiment','5');await knob(page,'spark_size',32);await knob(page,'notify_frame',2);s=await demo(page);const m=await metadata(page);expect(m.character.properties.spark_size).toBe(32);expect(s.notices.some((n:any)=>n.kind===m.trace_kinds.event&&n.aux===0&&n.tick===3)).toBe(true);expect(s.notices.some((n:any)=>n.kind===m.trace_kinds.event&&n.aux===m.event_kinds.spark)).toBe(true);
 await page.locator('#inspect').click();await page.selectOption('#data-kind','source');await page.selectOption('#data-item','characters/relay/states/special~charged.json');await expect(page.locator('#json')).toContainText('"base": "special"');await page.selectOption('#data-item','characters/relay/globals.json');await expect(page.locator('#json')).toContainText('override');expect((await metadata(page)).moves.find((m:any)=>m.id==='idle').name).toBe('Relay Ready');
});

test('edited project ZIP recompiles identically in the CLI; exported pack reloads and bad input preserves state',async({page})=>{
 await ready(page);await knob(page,'recovery',4);await page.selectOption('#experiment','7');
 const packWait=page.waitForEvent('download');await page.locator('#download-pack').click();const pack=await packWait,packPath=test.info().outputPath('relay.fspk');await pack.saveAs(packPath);
 const zipWait=page.waitForEvent('download');await page.locator('#download-project').click();const archive=await zipWait,zipPath=test.info().outputPath('project.zip');await archive.saveAs(zipPath);
 expect(readFileSync(packPath).subarray(0,4).toString()).toBe('FSPK');
 const out=test.info().outputPath('unzipped');execFileSync('python',['-c','import zipfile,sys; z=zipfile.ZipFile(sys.argv[1]); assert z.testzip() is None; assert all(not n.startswith("/") and ".." not in n.split("/") for n in z.namelist()); z.extractall(sys.argv[2])',zipPath,out]);
 const cli=fileURLToPath(new URL('../../src-tauri/target/debug/'+(process.platform==='win32'?'framesmith-cli.exe':'framesmith-cli'),import.meta.url));const rebuilt=test.info().outputPath('cli.fspk');execFileSync(cli,['export','--project',join(out,'framesmith-lab'),'--character','relay','--adapter','fspk','--out',rebuilt]);expect(readFileSync(rebuilt).equals(readFileSync(packPath))).toBe(true);
 const expected=await demo(page);await page.locator('#import-pack').setInputFiles(packPath);await expect(page.locator('#edit-message')).toContainText('Imported binary');expect((await snapshot(page)).editable).toBe(false);const imported=await demo(page);expect(imported).toEqual({...expected,editable:false});
 await page.locator('#import-pack').setInputFiles({name:'bad.fspk',mimeType:'application/octet-stream',buffer:Buffer.from('not a pack')});await page.waitForFunction(()=>!(document.querySelector('#import-pack') as HTMLInputElement).value);await expect(page.locator('#edit-message')).toContainText('Invalid FSPK');expect(await snapshot(page)).toEqual(imported);await expect(page.locator('#error')).toBeHidden();
});

test('phone first-run layout and real touch can clear a trial without console errors',async({browser},info)=>{
 const context=await browser.newContext({viewport:{width:390,height:844},isMobile:true,hasTouch:true,deviceScaleFactor:1,baseURL:info.project.use.baseURL});try{const page=await context.newPage(),errors:string[]=[];page.on('pageerror',e=>errors.push(e.message));page.on('console',m=>{if(m.type()==='error')errors.push(m.text());});await ready(page,false);await fit(page);await expect(page.locator('#moves button')).toHaveCount(4);await page.screenshot({path:info.outputPath('phone-first.png')});
 const left=page.locator('[data-motion=left]'),box=await left.boundingBox();expect(box).not.toBeNull();
 const cdp=await context.newCDPSession(page);await cdp.send('Input.dispatchTouchEvent',{type:'touchStart',touchPoints:[{x:box!.x+20,y:box!.y+20}]});await until(page,s=>s.actors[0].x<=-18);await cdp.send('Input.dispatchTouchEvent',{type:'touchCancel',touchPoints:[]});await page.locator('#pause').tap();const stopped=await snapshot(page);await page.locator('#pause').tap();await until(page,s=>s.tick>stopped.tick+6);await page.locator('#pause').tap();expect((await snapshot(page)).actors[0].x).toBe(stopped.actors[0].x);await cdp.detach();
 await page.locator('[data-mode="trials"]').tap();await fit(page);await page.selectOption('#speed','0.25');await manualRoute(page,[2,2],true);await page.selectOption('#trial-select','4');await manualRoute(page,[9,10,11],true);await page.selectOption('#trial-select','6');await manualRoute(page,[10,7],true);await page.selectOption('#trial-select','2');await manualRoute(page,[3,4],true);await page.screenshot({path:info.outputPath('phone-clear.png')});await page.locator('#retry').tap();expect((await snapshot(page)).trial_clear).toBe(false);expect((await snapshot(page)).tick).toBeLessThan(30);expect(errors).toEqual([]);}finally{await context.close();}
});


test('late target entry is continuous and super direction follows the fighter',async({page})=>{
 await ready(page);await page.selectOption('#speed','0.25');
 await page.selectOption('#experiment','2');await knob(page,'energy',100);await page.locator('#arena').focus();
 await pressMove(page,2);await until(page,s=>s.actors[0].frame>=8&&s.actors[0].command===2);await page.keyboard.press('KeyP');
 while((await snapshot(page)).actors[0].frame<9)await page.keyboard.press('Period');
 expect((await snapshot(page)).actors[0].frame).toBe(9);
 const before=await page.evaluate(()=>(window as any).framesmith.presentation()[0]);
 expect(before.clip).toBe('follow');expect(before.frame).toBeGreaterThanOrEqual(12);expect(before.frame).toBeLessThanOrEqual(17);
 await pressMove(page,12);await until(page,s=>s.actors[0].command===12);const entry=await page.evaluate(()=>(window as any).framesmith.presentation()[0]);
 expect(entry.clip).toBe('target');expect(entry.frame).toBe(0);
 await until(page,s=>s.hits===2);await page.locator('#pause').click();
 await page.selectOption('#speed','1');await knob(page,'energy',100);await page.locator('#arena').focus();
 await page.keyboard.down('KeyD');await page.keyboard.press('KeyW');await until(page,s=>s.actors[0].x>s.actors[1].x+24);await page.keyboard.up('KeyD');
 await until(page,s=>s.actors[0].y===0);expect((await snapshot(page)).actors[0].facing).toBe(-1);
 await pressMove(page,4);await until(page,s=>s.hits===3);await page.keyboard.press('KeyP');const end=await snapshot(page);
 expect([end.actors[0].command,end.ammo,end.energy]).toEqual([4,2,50]);
 if(!await page.locator('#workbench').isVisible())await page.locator('[data-mode=guided]').click();if(!await page.locator('#verify').isVisible())await page.locator('#tools summary').click();await page.locator('#verify').click();await expect(page.locator('#replay')).toContainText('PASS');expect(await snapshot(page)).toEqual(end);
});

test('direct movement, release, jump crossing, mirrored contacts and one-touch retry',async({page})=>{
 await ready(page,false);await page.locator('#arena').focus();
 const start=await snapshot(page);await page.keyboard.down('KeyA');await until(page,s=>s.actors[0].x<=-24);await page.keyboard.up('KeyA');await page.keyboard.press('KeyP');
 const back=await snapshot(page);expect(back.actors[1].x).toBe(start.actors[1].x);expect(back.actors[0].x).toBeLessThan(0);
 await page.keyboard.press('KeyP');await until(page,s=>s.tick>back.tick+8);await page.keyboard.press('KeyP');expect((await snapshot(page)).actors[0].x).toBe(back.actors[0].x);
 await page.keyboard.press('KeyR');await page.keyboard.down('KeyD');await until(page,s=>s.actors[0].x>=60);await page.keyboard.down('KeyW');await page.keyboard.up('KeyW');await until(page,s=>s.actors[0].x>s.actors[1].x&&s.actors[0].y<0);await page.keyboard.up('KeyD');await until(page,s=>s.actors[0].y===0);await page.keyboard.press('KeyP');
 expect((await snapshot(page)).actors[0].facing).toBe(-1);await page.keyboard.press('KeyJ');await until(page,s=>s.hits===1);await page.keyboard.press('KeyP');await page.screenshot({path:test.info().outputPath('mirrored-contact.png')});
 await page.keyboard.press('KeyR');expect((await snapshot(page)).distance).toBe(48);
 await page.keyboard.down('KeyD');await until(page,s=>s.actors[0].x>6);await page.evaluate(()=>window.dispatchEvent(new Event('blur')));const blurred=await snapshot(page);expect(await page.evaluate(()=>(window as any).framesmith.paused())).toBe(true);await page.keyboard.up('KeyD');await page.keyboard.press('KeyP');await until(page,s=>s.tick>blurred.tick+8);await page.keyboard.press('KeyP');expect((await snapshot(page)).actors[0].x).toBe(blurred.actors[0].x);await page.keyboard.press('KeyR');await page.keyboard.press('F2');await expect(page.locator('#meter-panel')).toBeVisible();await page.keyboard.press('F3');expect(await page.locator('#boxes').isChecked()).toBe(true);
 await page.locator('[data-mode=trials]').click();await page.keyboard.press('KeyJ');await until(page,s=>s.trial_failed);expect(await page.evaluate(()=>(window as any).framesmith.paused())).toBe(false);await until(page,s=>!s.trial_failed&&s.hits===0);await page.screenshot({path:test.info().outputPath('play-first.png')});
});
