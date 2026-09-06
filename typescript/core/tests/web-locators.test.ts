import { test, expect } from 'bun:test';
import { PageImpl } from '../src/page.js';
import { normalizeSelectorForTransport } from '../src/selectors.js';
import type { RuntimeClient } from '../src/types.js';
const page = () => new PageImpl({ sessionId: 'page', browserSessionId: 'browser', runtime: {} as RuntimeClient });

test('regex, false filters and quoted nested selectors survive transport normalization', () => {
  const p = page();
  const inner = p.getByRole('button', { name: /^Save "now"$/gi, pressed: false });
  expect(JSON.parse(JSON.parse(inner.selector.slice(3)))).toEqual({kind: 'role', role: 'button', name: {regex: '^Save "now"$', flags: 'gi'}, pressed: false});
  const locator = p.locator('article', { has: inner, hasNotText: 'xpath="trap"', visible: false }).getByTestId('a"b').last();
  expect(normalizeSelectorForTransport(locator.selector)).toBe(locator.selector);
  expect(locator.selector.startsWith('css="article" aw=')).toBe(true);
  expect(inner.selector).not.toContain('filter');
});
test('invalid scope and numeric options fail before sending a command', () => {
  const p = page();
  expect(() => p.locator('li').filter({ has: page().getByText('other page') })).toThrow('same page');
  expect(() => p.getByRole('heading', {level: 0})).toThrow('positive integer');
  expect(() => p.locator('li').nth(1.5)).toThrow('integer');
});

test('locator exclusion is immutable, encoded as identity exclusion, and rejects other pages', () => {
  const p = page();
  const original = p.getByRole('button');
  const other = p.getByText('Cancel');
  const result = original.not(other).first();
  expect(result.selector).toContain('exclude');
  expect(original.selector).not.toContain('exclude');
  expect(normalizeSelectorForTransport(result.selector)).toBe(result.selector);
  expect(() => original.not(page().getByRole('button'))).toThrow('same page');
});
