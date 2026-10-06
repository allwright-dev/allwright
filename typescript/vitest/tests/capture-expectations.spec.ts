import { afterEach, describe, expect as check, test, vi } from 'vitest';
import { expect } from '../src/index.js';
import { createPageExpect } from '../src/expectations.js';
import type { Locator, NativeApp, NativeAppLocator, Page } from '@allwright.dev/core';

const retry = { timeoutMs: 200, intervalMs: 1 };
const once = { timeoutMs: 0 };
function fixture() {
  const reads = {
    inputValue: vi.fn(), selectedOptions: vi.fn(), selectedText: vi.fn(), isChecked: vi.fn(),
    getAttribute: vi.fn(), boundingBox: vi.fn(), url: vi.fn(),
  };
  const locator = { ...reads, page: {}, click: vi.fn(), count: vi.fn(), textContent: vi.fn() } as unknown as Locator;
  const page = { url: reads.url, locator: vi.fn(() => locator) } as unknown as Page;
  return { reads, locator, page };
}
const cases = [
  { name: 'URL', read: 'url', before: 'https://example.test/loading', after: 'https://example.test/home',
    run: (f, negative, options) => (negative ? expect(f.page).not() : expect(f.page)).toHaveURL(/\/home$/g, options) },
  { name: 'value', read: 'inputValue', before: 'loading', after: 'ready',
    run: (f, negative, options) => (negative ? expect(f.locator).not : expect(f.locator)).toHaveValue(/ready/g, options) },
  { name: 'selected options', read: 'selectedOptions', before: [], after: [{ value: 'b', label: 'Beta', index: 1 }],
    run: (f, negative, options) => (negative ? expect(f.locator).not() : expect(f.locator)).toHaveSelectedOptions([{ value: /b/g, label: /Beta/, index: 1 }], options) },
  { name: 'selected text', read: 'selectedText', before: '', after: 'selection',
    run: (f, negative, options) => (negative ? expect(f.locator).not : expect(f.locator)).toHaveSelectedText(/selection/g, options) },
  { name: 'checked', read: 'isChecked', before: false, after: true,
    run: (f, negative, options) => (negative ? expect(f.locator).not() : expect(f.locator)).toBeChecked(options) },
  { name: 'attribute', read: 'getAttribute', before: null, after: 'ready',
    run: (f, negative, options) => (negative ? expect(f.locator).not : expect(f.locator)).toHaveAttribute('data-state', /ready/g, options) },
  { name: 'bounding box', read: 'boundingBox', before: null, after: { x: 10, y: 20, width: 100, height: 50 },
    run: (f, negative, options) => (negative ? expect(f.locator).not() : expect(f.locator)).toHaveBoundingBox({ width: 100, height: 50 }, options) },
] satisfies Array<{ name: string; read: keyof ReturnType<typeof fixture>['reads']; before: unknown; after: unknown;
  run: (f: ReturnType<typeof fixture>, negative: boolean, options: typeof retry | typeof once) => Promise<void> }>;

afterEach(() => vi.useRealTimers());
for (const c of cases) describe(c.name, () => {
  test('positive and negative assertions re-read until the condition matches', async () => {
    const f = fixture();
    f.reads[c.read].mockResolvedValueOnce(c.before).mockResolvedValue(c.after);
    await c.run(f, false, retry);
    check(f.reads[c.read]).toHaveBeenCalledTimes(2);
    f.reads[c.read].mockReset().mockResolvedValueOnce(c.after).mockResolvedValue(c.before);
    await c.run(f, true, retry);
    check(f.reads[c.read]).toHaveBeenCalledTimes(2);
  });
  test('mismatches and read failures fail for both polarities', async () => {
    const f = fixture();
    f.reads[c.read].mockResolvedValue(c.before);
    await check(c.run(f, false, once)).rejects.toThrow();
    f.reads[c.read].mockResolvedValue(c.after);
    await check(c.run(f, true, once)).rejects.toThrow();
    const error = new Error('detached frame');
    f.reads[c.read].mockRejectedValue(error);
    await check(c.run(f, false, once)).rejects.toBe(error);
    await check(c.run(f, true, once)).rejects.toBe(error);
  });
  test('persistent mismatches poll until timeout', async () => {
    vi.useFakeTimers();
    const f = fixture();
    f.reads[c.read].mockResolvedValue(c.before);
    const assertion = check(c.run(f, false, { timeoutMs: 50, intervalMs: 10 })).rejects.toThrow();
    await vi.advanceTimersByTimeAsync(50);
    await assertion;
    check(f.reads[c.read]).toHaveBeenCalledTimes(5);
    check(vi.getTimerCount()).toBe(0);
  });
});

test('page delegates every element matcher and preserves options, nulls, and selection order', async () => {
  const f = fixture();
  f.reads.inputValue.mockResolvedValue('ready');
  f.reads.selectedOptions.mockResolvedValue([{ value: 'b', label: 'Beta', index: 1 }, { value: 'c', label: 'Gamma', index: 2 }]);
  f.reads.selectedText.mockResolvedValue(null);
  f.reads.isChecked.mockResolvedValue(false);
  f.reads.getAttribute.mockResolvedValue(null);
  f.reads.boundingBox.mockResolvedValue(null);
  await expect(f.page).toHaveValue('#input', 'ready', once);
  await expect(f.page).toHaveSelectedOptions('#select', ['b', /c/], once);
  await check(expect(f.page).toHaveSelectedOptions('#select', ['c', 'b'], once)).rejects.toThrow();
  await expect(f.page).toHaveSelectedText('#input', null, once);
  await expect(f.page).not().toBeChecked('#checkbox', once);
  await expect(f.page).toHaveAttribute('#input', 'missing', null, once);
  await expect(f.page).toHaveBoundingBox('#hidden', null, once);
  check(f.reads.getAttribute).toHaveBeenCalledWith('missing', { timeoutMs: 1 });
  f.reads.getAttribute.mockResolvedValue('');
  await check(expect(f.page).toHaveAttribute('#input', 'empty', null, once)).rejects.toThrow();
  await expect(f.page).toHaveAttribute('#input', 'empty', '', once);
});

test('native apps expose shared state matchers without web-only selection methods', async () => {
  const inputValue = vi.fn().mockResolvedValue('ready');
  const isChecked = vi.fn().mockResolvedValue(true);
  const getAttribute = vi.fn().mockResolvedValue('enabled');
  const boundingBox = vi.fn().mockResolvedValue({ x: 4, y: 8, width: 80, height: 24 });
  const locator = {
    inputValue, isChecked, getAttribute, boundingBox,
    page: {}, selector: 'aw=native', click: vi.fn(), count: vi.fn(), focus: vi.fn(), fill: vi.fn(),
    press: vi.fn(), textContent: vi.fn(), innerText: vi.fn(), waitFor: vi.fn(), locator: vi.fn(),
    getByRole: vi.fn(), getByText: vi.fn(), getByLabel: vi.fn(), getByTestId: vi.fn(),
  } as unknown as NativeAppLocator;
  const app = { locator: vi.fn(() => locator) } as unknown as NativeApp;

  await expect(locator).toHaveValue('ready', once);
  await expect(locator).toBeChecked(once);
  await expect(locator).toHaveAttribute('state', 'enabled', once);
  await expect(app).toHaveBoundingBox('#field', { width: 80 }, once);
  check(boundingBox).toHaveBeenCalledWith({ timeoutMs: 1 });
});

test('common presence, visibility, enabled, focus, and editable assertions work on native locators', async () => {
  const states = { enabled: 'true', focused: 'true', editable: 'true' } as const;
  const locator = {
    page: {}, selector: 'aw=native',
    count: vi.fn().mockResolvedValue(1),
    waitFor: vi.fn().mockResolvedValue(undefined),
    getAttribute: vi.fn(async (name: keyof typeof states) => states[name] ?? null),
    inputValue: vi.fn(), isChecked: vi.fn(), boundingBox: vi.fn(), click: vi.fn(), focus: vi.fn(), fill: vi.fn(),
    press: vi.fn(), textContent: vi.fn(), innerText: vi.fn(), locator: vi.fn(), getByRole: vi.fn(),
    getByText: vi.fn(), getByLabel: vi.fn(), getByTestId: vi.fn(),
  } as unknown as NativeAppLocator;

  await expect(locator).toBeAttached(once);
  await expect(locator).toBeVisible(once);
  await expect(locator).toBeEnabled(once);
  await expect(locator).not.toBeDisabled(once);
  await expect(locator).toBeFocused(once);
  await expect(locator).toBeEditable(once);
  locator.count = vi.fn().mockResolvedValue(0);
  await expect(locator).toBeHidden(once);
});

test('web state assertions use computed-state capture attributes', async () => {
  const getAttribute = vi.fn(async (name: string) => ({
    __allwright_state_enabled: 'false',
    __allwright_state_focused: 'true',
    __allwright_state_editable: 'false',
  })[name] ?? null);
  const locator = {
    page: {}, selector: '#field', getAttribute,
    filter: vi.fn(() => ({ count: vi.fn().mockResolvedValue(1) })),
    count: vi.fn().mockResolvedValue(1), inputValue: vi.fn(), selectedOptions: vi.fn(), selectedText: vi.fn(),
    isChecked: vi.fn(), boundingBox: vi.fn(), textContent: vi.fn(), click: vi.fn(), focus: vi.fn(), fill: vi.fn(),
    hover: vi.fn(), press: vi.fn(), innerText: vi.fn(), waitFor: vi.fn(), locator: vi.fn(),
  } as unknown as Locator;

  await expect(locator).toBeDisabled(once);
  await expect(locator).toBeFocused(once);
  await expect(locator).not.toBeEditable(once);
  check(getAttribute).toHaveBeenCalledWith('__allwright_state_enabled', { timeoutMs: 1 });
});

test('URL exact matching, regex state, and immutable double negation', async () => {
  const f = fixture();
  f.reads.url.mockResolvedValue('https://example.test/home');
  const pattern = /home$/g;
  pattern.lastIndex = 100;
  const matchers = expect(f.page);
  await matchers.toHaveURL(pattern, once);
  await matchers.not.not().toHaveURL('https://example.test/home', once);
  await check(matchers.toHaveURL('/home', once)).rejects.toThrow();
  await check(matchers.not.toHaveURL(pattern, once)).rejects.toThrow();
  check(pattern.lastIndex).toBe(100);
});

test('a long polling interval does not fail before the timeout', async () => {
  vi.useFakeTimers();
  const f = fixture();
  f.reads.url.mockResolvedValue('loading');
  let settled = false;
  const assertion = check(expect(f.page).toHaveURL('ready', { timeoutMs: 50, intervalMs: 1000 })
    .finally(() => { settled = true; })).rejects.toThrow();
  await vi.advanceTimersByTimeAsync(49);
  check(settled).toBe(false);
  await vi.advanceTimersByTimeAsync(1);
  await assertion;
});

test('in-flight reads cannot exceed the assertion budget and nested command timeout is bounded', async () => {
  vi.useFakeTimers();
  const f = fixture();
  f.reads.url.mockImplementation(() => new Promise(() => {}));
  const assertion = check(expect(f.page).toHaveURL('ready', { timeoutMs: 50, command: { timeoutMs: 60_000 } })).rejects.toThrow(/timed out/);
  await vi.advanceTimersByTimeAsync(50);
  await assertion;
  check(f.reads.url).toHaveBeenCalledWith({ timeoutMs: 50 });
  check(vi.getTimerCount()).toBe(0);
});

test('configured defaults apply and invalid retry options are rejected', async () => {
  vi.useFakeTimers();
  const f = fixture();
  f.reads.url.mockResolvedValue('loading');
  const assertion = check(createPageExpect(f.page, () => ({ timeoutMs: 30, intervalMs: 10 })).toHaveURL('ready')).rejects.toThrow();
  await vi.advanceTimersByTimeAsync(30);
  await assertion;
  check(f.reads.url).toHaveBeenCalledTimes(3);
  await check(expect(f.page).toHaveURL('ready', { timeoutMs: -1 })).rejects.toThrow(/finite non-negative/);
});
