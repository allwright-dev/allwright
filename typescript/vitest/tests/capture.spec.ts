import { expect, test as base } from '../dist/index.js';
import { shutdown } from '@allwright.dev/core';
import { afterAll } from 'vitest';
import { createServer } from 'node:http';

const test = base.extend({
  allwright: async ({}, use) => {
    await use({ browser: process.env.ALLWRIGHT_TEST_BROWSER === 'firefox' ? 'firefox' : 'chromium' });
  },
});
afterAll(() => shutdown());

test.skipIf(!process.env.ALLWRIGHT_TEST_BROWSER)('capture live page and element state through the engine', { timeout: 60_000 }, async ({ page }) => {
  const input = page.getByLabel('Name', { exact: true });
  const html = `<label>Name<input id="name" value="initial"></label>
    <textarea id="notes">hello world</textarea><input id="number" type="number" value="42">
    <input id="checked" type="checkbox" checked><input id="radio" type="radio">
    <div role="checkbox" aria-checked="mixed" id="mixed"></div>
    <select id="choices" multiple><option value="a" selected>Alpha</option><option value="b">Beta</option><option value="c" selected label="Charlie">C</option></select>
    <input role="combobox" aria-controls="list" id="combo"><div role="listbox" id="list"><div role="option" aria-selected="true" value="x">Custom</div></div>
    <div id="box" data-empty="" data-quote='a&quot;b' style="position:fixed;left:20px;top:30px;width:100px;height:40px">Hello<span style="display:none"> hidden</span></div>
    <div id="hidden" style="display:none">Hidden</div>
    <iframe id="frame" srcdoc="<input value='inside'>"></iframe>
    <script>document.querySelector('#notes').setSelectionRange(1,4);</script>`;
  const url = 'data:text/html,' + encodeURIComponent(html);
  await page.goto(url);
  expect(await page.url()).toBe(url);
  expect(await input.inputValue()).toBe('initial');
  await input.fill('changed');
  expect(await input.inputValue()).toBe('changed');
  expect(await input.getAttribute('value')).toBe('initial');
  expect(await page.inputValue('#notes')).toBe('hello world');
  expect(await page.locator('#notes').selectedText()).toBe('ell');
  expect(await page.selectedText('#number')).toBeNull();
  expect(await page.locator('#choices').selectedOptions()).toEqual([
    { value: 'a', label: 'Alpha', index: 0 }, { value: 'c', label: 'Charlie', index: 2 },
  ]);
  expect(await page.selectedOptions('#combo')).toEqual([{ value: 'x', label: 'Custom', index: 0 }]);
  expect(await page.locator('#checked').isChecked()).toBe(true);
  expect(await page.isChecked('#radio')).toBe(false);
  expect(await page.isChecked('#mixed')).toBe(false);
  expect(await page.locator('#box').textContent()).toMatchObject({ text: 'Hello hidden' });
  expect(await page.innerText('#box')).toMatchObject({ text: 'Hello' });
  expect(await page.getAttribute('#box', 'data-empty')).toBe('');
  expect(await page.getAttribute('#box', 'data-quote')).toBe('a"b');
  expect(await page.getAttribute('#box', 'absent')).toBeNull();
  expect(await page.locator('#box').boundingBox()).toEqual({ x: 20, y: 30, width: 100, height: 40 });
  expect(await page.boundingBox('#hidden')).toBeNull();
  await expect(page.inputValue('#box', { timeoutMs: 100 })).rejects.toThrow(/inputValue/);
  await expect(page.getAttribute('#absent', 'id', { timeoutMs: 100 })).rejects.toThrow(/No element/);
  const frame = await page.locator('#frame').frame();
  expect(await frame.url()).toBe('about:srcdoc');
  expect(await frame.locator('input').inputValue()).toBe('inside');
});

test.skipIf(!process.env.ALLWRIGHT_TEST_BROWSER)('state expectations wait for asynchronous browser updates', { timeout: 60_000 }, async ({ page }) => {
  const html = `
    <input id="input" value="loading"><textarea id="text">hello world</textarea>
    <input id="check" type="checkbox"><select id="select" multiple><option value="a">Alpha</option><option value="b">Beta</option></select>
    <div id="box" style="width:10px;height:10px">Loading</div>
    <button id="value" onclick="setTimeout(() => document.querySelector('#input').value = 'ready', 150)">Value</button>
    <button id="selection" onclick="setTimeout(() => document.querySelector('#text').setSelectionRange(0,5), 150)">Selection</button>
    <button id="options" onclick="setTimeout(() => document.querySelector('#select').options[1].selected = true, 150)">Options</button>
    <button id="checked" onclick="setTimeout(() => document.querySelector('#check').checked = true, 150)">Checked</button>
    <button id="attribute" onclick="setTimeout(() => document.querySelector('#box').setAttribute('data-state', 'ready'), 150)">Attribute</button>
    <button id="size" onclick="setTimeout(() => document.querySelector('#box').style.width = '100px', 150)">Size</button>
    <button id="content" onclick="setTimeout(() => document.querySelector('#box').textContent = 'Ready', 150)">Text</button>
    <button id="url" onclick="setTimeout(() => location.hash = 'done', 150)">URL</button>
  `;
  const server = createServer((_request, response) => {
    response.writeHead(200, { 'Content-Type': 'text/html' });
    response.end(html);
  });
  await new Promise<void>(resolve => server.listen(0, '127.0.0.1', resolve));
  const address = server.address();
  if (!address || typeof address === 'string') throw new Error('Missing fixture address');
  try {
    await page.goto(`http://127.0.0.1:${address.port}/`);
    await page.locator('#value').click();
    await expect(page.locator('#input')).toHaveValue(/^ready$/);
    await page.locator('#selection').click();
    await expect(page).toHaveSelectedText('#text', 'hello');
    await page.locator('#options').click();
    await expect(page.locator('#select')).toHaveSelectedOptions([{ value: 'b', label: /Beta/ }]);
    await page.locator('#checked').click();
    await expect(page).toBeChecked('#check');
    await page.locator('#attribute').click();
    await expect(page.locator('#box')).not().toHaveAttribute('data-state', null);
    await page.locator('#size').click();
    await expect(page).toHaveBoundingBox('#box', { width: 100, height: 10 });
    await page.locator('#content').click();
    await expect(page.locator('#box')).toHaveText('Ready');
    await page.locator('#url').click();
    await expect(page).toHaveURL(/#done$/g);
    await expect(page).not.toHaveURL(/#loading$/);
  } finally {
    server.closeAllConnections();
    await new Promise<void>((resolve, reject) => server.close(error => error ? reject(error) : resolve()));
  }
});
