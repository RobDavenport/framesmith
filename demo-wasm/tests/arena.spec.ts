import { test, expect, type Page } from '@playwright/test';
import { readFileSync } from 'node:fs';
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
async function manualRoute(page:Page,commands:number[],touch=false){
 await page.locator('#retry').click();await page.locator('#arena').focus();
 for(let i=0;i<commands.length;i++){
  const command=commands[i];await until(page,s=>s.trial_progress===i&&s.available.find((a:any)=>a.command===command).allowed);
  if(touch)await page.locator(`[data-command="${command}"]`).tap();else await page.keyboard.press(`Digit${command}`);
  await until(page,s=>s.trial_progress>i||s.trial_failed);expect((await snapshot(page)).trial_failed).toBe(false);
 }
 await until(page,s=>s.trial_clear);await expect(page.locator('#trial-call')).toContainText('TRIAL CLEAR');
}

test('cold repository subpath, real binary-only runtime and asset identity',async({page})=>{
 const errors:string[]=[],requests:string[]=[];page.on('pageerror',e=>errors.push(e.message));page.on('request',r=>requests.push(r.url()));await ready(page,false);await fit(page);
 expect(new URL(page.url()).pathname).toBe('/framesmith/');expect(await page.evaluate(()=>(window as any).framesmith.paused())).toBe(false);await expect(page.locator('#workbench')).toBeHidden();
 const paths=requests.map(x=>new URL(x).pathname);expect(paths.filter(x=>x.endsWith('.fspk'))).toHaveLength(1);expect(paths.some(x=>x.endsWith('.wasm'))).toBe(true);expect(paths.filter(x=>x.endsWith('.json')).map(x=>x.split('/').at(-1))).toEqual(['build-info.json']);
 const checked=await page.evaluate(async()=>{const info=await(await fetch('./build-info.json',{cache:'no-store'})).json();const mismatches=[];for(const [name,expected]of Object.entries(info.files)){const response=await fetch('./'+name,{cache:'no-store'});const hash=[...new Uint8Array(await crypto.subtle.digest('SHA-256',await response.arrayBuffer()))].map(x=>x.toString(16).padStart(2,'0')).join('');if(!response.ok||hash!==expected)mismatches.push(name);}return{mismatches,count:Object.keys(info.files).length};});expect(checked.mismatches).toEqual([]);expect(checked.count).toBeGreaterThan(5);expect(errors).toEqual([]);
});

test('broken route → real keyboard authoring edit → uninterrupted link and cancels',async({page})=>{
 const errors:string[]=[];page.on('pageerror',e=>errors.push(e.message));await ready(page);let s=await demo(page);expect(s.max_combo).toBe(1);expect(s.blocks).toBe(1);await expect(page.locator('#reason')).toContainText('recovered first');
 // Real range-key input, not a simulation-state mutation.
 await page.locator('#edit-recovery').press('Home');for(let i=0;i<4;i++)await page.locator('#edit-recovery').press('ArrowRight');await page.locator('#edit-recovery').press('Tab');expect((await metadata(page)).moves.find((m:any)=>m.id==='jab').recovery).toBe(4);
 s=await demo(page);expect([s.max_combo,s.links,s.cancels,s.energy,s.ammo]).toEqual([4,1,2,10,1]);expect(s.blocks).toBe(0);expect(s.events).toBeGreaterThan(0);
 await page.locator('#inspect').click();await expect(page.locator('#frame-rows tr')).toHaveCount(10);await page.locator('#filter').fill('chainable');await expect(page.locator('#frame-rows tr')).toHaveCount(2);await page.selectOption('#sort','startup');await expect(page.locator('#graph')).toContainText('chainable');await page.getByRole('button',{name:'Close data inspector'}).click();
 await page.screenshot({path:test.info().outputPath('desktop-solved.png')});expect(errors).toEqual([]);
});

test('real tag, deny, window, confirmation, resource, geometry and validation boundaries',async({page})=>{
 await ready(page);await page.selectOption('#speed','1');await page.selectOption('#experiment','3');await page.locator('#edit-tagged').uncheck();let s=await demo(page);expect(s.cancels).toBe(0);expect(s.max_combo).toBe(2);
 await page.locator('#edit-tagged').check();await page.locator('#edit-deny').check();s=await demo(page);expect(s.cancels).toBe(0);await expect(page.locator('#reason')).toContainText('deny');await page.locator('#edit-deny').uncheck();
 await page.selectOption('#experiment','1');await page.locator('#edit-window_end').fill('0');await page.locator('#edit-window_end').press('Tab');s=await demo(page);expect(s.cancels).toBe(0);await expect(page.locator('#reason')).toContainText('window');await page.locator('#edit-window_end').fill('255');await page.locator('#edit-window_end').press('Tab');await page.selectOption('#edit-condition','block');s=await demo(page);expect(s.cancels).toBe(0);
 await page.selectOption('#experiment','4');await knob(page,'distance',20);await page.locator('#arena').focus();await page.keyboard.press('Period');expect((await snapshot(page)).distance).toBeGreaterThanOrEqual(36);await expect(page.locator('#reason')).toContainText('Pushboxes');await knob(page,'distance',74);await page.selectOption('#edit-dummy','1');s=await demo(page);expect(s.max_combo).toBe(0);expect(s.blocks).toBeGreaterThan(0);expect(s.cancels).toBe(1);
 for(const kind of ['1','2','3']){await page.selectOption('#geometry',kind);await expect(page.locator('#reason')).toContainText('OVERLAP');await knob(page,'distance',220);await expect(page.locator('#reason')).toContainText('SEPARATE');await knob(page,'distance',74);}
 await knob(page,'distance',220);s=await demo(page);expect(s.hits).toBe(0);await expect(page.locator('#reason')).toContainText('Whiff');
 await page.selectOption('#edit-dummy','3');await knob(page,'distance',74);await page.selectOption('#experiment','1');await page.selectOption('#edit-condition','hit');
 await page.selectOption('#experiment','2');await knob(page,'energy',100);await knob(page,'ammo',0);await page.locator('#arena').focus();await page.keyboard.press('Digit4');await until(page,s=>s.tick>7);await page.keyboard.press('KeyP');s=await snapshot(page);expect([s.energy,s.ammo,s.hits]).toEqual([100,0,0]);
 await page.selectOption('#experiment','7');const before=await snapshot(page);await page.locator('#invalid').click();await expect(page.locator('#edit-message')).toContainText('REJECTED');expect(await snapshot(page)).toEqual(before);
});

test('four actual manual trials; demo, wrong order and missed timing cannot earn credit',async({page})=>{
 await ready(page);await page.locator('[data-mode="trials"]').click();expect((await snapshot(page)).editable).toBe(false);
 const routes=[[1,2],[2,3],[3,4],[1,2,3,4]];
 for(let trial=0;trial<routes.length;trial++){
  await page.selectOption('#trial-select',String(trial));await page.keyboard.press('F1');await page.selectOption('#speed','1');let s=await demo(page);expect(s.trial_clear).toBe(false);expect(s.manual).toBe(false);expect(s.max_combo).toBe(routes[trial].length);
  await page.locator('#close-tools').click();await page.selectOption('#speed','0.25');await manualRoute(page,routes[trial]);s=await snapshot(page);expect(s.max_combo).toBe(routes[trial].length);expect(s.manual).toBe(true);
 }
 await expect(page.locator('#clear-count')).toHaveText('4/4');await page.screenshot({path:test.info().outputPath('trial-clear.png')});
 await page.selectOption('#trial-select','0');await page.locator('#arena').focus();await page.keyboard.press('Digit2');await until(page,s=>s.trial_failed);expect((await snapshot(page)).trial_clear).toBe(false);
 await page.locator('#retry').click();await page.selectOption('#speed','1');await page.locator('#arena').focus();await page.keyboard.press('Digit1');await until(page,s=>s.trial_progress===1&&s.actors[1].phase===0);await page.keyboard.press('Digit2');await until(page,s=>s.trial_failed);expect((await snapshot(page)).trial_clear).toBe(false);
 await page.locator('[data-mode="guided"]').click();expect((await metadata(page)).settings.recovery).toBe(12); // trial presets did not overwrite the design draft
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
 const context=await browser.newContext({viewport:{width:390,height:844},isMobile:true,hasTouch:true,deviceScaleFactor:1,baseURL:info.project.use.baseURL});try{const page=await context.newPage(),errors:string[]=[];page.on('pageerror',e=>errors.push(e.message));page.on('console',m=>{if(m.type()==='error')errors.push(m.text());});await ready(page,false);await fit(page);await page.screenshot({path:info.outputPath('phone-first.png')});
 const left=page.locator('[data-motion=left]'),box=await left.boundingBox();expect(box).not.toBeNull();
 const cdp=await context.newCDPSession(page);await cdp.send('Input.dispatchTouchEvent',{type:'touchStart',touchPoints:[{x:box!.x+20,y:box!.y+20}]});await until(page,s=>s.actors[0].x<=-18);await cdp.send('Input.dispatchTouchEvent',{type:'touchCancel',touchPoints:[]});await page.locator('#pause').tap();const stopped=await snapshot(page);await page.locator('#pause').tap();await until(page,s=>s.tick>stopped.tick+6);await page.locator('#pause').tap();expect((await snapshot(page)).actors[0].x).toBe(stopped.actors[0].x);await cdp.detach();
 await page.locator('[data-mode="trials"]').tap();await fit(page);await page.selectOption('#speed','0.25');await manualRoute(page,[1,2],true);await page.screenshot({path:info.outputPath('phone-clear.png')});await page.locator('#retry').tap();expect((await snapshot(page)).trial_clear).toBe(false);expect((await snapshot(page)).tick).toBeLessThan(30);expect(errors).toEqual([]);}finally{await context.close();}
});


test('direct movement, release, jump crossing, mirrored contacts and one-touch retry',async({page})=>{
 await ready(page,false);await page.locator('#arena').focus();
 const start=await snapshot(page);await page.keyboard.down('KeyA');await until(page,s=>s.actors[0].x<=-24);await page.keyboard.up('KeyA');await page.keyboard.press('KeyP');
 const back=await snapshot(page);expect(back.actors[1].x).toBe(start.actors[1].x);expect(back.actors[0].x).toBeLessThan(0);
 await page.keyboard.press('KeyP');await until(page,s=>s.tick>back.tick+8);await page.keyboard.press('KeyP');expect((await snapshot(page)).actors[0].x).toBe(back.actors[0].x);
 await page.keyboard.press('KeyR');await page.keyboard.down('KeyD');await until(page,s=>s.actors[0].x>=60);await page.keyboard.down('KeyW');await page.keyboard.up('KeyW');await until(page,s=>s.actors[0].x>s.actors[1].x&&s.actors[0].y<0);await page.keyboard.up('KeyD');await until(page,s=>s.actors[0].y===0);await page.keyboard.press('KeyP');
 expect((await snapshot(page)).actors[0].facing).toBe(-1);await page.keyboard.press('KeyJ');await until(page,s=>s.hits===1);await page.keyboard.press('KeyP');await page.screenshot({path:test.info().outputPath('mirrored-contact.png')});
 await page.keyboard.press('KeyR');expect((await snapshot(page)).distance).toBe(130);
 await page.keyboard.down('KeyD');await until(page,s=>s.actors[0].x>6);await page.evaluate(()=>window.dispatchEvent(new Event('blur')));const blurred=await snapshot(page);expect(await page.evaluate(()=>(window as any).framesmith.paused())).toBe(true);await page.keyboard.up('KeyD');await page.keyboard.press('KeyP');await until(page,s=>s.tick>blurred.tick+8);await page.keyboard.press('KeyP');expect((await snapshot(page)).actors[0].x).toBe(blurred.actors[0].x);await page.keyboard.press('KeyR');await page.keyboard.press('F2');await expect(page.locator('#meter-panel')).toBeVisible();await page.keyboard.press('F3');expect(await page.locator('#boxes').isChecked()).toBe(true);
 await page.locator('[data-mode=trials]').click();await page.keyboard.press('KeyK');await until(page,s=>s.trial_failed);expect(await page.evaluate(()=>(window as any).framesmith.paused())).toBe(false);await until(page,s=>!s.trial_failed&&s.hits===0);await page.screenshot({path:test.info().outputPath('play-first.png')});
});
