import { describe, expect as check, test, vi } from 'vitest';
import { expect } from '../src/index.js';
import type { Locator, MobileAndroidLocator, Page, MobileAndroidApp } from '@allwright.dev/core';

function fixture(web = true) {
  const count = vi.fn(async (_options?: unknown) => ({ count: 1 }));
  const visibleCount = vi.fn(async (_options?: unknown) => ({ count: 1 }));
  const textContent = vi.fn(async (_options?: unknown) => ({ text: 'Ready' }));
  const waitFor = vi.fn(async (_options?: unknown) => ({ visible: true }));
  const locator = {
    page: {}, click: async () => {}, count, textContent, waitFor,
    ...(web ? { filter: vi.fn(() => ({ count: visibleCount })) } : {}),
  } as unknown as Locator;
  const page = { locator: () => locator } as unknown as Page;
  return { locator, page, count, visibleCount, textContent, waitFor };
}
const once = { timeoutMs: 0 };
const retry = { timeoutMs: 100, intervalMs: 1 };

for (const web of [true, false]) describe(web ? 'web' : 'Android', () => {
  test('property and callable negation work on locators and pages without mutating matchers', async () => {
    const f = fixture(web);
    const matchers = expect(f.locator);
    await matchers.not().toHaveCount(0, once);
    await matchers.toHaveCount(1, once);
    await matchers.not.not().toHaveCount(1, once);
    await matchers.not.toHaveText('Loading', once);
    await expect(f.page).not().toContainText('ignored selector', /load/i, once);
    await expect(f.page).not.toHaveCount('ignored selector', 0, once);
    // These assignments also exercise the public mobile matcher overloads.
    if (!web) {
      await expect(f.locator as unknown as MobileAndroidLocator).not().toHaveText('Loading', once);
      await expect(f.page as unknown as MobileAndroidApp).not.toHaveCount('ignored', 0, once);
    }
  });
  test('negative text retries the inverted predicate', async () => {
    const f = fixture(web);
    f.textContent.mockResolvedValueOnce({ text: 'Loading' }).mockResolvedValue({ text: 'Ready' });
    await expect(f.locator).not.toHaveText('Loading', retry);
    check(f.textContent).toHaveBeenCalledTimes(2);
    f.textContent.mockResolvedValueOnce({ text: 'Still loading' }).mockResolvedValue({ text: 'Ready' });
    await expect(f.page).not().toContainText('ignored', /loading/gi, retry);
  });
  test('negative count retries until counts differ and failures retain the negative assertion', async () => {
    const f = fixture(web);
    f.count.mockResolvedValueOnce({ count: 2 }).mockResolvedValue({ count: 1 });
    await expect(f.locator).not().toHaveCount(2, retry);
    check(f.count).toHaveBeenCalledTimes(2);
    await check(expect(f.locator).not.toHaveCount(1, once)).rejects.toThrow();
    await check(expect(f.locator).not.toHaveText('Ready', once)).rejects.toThrow();
    await check(expect(f.locator).not().toContainText('ead', once)).rejects.toThrow();
  });
  test('transport and selector errors never satisfy negative assertions', async () => {
    const f = fixture(web);
    const failure = new Error('connection closed');
    f.count.mockRejectedValue(failure);
    f.visibleCount.mockRejectedValue(failure);
    f.textContent.mockRejectedValue(failure);
    await check(expect(f.locator).not.toHaveCount(1, once)).rejects.toBe(failure);
    await check(expect(f.locator).not.toHaveText('Ready', once)).rejects.toBe(failure);
    await check(expect(f.locator).not().toBeVisible(once)).rejects.toBe(failure);
  });
});

test('web negative visibility observes hidden and removed elements without waiting for visibility', async () => {
  const f = fixture();
  f.visibleCount.mockResolvedValueOnce({ count: 1 }).mockResolvedValue({ count: 0 });
  await expect(f.locator).not().toBeVisible(retry);
  check(f.visibleCount).toHaveBeenCalledTimes(2);
  check(f.waitFor).not.toHaveBeenCalled();
  await expect(f.page).not.toBeVisible('missing', once);
  f.visibleCount.mockResolvedValueOnce({ count: 0 }).mockResolvedValue({ count: 1 });
  await expect(f.locator).toBeVisible(retry);
  await check(expect(f.locator).not.toBeVisible(once)).rejects.toThrow();
});

test('Android negative visibility accepts absence and only the native hidden-node error', async () => {
  const f = fixture(false);
  f.count.mockResolvedValueOnce({ count: 0 });
  await expect(f.locator).not().toBeVisible(once);
  check(f.waitFor).not.toHaveBeenCalled();
  f.waitFor.mockRejectedValueOnce(new Error('selector found but not visible: text="Hidden"'));
  await expect(f.locator).not.toBeVisible(once);
  const failure = new Error('invalid selector with text selector found but not visible:');
  f.waitFor.mockRejectedValue(failure);
  await check(expect(f.locator).not.toBeVisible(once)).rejects.toBe(failure);
});

test('regex retries do not depend on a caller regex lastIndex', async () => {
  const f = fixture();
  const pattern = /Ready/g;
  pattern.lastIndex = 5;
  await expect(f.locator).toHaveText(pattern, once);
  await check(expect(f.locator).not.toHaveText(pattern, once)).rejects.toThrow();
  check(pattern.lastIndex).toBe(5);
});

test('nested command timeouts are bounded by the assertion budget', async () => {
  const f = fixture();
  await expect(f.locator).not.toHaveText('Loading', { timeoutMs: 10, command: { timeoutMs: 60_000 } });
  const command = f.textContent.mock.calls[0][0] as { timeoutMs: number };
  check(command.timeoutMs).toBeGreaterThan(0);
  check(command.timeoutMs).toBeLessThanOrEqual(10);
});

test('ordinary Vitest assertions and asymmetric negation remain available', () => {
  expect(1).not.toBe(2);
  expect({ value: 'ready' }).toEqual({ value: expect.not.stringContaining('loading') });
});
