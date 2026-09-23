import { test, expect, type Page } from '@playwright/test';

const snapshot = (page: Page) => page.evaluate(() => (window as any).framesmith.snapshot());
async function ready(page: Page, practice = true) {
  await page.goto('./');
  await expect(page.locator('#play')).toBeEnabled();
  if (practice) await page.selectOption('#mode', '1');
  await page.locator('#play').click();
}
async function ticks(page: Page, n: number) {
  const start = (await snapshot(page)).tick;
  await page.waitForFunction(([start,n]) => {
    const s=(window as any).framesmith.snapshot();return s.tick>=start+n||s.winner;
  }, [start,n]);
}
async function tap(page: Page, key: string) { await page.keyboard.press(key);await ticks(page,2); }
async function until(page: Page, predicate: (s: any) => boolean) {
  await expect.poll(async () => predicate(await snapshot(page)), { intervals: [16], timeout: 15_000 }).toBe(true);
}
async function fit(page: Page) {
  expect(await page.evaluate(() => {
    const d=document.documentElement;
    const controls=[...document.querySelectorAll('.controls button,.toolbar button,.toolbar select')];
    return {overflow:d.scrollHeight>innerHeight||d.scrollWidth>innerWidth,
      clipped:controls.some(e=>{const r=e.getBoundingClientRect();return r.bottom>innerHeight||r.right>innerWidth||r.left<0||r.top<0;})};
  })).toEqual({overflow:false,clipped:false});
}

test('cold Pages subpath, real binary assets and two-button surface', async ({page}) => {
  const errors:string[]=[],requests:string[]=[];
  page.on('pageerror',e=>errors.push(e.message));page.on('request',r=>requests.push(r.url()));
  await ready(page);
  const s=await snapshot(page);
  expect(s.platforms).toEqual([{left:120,right:680,top:0},{left:190,right:320,top:-95},{left:480,right:610,top:-95},{left:335,right:465,top:-190}]);
  expect(s.actors.map((a:any)=>a.stocks)).toEqual([3,3]);
  await expect(page.locator('.attacks button')).toHaveCount(2);
  await expect(page.getByRole('button',{name:'Normal attack, J'})).toBeVisible();
  await fit(page);
  const path=new URL(page.url()).pathname;
  expect(path).toMatch(/\/framesmith\/$/);
  const urls=requests.map(u=>new URL(u));
  expect(urls.filter(u=>u.pathname.endsWith('.fspk'))).toHaveLength(4);
  expect(urls.some(u=>u.pathname.endsWith('.wasm'))).toBe(true);
  expect(urls.filter(u=>u.pathname.endsWith('.json')).map(u=>u.pathname.split('/').pop())).toEqual(['build-info.json']);
  const info=await page.evaluate(async()=> (await fetch('./build-info.json',{cache:'no-store'})).json());
  expect(info.files).toBeTruthy();
  await ticks(page,80);
  await page.screenshot({path:test.info().outputPath('desktop-platform.png')});
  expect(errors).toEqual([]);
});

test('real movement crosses rival, two jumps land on triangle and drop through', async ({page}) => {
  await ready(page);
  const initial=await snapshot(page);
  await page.keyboard.down('ArrowLeft');await ticks(page,10);await page.keyboard.up('ArrowLeft');
  let s=await snapshot(page);expect(s.actors[0].x).toBeLessThan(initial.actors[0].x-30);expect(s.actors[1].x).toBe(initial.actors[1].x);
  await page.keyboard.down('ArrowRight');await until(page,s=>s.actors[0].x>s.actors[1].x+15);await page.keyboard.up('ArrowRight');
  s=await snapshot(page);expect(s.actors[1].x).toBe(initial.actors[1].x);
  await page.locator('#reset').click();await ticks(page,2);
  await tap(page,'Space');await until(page,s=>s.actors[0].platform===1);
  s=await snapshot(page);expect(s.actors[0].y).toBe(-95);expect(s.actors[0].jumps_remaining).toBe(2);
  await ticks(page,8);
  await page.keyboard.down('ArrowRight');await tap(page,'Space');await until(page,s=>s.actors[0].x>=390);await page.keyboard.up('ArrowRight');
  await until(page,s=>s.actors[0].platform===3);expect((await snapshot(page)).actors[0].y).toBe(-190);
  await ticks(page,8);
  await page.keyboard.down('ArrowRight');await tap(page,'Space');await until(page,s=>s.actors[0].x>=530);await page.keyboard.up('ArrowRight');
  await until(page,s=>s.actors[0].platform===2);await ticks(page,8);
  await page.keyboard.down('ArrowDown');await tap(page,'Space');await page.keyboard.up('ArrowDown');
  await until(page,s=>s.actors[0].platform===0);
  await ticks(page,8);await tap(page,'Space');await ticks(page,6);await tap(page,'Space');
  s=await snapshot(page);expect(s.actors[0].jumps_remaining).toBe(0);const vy=s.actors[0].vy;
  await tap(page,'Space');expect((await snapshot(page)).actors[0].vy).toBeGreaterThan(vy);
});

test('all four fighters can recover offstage with up Special; quick modifiers survive release', async ({page}) => {
  await ready(page);
  for(const fighter of ['relay','bulwark','sable','zip']){
    await page.selectOption('#fighter',fighter);await page.locator('#play').click();
    await page.keyboard.down('ArrowLeft');await until(page,s=>s.actors[0].x<110);await page.keyboard.up('ArrowLeft');
    await page.keyboard.down('ArrowRight');await page.keyboard.down('ArrowUp');await page.keyboard.press('KeyK');await page.keyboard.up('ArrowUp');
    await ticks(page,4);let s=await snapshot(page);
    expect(s.actors[0].recovery_ready).toBe(false);expect(s.actors[0].vy).toBeLessThan(0);
    await until(page,s=>s.actors[0].x>235);await page.keyboard.up('ArrowRight');
    await until(page,s=>s.actors[0].platform>=0);
    s=await snapshot(page);expect(s.actors[0].stocks).toBe(3);expect(s.actors[0].recovery_ready).toBe(true);
  }
});

test('Normal and Special cause real hits, percentage launch, spending and replay', async ({page}) => {
  const errors:string[]=[];page.on('pageerror',e=>errors.push(e.message));
  await ready(page);
  await page.keyboard.down('ArrowRight');await until(page,s=>s.actors[0].x>=495);await page.keyboard.up('ArrowRight');await ticks(page,2);
  await tap(page,'KeyJ');await until(page,s=>s.stats.hits[0]>0);
  let s=await snapshot(page);expect(s.actors[1].damage).toBeGreaterThan(0);expect(s.actors[1].vx).toBeGreaterThan(0);
  await page.locator('#reset').click();await tap(page,'KeyK');await until(page,s=>s.projectiles.length>0);
  await page.locator('#pause').click();await page.locator('#diagnostics summary').click();
  await page.locator('#save').click();const saved=await snapshot(page);
  for(let i=0;i<4;i++)await page.locator('#step').click();
  const advanced=await snapshot(page);expect(advanced.tick).toBe(saved.tick+4);
  await page.locator('#restore').click();expect(await snapshot(page)).toEqual(saved);
  await page.locator('#verify').click();await expect(page.locator('#replay')).toContainText('empty replay');
  for(let i=0;i<4;i++)await page.locator('#step').click();
  expect(await snapshot(page)).toEqual(advanced);
  await page.locator('#verify').click();await expect(page.locator('#replay')).toContainText('PASS');
  await page.locator('#diagnostics summary').click();await page.locator('#pause').click();
  await until(page,s=>s.stats.hits[0]>0);s=await snapshot(page);expect(s.actors[1].damage).toBeGreaterThan(0);
  await page.locator('#reset').click();await page.keyboard.down('ArrowDown');await tap(page,'KeyK');await page.keyboard.up('ArrowDown');
  await until(page,s=>s.stats.spent[0]===50);
  await until(page,s=>s.actors[0].id==='idle');await page.getByRole('button',{name:'Normal attack, J'}).focus();await tap(page,'Space');
  expect((await snapshot(page)).actors[0].id).toBe('light');
  expect(errors).toEqual([]);
});

test('ring-outs, protected respawn, complete stock match and rematch through controls', async ({page}) => {
  await ready(page);
  await page.keyboard.down('ArrowLeft');await until(page,s=>s.actors[0].stocks===2);await page.keyboard.up('ArrowLeft');
  let s=await snapshot(page);expect(s.actors[0].damage).toBe(0);expect(s.actors[0].respawn).toBeGreaterThan(0);
  await expect(page.locator('#p1-stocks')).toHaveAttribute('aria-label','2 stocks remaining');
  await until(page,s=>s.actors[0].respawn===0);expect((await snapshot(page)).actors[0].invulnerable).toBeGreaterThan(0);
  await page.keyboard.down('ArrowLeft');await until(page,s=>s.winner===2);await page.keyboard.up('ArrowLeft');
  await expect(page.locator('#overlay')).toBeVisible();await expect(page.locator('#overlay-title')).toContainText('Bulwark wins.');
  s=await snapshot(page);expect(s.actors[0].stocks).toBe(0);expect(s.stats.kos[1]).toBe(3);
  await page.locator('#play').click();s=await snapshot(page);expect(s.actors.map((a:any)=>a.stocks)).toEqual([3,3]);expect(s.winner).toBe(0);
});

test('CPU navigates, attacks, and reproduces exact full-state replay', async ({page}) => {
  await ready(page,false);
  await until(page,s=>s.stats.hits[1]>0);
  const s=await snapshot(page);expect(s.actors[1].x).not.toBe(540);expect(s.actors[0].damage+s.stats.kos[1]).toBeGreaterThan(0);
  await page.locator('#pause').click();await page.locator('#diagnostics summary').click();await page.locator('#verify').click();
  await expect(page.locator('#replay')).toContainText('PASS');
});

test('phone real multi-touch, pointer release, two attack buttons and complete layout', async ({browser}, info) => {
  const context=await browser.newContext({viewport:{width:390,height:844},deviceScaleFactor:1,isMobile:true,hasTouch:true,baseURL:info.project.use.baseURL});
  const page=await context.newPage();const errors:string[]=[];page.on('pageerror',e=>errors.push(e.message));await ready(page);
  await fit(page);await expect(page.locator('.attacks button')).toHaveCount(2);
  const cdp=await context.newCDPSession(page);
  const point=async(input:number,id:number)=>{const b=await page.locator(`[data-input="${input}"]`).boundingBox();return {x:b!.x+b!.width/2,y:b!.y+b!.height/2,id};};
  const left=await point(1,1),jump=await point(16,2);
  await cdp.send('Input.dispatchTouchEvent',{type:'touchStart',touchPoints:[left]});await ticks(page,4);
  await cdp.send('Input.dispatchTouchEvent',{type:'touchStart',touchPoints:[left,jump]});await ticks(page,5);
  await expect(page.locator('[data-input="1"]')).toHaveAttribute('aria-pressed','true');
  await expect(page.locator('[data-input="16"]')).toHaveAttribute('aria-pressed','true');
  let s=await snapshot(page);expect(s.actors[0].x).toBeLessThan(245);expect(s.actors[0].y).toBeLessThan(0);
  await cdp.send('Input.dispatchTouchEvent',{type:'touchEnd',touchPoints:[]});await ticks(page,3);
  const x=(await snapshot(page)).actors[0].x;await ticks(page,5);expect((await snapshot(page)).actors[0].x).toBe(x);
  expect(await page.locator('[aria-pressed="true"]').count()).toBe(0);
  await page.locator('#reset').click();const up=await point(4,1),special=await point(64,2);
  await cdp.send('Input.dispatchTouchEvent',{type:'touchStart',touchPoints:[up]});
  await cdp.send('Input.dispatchTouchEvent',{type:'touchStart',touchPoints:[up,special]});
  await cdp.send('Input.dispatchTouchEvent',{type:'touchEnd',touchPoints:[]});await ticks(page,4);
  s=await snapshot(page);expect(s.actors[0].vy).toBeLessThan(0);expect(s.actors[0].recovery_ready).toBe(false);
  await fit(page);await page.screenshot({path:info.outputPath('phone-platform-active.png')});await page.locator('#pause').click();
  expect(errors).toEqual([]);await context.close();
});
