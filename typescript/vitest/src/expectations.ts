import type { CommandOptions, Locator, MobileAndroidApp, MobileAndroidLocator, Page, WaitForSelectorOptions } from "@allwright.dev/core";
import { expect as vitestExpect } from "vitest";

export interface RetryExpectationOptions { timeoutMs?: number; intervalMs?: number }
export interface TextExpectationOptions extends RetryExpectationOptions { command?: CommandOptions }
export interface VisibleExpectationOptions extends RetryExpectationOptions { command?: WaitForSelectorOptions }
export interface PageExpectMatchers {
  readonly not: PageExpectMatchers & (() => PageExpectMatchers);
  toHaveText(selector: string, expected: string | RegExp, options?: TextExpectationOptions): Promise<void>;
  toContainText(selector: string, expected: string | RegExp, options?: TextExpectationOptions): Promise<void>;
  toHaveCount(selector: string, expected: number, options?: RetryExpectationOptions): Promise<void>;
  toBeVisible(selector: string, options?: VisibleExpectationOptions): Promise<void>;
}
export interface LocatorExpectMatchers {
  readonly not: LocatorExpectMatchers & (() => LocatorExpectMatchers);
  toHaveText(expected: string | RegExp, options?: TextExpectationOptions): Promise<void>;
  toContainText(expected: string | RegExp, options?: TextExpectationOptions): Promise<void>;
  toHaveCount(expected: number, options?: RetryExpectationOptions): Promise<void>;
  toBeVisible(options?: VisibleExpectationOptions): Promise<void>;
}
type Defaults = () => RetryExpectationOptions;
type AnyLocator = Locator | MobileAndroidLocator;

function callableMatchers<T extends object>(matchers: T): T & (() => T) {
  return Object.defineProperties(() => matchers, Object.getOwnPropertyDescriptors(matchers)) as T & (() => T);
}

async function retryExpectation(
  callback: (command: CommandOptions) => Promise<void>,
  options: TextExpectationOptions,
  defaults: RetryExpectationOptions,
): Promise<void> {
  const timeoutMs = options.timeoutMs ?? defaults.timeoutMs ?? 5_000;
  const intervalMs = options.intervalMs ?? defaults.intervalMs ?? 100;
  if (!Number.isFinite(timeoutMs) || timeoutMs < 0 || !Number.isFinite(intervalMs) || intervalMs < 0) {
    throw new Error("Expectation timeoutMs and intervalMs must be finite non-negative numbers");
  }
  const deadline = Date.now() + timeoutMs;
  while (true) {
    try {
      // The assertion owns retrying; do not let a nested command use the server's full default timeout.
      await callback({ timeoutMs: Math.max(1, Math.min(options.command?.timeoutMs ?? Math.max(1, intervalMs), deadline - Date.now())) });
      return;
    } catch (error) {
      if (Date.now() + intervalMs >= deadline) throw error;
      await new Promise(resolve => setTimeout(resolve, intervalMs));
    }
  }
}

async function isVisible(locator: AnyLocator, command: CommandOptions): Promise<boolean> {
  if ("filter" in locator) {
    return (await locator.filter({ visible: true }).count(command)).count > 0;
  }
  // Android has no visibility-filter query. An absent node is not visible; only
  // its explicit native visibility failure is a negative observation.
  if ((await locator.count(command)).count === 0) return false;
  try {
    return (await locator.waitFor({ ...command, visible: true })).visible;
  } catch (error) {
    if (error instanceof Error && error.message.startsWith("selector found but not visible: ")) return false;
    throw error;
  }
}

export function createLocatorExpect(locator: AnyLocator, defaults: Defaults, negated = false): LocatorExpectMatchers {
  const assertion = (value: unknown) => negated ? vitestExpect(value).not : vitestExpect(value);
  return {
    get not() { return callableMatchers(createLocatorExpect(locator, defaults, !negated)); },
    async toHaveText(expected, options = {}) {
      await retryExpectation(async command => {
        const { text } = await locator.textContent(command);
        if (expected instanceof RegExp) assertion(text).toMatch(new RegExp(expected.source, expected.flags));
        else assertion(text).toBe(expected);
      }, options, defaults());
    },
    async toContainText(expected, options = {}) {
      await retryExpectation(async command => {
        const { text } = await locator.textContent(command);
        if (expected instanceof RegExp) assertion(text).toMatch(new RegExp(expected.source, expected.flags));
        else assertion(text).toContain(expected);
      }, options, defaults());
    },
    async toHaveCount(expected, options = {}) {
      await retryExpectation(async command => {
        assertion((await locator.count(command)).count).toBe(expected);
      }, options, defaults());
    },
    async toBeVisible(options = {}) {
      await retryExpectation(async command => {
        assertion(await isVisible(locator, command)).toBe(true);
      }, options, defaults());
    },
  };
}

export function createPageExpect(page: Page | MobileAndroidApp, defaults: Defaults, negated = false): PageExpectMatchers {
  const matcher = (selector: string) => createLocatorExpect(page.locator(selector), defaults, negated);
  return {
    get not() { return callableMatchers(createPageExpect(page, defaults, !negated)); },
    toHaveText: (selector, expected, options) => matcher(selector).toHaveText(expected, options),
    toContainText: (selector, expected, options) => matcher(selector).toContainText(expected, options),
    toHaveCount: (selector, expected, options) => matcher(selector).toHaveCount(expected, options),
    toBeVisible: (selector, options) => matcher(selector).toBeVisible(options),
  };
}
