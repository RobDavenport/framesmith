import { test, expect, type Page } from '@playwright/test';
import { createHash } from 'node:crypto';

const snapshot = (page: Page) => page.evaluate(() => (window as any).framesmith.snapshot());
async function load(page: Page, mode = '0', fighter = 'relay', rival = 'bulwark') {
  await page.clock.install();
  await page.goto('./');
  await expect(page.locator('#play')).toBeEnabled();
  await expect(page.locator('#error')).toBeHidden();
  await page.locator('#fighter').selectOption(fighter);
  await page.locator('#rival').selectOption(rival);
  if (mode !== '0') await page.locator('#mode').selectOption(mode);
  await page.locator('#play').click();
  await expect(page.locator('#overlay')).toBeHidden();
}
async function fits(page: Page) {
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth && document.documentElement.scrollHeight <= innerHeight)).toBe(true);
  for (const selector of ['#arena', '.controls', '#pause', '#reset', '#mode', 'footer']) {
    const b = await page.locator(selector).boundingBox();
    expect(b, selector).toBeTruthy();
    const size = page.viewportSize()!;
    expect(b!.x, selector).toBeGreaterThanOrEqual(0);
    expect(b!.y + b!.height, selector).toBeLessThanOrEqual(size.height + 1);
    expect(b!.x + b!.width, selector).toBeLessThanOrEqual(size.width + 1);
  }
}

test('cold subpath build loads real WASM/packs and matches its asset manifest', async ({ page, request }, testInfo) => {
  const errors: string[] = [], urls: string[] = [];
  page.on('pageerror', error => errors.push(error.message));
  page.on('console', message => { if (message.type() === 'error') errors.push(message.text()); });
  page.on('request', r => urls.push(r.url()));
  await load(page);
  await page.clock.runFor(500);
  expect((await snapshot(page)).tick).toBeGreaterThan(0);
  expect(urls.some(u => u.endsWith('.wasm'))).toBe(true);
  expect(new Set(urls.filter(u => u.endsWith('.fspk'))).size).toBe(4);
  expect(urls.filter(u => u.endsWith('.json')).every(u => u.endsWith('/build-info.json'))).toBe(true);
  expect(urls.every(u => new URL(u).origin === new URL(page.url()).origin)).toBe(true);
  const info = await (await request.get('./build-info.json')).json();
  expect(info.commit).toMatch(/^[0-9a-f]{40}$/);
  for (const [name, digest] of Object.entries(info.files)) {
    const response = await request.get(`./${name}`);
    expect(response.ok(), name).toBe(true);
    expect(createHash('sha256').update(await response.body()).digest('hex'), name).toBe(digest);
    if (name.endsWith('.wasm')) expect(response.headers()['content-type']).toContain('application/wasm');
    if (name.endsWith('.fspk')) expect((await response.body()).subarray(0, 4).toString()).toBe('FSPK');
  }
  await fits(page);
  await page.screenshot({ path: testInfo.outputPath('desktop.png') });
  expect(errors).toEqual([]);
});

test('keyboard combo reaches KO, spends meter, cancels and replays; checkpoint and reset are real', async ({ page }) => {
  await load(page, '1');
  await page.keyboard.down('d'); await page.clock.runFor(1850); await page.keyboard.up('d');
  expect((await snapshot(page)).actors[0].x).toBeGreaterThan(310);
  await page.keyboard.press('j'); await page.clock.runFor(150);
  let s = await snapshot(page);
  expect(s.actors[1].hp).toBeLessThan(s.actors[1].max_hp);
  await page.locator('#diagnostics summary').click();
  await page.locator('#save').click();
  const saved = await snapshot(page);
  await page.locator('#step').click();
  expect((await snapshot(page)).tick).toBe(saved.tick + 1);
  await page.locator('#restore').click();
  expect(await snapshot(page)).toEqual(saved);
  await page.locator('#diagnostics summary').click();
  await page.locator('#play').click();
  for (let i = 0; i < 700; i++) {
    s = await snapshot(page);
    if (s.winner) break;
    const a = s.actors[0];
    if (s.actors[1].x - a.x > 65) await page.keyboard.down('d'); else await page.keyboard.up('d');
    if (a.id === 'idle' || a.id === 'guard') await page.keyboard.press('j');
    else if (a.id === 'light' && a.confirmed) await page.keyboard.press('k');
    else if (a.id === 'medium' && a.confirmed) await page.keyboard.press('l');
    else if (a.id === 'heavy' && a.confirmed) {
      if (a.meter>=50) {await page.keyboard.down('l');await page.keyboard.press('u');await page.keyboard.up('l');}
      else await page.keyboard.press('u');
    }
    await page.clock.runFor(80);
  }
  await page.keyboard.up('d');
  s = await snapshot(page);
  expect(s.winner).toBe(1);
  expect(s.actors[1].hp).toBe(0);
  expect(s.stats.cancels[0]).toBeGreaterThan(0);
  expect(s.stats.spent[0]).toBeGreaterThanOrEqual(50);
  expect(s.stats.signals[0]).toBeGreaterThan(0);
  await expect(page.locator('#overlay-title')).toHaveText('You win.');
  await page.locator('#diagnostics summary').click();
  await page.locator('#verify').click();
  await expect(page.locator('#replay')).toContainText(`PASS · ${s.recorded} frames replayed`);
  await page.locator('#reset').click();
  const reset = await snapshot(page);
  expect(reset.tick).toBe(0);
  expect(reset.actors.every((a: any) => a.hp === a.max_hp)).toBe(true);
  expect(reset.stats.cancels).toEqual([0, 0]);
});

test('guard blocks the live bot; releasing it allows a real opponent KO', async ({ page }) => {
  await load(page, '0', 'relay', 'relay');
  await page.keyboard.down('a'); await page.clock.runFor(8000);
  const guarded = await snapshot(page);
  expect(guarded.stats.blocks[0]).toBeGreaterThan(0);
  expect(guarded.actors[0].hp).toBeGreaterThan(0); // Specials can chip; lows beat standing guard.
  await page.keyboard.up('a');
  for (let i = 0; i < 30 && !(await snapshot(page)).winner; i++) await page.clock.runFor(5000);
  const lost = await snapshot(page);
  expect(lost.winner).toBe(2);
  expect(lost.actors[0].hp).toBe(0);
  await expect(page.locator('#overlay-title')).toHaveText('Relay wins.');
  await page.locator('#diagnostics summary').click(); await page.locator('#verify').click();
  await expect(page.locator('#replay')).toContainText(`PASS · ${lost.recorded} frames replayed`);
});

test('actual WASM rejects malformed packs and invalid numbers without state mutation', async ({ page }) => {
  await load(page);
  const result = await page.evaluate(async () => {
    const url = new URL('./pkg/framesmith_arena.js', location.href).href;
    const { Arena } = await import(/* @vite-ignore */ url);
    const packs = await Promise.all(['relay', 'bulwark'].map(async n => new Uint8Array(await (await fetch(`./packs/${n}.fspk`)).arrayBuffer())));
    const game = new Arena(packs[0], packs[1], 17, 0), before = JSON.stringify(game.view());
    let rejected = 0;
    for (const input of [NaN, Infinity, -1, .5, 256, 2 ** 32]) {
      try { game.step(input); } catch { rejected++; }
      if (JSON.stringify(game.view()) !== before) throw new Error('Invalid input mutated state');
    }
    for (const pack of [new Uint8Array(), packs[0].slice(0, 16), packs[0].slice(0, -1)]) {
      try { const invalid = new Arena(pack, packs[1], 17, 0); invalid.free(); } catch { rejected++; }
    }
    game.free(); return rejected;
  });
  expect(result).toBe(9);
});

test('all four selections exercise their signature through real buttons', async ({ page }, testInfo) => {
  await load(page, '1');
  const witnesses: any[] = [];
  for (const [fighter, style] of [['relay','shoto'],['bulwark','grappler'],['sable','zoner'],['zip','rushdown']]) {
    await page.locator('#fighter').selectOption(fighter);
    await page.locator('#rival').selectOption(fighter==='bulwark'?'relay':'bulwark');
    await page.locator('#mode').selectOption(fighter==='bulwark'?'2':'1');
    await page.locator('#play').click();
    expect((await snapshot(page)).actors[0].style).toBe(style);
    if (fighter==='bulwark') {
      await page.keyboard.down('d');
      for (let i=0;i<160;i++) {const s=await snapshot(page);if(s.actors[1].x-s.actors[0].x<=64)break;await page.clock.runFor(17);}
      await page.keyboard.up('d');await page.clock.runFor(17);
    }
    const start=await snapshot(page);
    await page.keyboard.press('u');
    if (fighter==='relay'||fighter==='sable') {
      let s=await snapshot(page);
      for (let i=0;i<35&&!s.projectiles.length;i++){await page.clock.runFor(17);s=await snapshot(page);}
      expect(s.projectiles.length).toBeGreaterThan(0);
      const x=s.projectiles[0].x;await page.clock.runFor(34);
      expect((await snapshot(page)).projectiles[0].x).toBeGreaterThan(x);
      witnesses.push({fighter,style,projectile:true});
    } else if (fighter==='bulwark') {
      await page.clock.runFor(300);const s=await snapshot(page);
      expect(s.stats.throws[0]).toBe(1);expect(s.stats.blocks[1]).toBe(0);
      expect(s.actors[1].hp).toBeLessThan(start.actors[1].hp);
      witnesses.push({fighter,style,guardBroken:true,damage:start.actors[1].hp-s.actors[1].hp});
    } else {
      await page.clock.runFor(250);const s=await snapshot(page);
      expect(s.actors[0].x).toBeGreaterThan(start.actors[0].x+40);
      expect(s.projectiles.length).toBe(0);witnesses.push({fighter,style,lunge:s.actors[0].x-start.actors[0].x});
    }
    await page.locator('#diagnostics summary').click();await page.locator('#verify').click();
    await expect(page.locator('#replay')).toContainText('PASS');await page.locator('#diagnostics summary').click();
  }
  expect(new Set(witnesses.map(w=>w.style)).size).toBe(4);
  await testInfo.attach('roster-witnesses.json',{body:JSON.stringify(witnesses,null,2),contentType:'application/json'});
});

test('down-heavy launcher, jump cancel and air chain work from the keyboard', async ({ page }, testInfo) => {
  await load(page,'1','zip','bulwark');
  await page.keyboard.down('d');
  for (let i=0;i<130;i++){const s=await snapshot(page);if(s.actors[1].x-s.actors[0].x<=52)break;await page.clock.runFor(17);}
  await page.keyboard.up('d');await page.clock.runFor(17);
  expect((await snapshot(page)).actors[1].x-(await snapshot(page)).actors[0].x).toBeLessThanOrEqual(58);
  await page.keyboard.down('s');await page.clock.runFor(34);await page.keyboard.down('l');
  await page.clock.runFor(34);const afterInput=await snapshot(page);expect(afterInput.actors[0].id, JSON.stringify(afterInput)).toBe('launch');
  await page.keyboard.up('l');await page.keyboard.up('s');
  for(let i=0;i<30&&!((await snapshot(page)).actors[0].confirmed);i++)await page.clock.runFor(17);
  expect((await snapshot(page)).actors[0].id, JSON.stringify(await snapshot(page))).toBe('launch');
  expect((await snapshot(page)).actors[1].y).toBeLessThan(0);
  await page.keyboard.down('d');await page.keyboard.press('w');
  for(let i=0;i<20&&(await snapshot(page)).actors[0].y===0;i++)await page.clock.runFor(17);
  expect((await snapshot(page)).actors[0].y).toBeLessThan(0);
  await page.keyboard.up('d');
  for(let i=0;i<75;i++){
    const a=(await snapshot(page)).actors[0];
    if(a.id==='jump')await page.keyboard.press('j');
    else if(a.id==='air_light'&&a.confirmed)await page.keyboard.press('k');
    else if(a.id==='air_medium'&&a.confirmed)await page.keyboard.press('l');
    await page.clock.runFor(17);
    const s=await snapshot(page);
    if(s.stats.air_hits[0]>=2){await page.screenshot({path:testInfo.outputPath('air-combo.png')});break;}
  }
  const s=await snapshot(page);expect(s.stats.air_hits[0]).toBeGreaterThanOrEqual(2);expect(s.actors[1].combo).toBeGreaterThanOrEqual(3);
  await page.locator('#diagnostics summary').click();await page.locator('#verify').click();await expect(page.locator('#replay')).toContainText('PASS');
});

test.describe('phone', () => {
  test.use({ viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true });
  test('multi-touch jump/attack, cancellation and quick taps work without scrolling', async ({ page, context }, testInfo) => {
    await load(page, '1'); await fits(page);
    const client = await context.newCDPSession(page);
    const center = async (selector: string, id: number) => { const b = (await page.locator(selector).boundingBox())!; return { x: b.x + b.width / 2, y: b.y + b.height / 2, id }; };
    const right = await center('[data-input="2"]', 1), jump = await center('[data-input="64"]', 2);
    await client.send('Input.dispatchTouchEvent', { type: 'touchStart', touchPoints: [right] });
    await page.clock.runFor(1200);
    const moved = (await snapshot(page)).actors[0].x;
    await client.send('Input.dispatchTouchEvent', { type: 'touchStart', touchPoints: [right, jump] });
    await page.clock.runFor(250);
    const guarded = await snapshot(page);
    expect(guarded.actors[0].x).toBeGreaterThan(moved);
    expect(guarded.actors[0].y).toBeLessThan(0);
    await client.send('Input.dispatchTouchEvent', { type: 'touchCancel', touchPoints: [] });
    await page.clock.runFor(900);
    const stopped = await snapshot(page);
    await page.clock.runFor(150);
    expect((await snapshot(page)).actors[0].x).toBe(stopped.actors[0].x);
    expect(await page.locator('.held').count()).toBe(0);
    await page.locator('[data-input="4"]').tap(); await page.clock.runFor(180);
    expect((await snapshot(page)).actors[0].id).toBe('light');
    await fits(page);
    await page.screenshot({ path: testInfo.outputPath('mobile.png') });
    await client.detach();
  });
});
