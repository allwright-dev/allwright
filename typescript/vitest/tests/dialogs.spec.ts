import { expect, test as base } from '../dist/index.js';
import { hooks, shutdown } from '@allwright.dev/core';
import { afterAll } from 'vitest';

const test = base.extend({
  allwright: async ({}, use) => {
    await use({ browser: process.env.ALLWRIGHT_TEST_BROWSER === 'firefox' ? 'firefox' : 'chromium' });
  },
});
afterAll(() => shutdown());

test.skipIf(!process.env.ALLWRIGHT_TEST_BROWSER)('consecutive dialogs survive one-shot hook transitions', { timeout: 60_000 }, async ({ page }) => {
  await page.goto('data:text/html,' + encodeURIComponent(`
    <output id="result"></output>
    <button onclick="alert('first'); const yes=confirm('second'); const name=prompt('third', 'default'); document.querySelector('#result').textContent=JSON.stringify([yes,name])">Open</button>
  `));
  const options = { timeoutMs: 3000 };
  let hook = await page.registerHook(hooks.dialog);
  await page.click('button');
  const first = await hook.wait(options);
  expect(first.message).toBe('first');
  await first.accept(undefined, options);
  // Let the resumed script open its next modal before registering another hook.
  await new Promise(resolve => setTimeout(resolve, 100));
  hook = await page.registerHook(hooks.dialog);
  const second = await hook.wait(options);
  expect(second.message).toBe('second');
  await second.dismiss(options);
  await new Promise(resolve => setTimeout(resolve, 100));
  hook = await page.registerHook(hooks.dialog);
  const third = await hook.wait(options);
  expect(third.message).toBe('third');
  await third.accept('Ada', options);
  await expect(page.locator('#result')).toHaveText('[false,"Ada"]');
  await expect(first.dismiss(options)).rejects.toThrow(/already handled/);
});

for (const event of ['keydown', 'keyup', 'input', 'change', 'focus', 'mouseover'] as const) {
  test.skipIf(!process.env.ALLWRIGHT_TEST_BROWSER)(`dialog hooks yield during ${event} handlers`, { timeout: 30_000 }, async ({ page }) => {
    await page.goto('data:text/html,' + encodeURIComponent(`
      <output id="result"></output>
      <button id="neutral">Neutral</button><div role="group" aria-label="Profile"><label>Name<input id="target"></label></div>
      <script>document.querySelector('#target').addEventListener('${event}', () => {
        const answer=prompt('${event}', 'default');
        document.querySelector('#result').textContent=answer===null?'dismissed':answer;
      }, { once: true });</script>
    `));
    // Activate the document and establish a different focused element first.
    await page.locator('#neutral').click();
    const hook = await page.registerHook(hooks.dialog);
    if (event === 'keydown' || event === 'keyup') await page.locator('#target').press('Enter');
    else if (event === 'input' || event === 'change') await page.locator('#target').fill('typed');
    else if (event === 'focus') await page.getByRole('group', { name: 'Profile' }).getByLabel('Name').focus();
    else await page.locator('#target').hover();
    const dialog = await hook.wait({ timeoutMs: 2000 });
    expect(dialog.message).toBe(event);
    if (event === 'change') await dialog.dismiss(); else await dialog.accept('handled');
    await expect(page.locator('#result')).toHaveText(event === 'change' ? 'dismissed' : 'handled');
    // Normal input and reads still work after the suspended script resumes.
    await page.locator('#target').fill('after');
    expect(await page.locator('#target').inputValue()).toBe('after');
    await expect(page.locator('#missing').fill('x', { timeoutMs: 50 })).rejects.toThrow();
  });
}

test.skipIf(!process.env.ALLWRIGHT_TEST_BROWSER)('dialog hooks accept, dismiss, and supply prompt text', { timeout: 90_000 }, async ({ page }) => {
  await page.goto('data:text/html,' + encodeURIComponent(`
    <output id="result"></output>
    <button id="alert" onclick="alert('Hello');document.querySelector('#result').textContent='alert done'">Alert</button>
    <button id="confirm" onclick="document.querySelector('#result').textContent=String(confirm('Continue?'))">Confirm</button>
    <button id="prompt" onclick="document.querySelector('#result').textContent=JSON.stringify(prompt('Name?', 'default'))">Prompt</button>
  `));
  const cases = [
    { selector: '#alert', type: 'alert', message: 'Hello', accept: true, expected: 'alert done' },
    { selector: '#confirm', type: 'confirm', message: 'Continue?', accept: false, expected: 'false' },
    { selector: '#confirm', type: 'confirm', message: 'Continue?', accept: true, expected: 'true' },
    { selector: '#prompt', type: 'prompt', message: 'Name?', accept: true, text: 'Ada "☃"', expected: JSON.stringify('Ada "☃"') },
    { selector: '#prompt', type: 'prompt', message: 'Name?', accept: true, text: '', expected: '""' },
    { selector: '#prompt', type: 'prompt', message: 'Name?', accept: true, expected: '"default"' },
    { selector: '#prompt', type: 'prompt', message: 'Name?', accept: false, expected: 'null' },
  ];
  for (const c of cases) {
    const hook = await page.registerHook(hooks.dialog);
    await expect(page.registerHook(hooks.dialog)).rejects.toThrow(/already exists/);
    await page.click(c.selector);
    const dialog = await hook.wait({ timeoutMs: 5000 });
    expect(dialog.type).toBe(c.type);
    expect(dialog.message).toBe(c.message);
    if (c.type === 'prompt') expect(dialog.defaultValue).toBe('default');
    if (c.type !== 'prompt') await expect(dialog.accept('invalid')).rejects.toThrow(/prompt text/);
    if (c.accept) await dialog.accept(c.text); else await dialog.dismiss();
    await expect(page.locator('#result')).toHaveText(c.expected);
    await expect(dialog.dismiss()).rejects.toThrow(/already handled/);
  }
});

test.skipIf(!process.env.ALLWRIGHT_TEST_BROWSER)('closing drains consecutive dialogs', { timeout: 30_000 }, async ({ browser }) => {
  const page = await browser.newPage();
  await page.goto('data:text/html,' + encodeURIComponent(`<button onclick="alert('one');alert('two');alert('three')">Open</button>`));
  const hook = await page.registerHook(hooks.dialog);
  await page.click('button');
  expect((await hook.wait()).message).toBe('one');
  await Promise.race([
    page.close(),
    new Promise((_, reject) => setTimeout(() => reject(new Error('page close did not drain dialogs')), 5000)),
  ]);
});

test.skipIf(!process.env.ALLWRIGHT_TEST_BROWSER)('dialog handling deadline remains pollable', { timeout: 30_000 }, async ({ page }) => {
  await page.goto('data:text/html,' + encodeURIComponent(`<button onclick="alert('deadline');const end=performance.now()+250;while(performance.now()<end){}">Open</button>`));
  const hook = await page.registerHook(hooks.dialog);
  await page.click('button');
  const dialog = await hook.wait();
  await expect(dialog.dismiss({ timeoutMs: 0 })).rejects.toThrow(/timed out handling dialog/);
  // The single attempt may finish after its caller deadline, but is never replayed.
  await new Promise(resolve => setTimeout(resolve, 250));
  await expect(dialog.dismiss()).rejects.toThrow(/already handled/);
});

test.skipIf(!process.env.ALLWRIGHT_TEST_BROWSER)('dialog hooks isolate pages and survive a wait timeout', { timeout: 60_000 }, async ({ browser }) => {
  const first = await browser.newPage();
  const second = await browser.newPage();
  const url = 'data:text/html,' + encodeURIComponent('<button onclick="confirm(\'Scoped\')">Open</button>');
  await first.goto(url);
  await second.goto(url);
  const a = await first.registerHook(hooks.dialog);
  const b = await second.registerHook(hooks.dialog);
  await first.click('button');
  await expect(b.wait({ timeoutMs: 100 })).rejects.toThrow();
  await (await a.wait()).dismiss();
  await second.click('button');
  await (await b.wait()).accept();
  // Closing a page with an unused hook must release its subscription.
  await first.registerHook(hooks.dialog);
  await first.close();
  await second.close();
});
