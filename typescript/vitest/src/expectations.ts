import type { BoundingBox, CommandOptions, Locator, NativeApp, NativeAppLocator, Page, WaitForSelectorOptions } from "@allwright.dev/core";
import { expect as vitestExpect } from "vitest";

export interface RetryExpectationOptions { timeoutMs?: number; intervalMs?: number }
export interface TextExpectationOptions extends RetryExpectationOptions { command?: CommandOptions }
export interface VisibleExpectationOptions extends RetryExpectationOptions { command?: WaitForSelectorOptions }
export interface MobilePageExpectMatchers {
  readonly not: MobilePageExpectMatchers & (() => MobilePageExpectMatchers);
  toHaveText(selector: string, expected: string | RegExp, options?: TextExpectationOptions): Promise<void>;
  toContainText(selector: string, expected: string | RegExp, options?: TextExpectationOptions): Promise<void>;
  toHaveCount(selector: string, expected: number, options?: RetryExpectationOptions): Promise<void>;
  toBeVisible(selector: string, options?: VisibleExpectationOptions): Promise<void>;
  toBeHidden(selector: string, options?: VisibleExpectationOptions): Promise<void>;
  toBeAttached(selector: string, options?: TextExpectationOptions): Promise<void>;
  toBeEnabled(selector: string, options?: TextExpectationOptions): Promise<void>;
  toBeDisabled(selector: string, options?: TextExpectationOptions): Promise<void>;
  toBeFocused(selector: string, options?: TextExpectationOptions): Promise<void>;
  toBeEditable(selector: string, options?: TextExpectationOptions): Promise<void>;
  toHaveValue(selector: string, expected: string | RegExp, options?: TextExpectationOptions): Promise<void>;
  toBeChecked(selector: string, options?: TextExpectationOptions): Promise<void>;
  toHaveAttribute(selector: string, name: string, expected: string | RegExp | null, options?: TextExpectationOptions): Promise<void>;
  toHaveBoundingBox(selector: string, expected: Partial<BoundingBox> | null, options?: TextExpectationOptions): Promise<void>;
}
export interface MobileLocatorExpectMatchers {
  readonly not: MobileLocatorExpectMatchers & (() => MobileLocatorExpectMatchers);
  toHaveText(expected: string | RegExp, options?: TextExpectationOptions): Promise<void>;
  toContainText(expected: string | RegExp, options?: TextExpectationOptions): Promise<void>;
  toHaveCount(expected: number, options?: RetryExpectationOptions): Promise<void>;
  toBeVisible(options?: VisibleExpectationOptions): Promise<void>;
  toBeHidden(options?: VisibleExpectationOptions): Promise<void>;
  toBeAttached(options?: TextExpectationOptions): Promise<void>;
  toBeEnabled(options?: TextExpectationOptions): Promise<void>;
  toBeDisabled(options?: TextExpectationOptions): Promise<void>;
  toBeFocused(options?: TextExpectationOptions): Promise<void>;
  toBeEditable(options?: TextExpectationOptions): Promise<void>;
  toHaveValue(expected: string | RegExp, options?: TextExpectationOptions): Promise<void>;
  toBeChecked(options?: TextExpectationOptions): Promise<void>;
  toHaveAttribute(name: string, expected: string | RegExp | null, options?: TextExpectationOptions): Promise<void>;
  toHaveBoundingBox(expected: Partial<BoundingBox> | null, options?: TextExpectationOptions): Promise<void>;
}
export type SelectedOptionExpectation = string | RegExp | { value?: string | RegExp; label?: string | RegExp; index?: number };
export interface LocatorExpectMatchers extends MobileLocatorExpectMatchers {
  readonly not: LocatorExpectMatchers & (() => LocatorExpectMatchers);
  toHaveSelectedOptions(expected: SelectedOptionExpectation[], options?: TextExpectationOptions): Promise<void>;
  toHaveSelectedText(expected: string | RegExp | null, options?: TextExpectationOptions): Promise<void>;
}
export interface PageExpectMatchers extends MobilePageExpectMatchers {
  readonly not: PageExpectMatchers & (() => PageExpectMatchers);
  toHaveURL(expected: string | RegExp, options?: TextExpectationOptions): Promise<void>;
  toHaveSelectedOptions(selector: string, expected: SelectedOptionExpectation[], options?: TextExpectationOptions): Promise<void>;
  toHaveSelectedText(selector: string, expected: string | RegExp | null, options?: TextExpectationOptions): Promise<void>;
}
type Defaults = () => RetryExpectationOptions;
type AnyLocator = Locator | NativeAppLocator;

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
  let lastError: unknown;
  while (true) {
    let timer: ReturnType<typeof setTimeout> | undefined;
    try {
      // The assertion owns polling and bounds both the command hint and in-flight read.
      const attempt = callback({ timeoutMs: Math.max(1, Math.min(options.command?.timeoutMs ?? Math.max(1, intervalMs), deadline - Date.now())) });
      if (timeoutMs === 0) await attempt; // One observation, without polling.
      else await Promise.race([
        attempt,
        new Promise<never>((_, reject) => {
          timer = setTimeout(() => reject(lastError ?? new Error(`Expectation timed out after ${timeoutMs}ms while reading state`)), Math.max(0, deadline - Date.now()));
        }),
      ]);
      return;
    } catch (error) {
      lastError = error;
    } finally {
      if (timer !== undefined) clearTimeout(timer);
    }
    const remaining = deadline - Date.now();
    if (remaining <= 0) throw lastError;
    await new Promise(resolve => setTimeout(resolve, Math.min(intervalMs, remaining)));
    if (Date.now() >= deadline) throw lastError;
  }
}

function expectedText(expected: string | RegExp | null) {
  return expected instanceof RegExp ? vitestExpect.stringMatching(new RegExp(expected.source, expected.flags)) : expected;
}

async function isVisible(locator: AnyLocator, command: CommandOptions): Promise<boolean> {
  if ("filter" in locator) {
    return (await locator.filter({ visible: true }).count(command)) > 0;
  }
  // Android has no visibility-filter query. An absent node is not visible; only
  // its explicit native visibility failure is a negative observation.
  if ((await locator.count(command)) === 0) return false;
  try {
    await locator.waitFor({ ...command, visible: true });
    return true;
  } catch (error) {
    if (error instanceof Error && (
      error.message.startsWith("selector found but not visible: ")
      || error.message.includes("did not become visible")
    )) return false;
    throw error;
  }
}

async function nativeOrWebState(
  locator: AnyLocator,
  state: "enabled" | "focused" | "editable",
  command: CommandOptions,
): Promise<boolean> {
  const attribute = "filter" in locator ? `__allwright_state_${state}` : state;
  const value = await locator.getAttribute(attribute, command);
  if (value !== "true" && value !== "false") {
    throw new Error(`Element did not expose a ${state} state`);
  }
  return value === "true";
}

export function createLocatorExpect(locator: Locator, defaults: Defaults, negated?: boolean): LocatorExpectMatchers;
export function createLocatorExpect(locator: NativeAppLocator, defaults: Defaults, negated?: boolean): MobileLocatorExpectMatchers;
export function createLocatorExpect(locator: AnyLocator, defaults: Defaults, negated?: boolean): MobileLocatorExpectMatchers;
export function createLocatorExpect(locator: AnyLocator, defaults: Defaults, negated = false): MobileLocatorExpectMatchers {
  const assertion = (value: unknown) => negated ? vitestExpect(value).not : vitestExpect(value);
  const common: MobileLocatorExpectMatchers = {
    get not() { return callableMatchers(createLocatorExpect(locator, defaults, !negated)); },
    async toHaveText(expected, options = {}) {
      await retryExpectation(async command => {
        const text = await locator.textContent(command);
        if (expected instanceof RegExp) assertion(text).toMatch(new RegExp(expected.source, expected.flags));
        else assertion(text).toBe(expected);
      }, options, defaults());
    },
    async toContainText(expected, options = {}) {
      await retryExpectation(async command => {
        const text = await locator.textContent(command);
        if (expected instanceof RegExp) assertion(text).toMatch(new RegExp(expected.source, expected.flags));
        else assertion(text).toContain(expected);
      }, options, defaults());
    },
    async toHaveCount(expected, options = {}) {
      await retryExpectation(async command => {
        assertion(await locator.count(command)).toBe(expected);
      }, options, defaults());
    },
    async toBeVisible(options = {}) {
      await retryExpectation(async command => {
        assertion(await isVisible(locator, command)).toBe(true);
      }, options, defaults());
    },
    async toBeHidden(options = {}) {
      await retryExpectation(async command => {
        assertion(await isVisible(locator, command)).toBe(false);
      }, options, defaults());
    },
    async toBeAttached(options = {}) {
      await retryExpectation(async command => {
        assertion((await locator.count(command)) > 0).toBe(true);
      }, options, defaults());
    },
    async toBeEnabled(options = {}) {
      await retryExpectation(async command => {
        assertion(await nativeOrWebState(locator, "enabled", command)).toBe(true);
      }, options, defaults());
    },
    async toBeDisabled(options = {}) {
      await retryExpectation(async command => {
        assertion(await nativeOrWebState(locator, "enabled", command)).toBe(false);
      }, options, defaults());
    },
    async toBeFocused(options = {}) {
      await retryExpectation(async command => {
        assertion(await nativeOrWebState(locator, "focused", command)).toBe(true);
      }, options, defaults());
    },
    async toBeEditable(options = {}) {
      await retryExpectation(async command => {
        assertion(await nativeOrWebState(locator, "editable", command)).toBe(true);
      }, options, defaults());
    },
    async toHaveValue(expected, options = {}) {
      await retryExpectation(async command => assertion(await locator.inputValue(command)).toEqual(expectedText(expected)), options, defaults());
    },
    async toBeChecked(options = {}) {
      await retryExpectation(async command => assertion(await locator.isChecked(command)).toBe(true), options, defaults());
    },
    async toHaveAttribute(name, expected, options = {}) {
      await retryExpectation(async command => assertion(await locator.getAttribute(name, command)).toEqual(expectedText(expected)), options, defaults());
    },
    async toHaveBoundingBox(expected, options = {}) {
      await retryExpectation(async command => assertion(await locator.boundingBox(command)).toEqual(expected === null ? null : vitestExpect.objectContaining(expected)), options, defaults());
    },
  };
  if (!("selectedOptions" in locator)) return common;
  const web: LocatorExpectMatchers = {
    toHaveText: common.toHaveText,
    toContainText: common.toContainText,
    toHaveCount: common.toHaveCount,
    toBeVisible: common.toBeVisible,
    toBeHidden: common.toBeHidden,
    toBeAttached: common.toBeAttached,
    toBeEnabled: common.toBeEnabled,
    toBeDisabled: common.toBeDisabled,
    toBeFocused: common.toBeFocused,
    toBeEditable: common.toBeEditable,
    get not() { return callableMatchers(createLocatorExpect(locator, defaults, !negated)); },
    toHaveValue: common.toHaveValue,
    async toHaveSelectedOptions(expected, options = {}) {
      await retryExpectation(async command => {
        const wanted = expected.map(option => {
          if (typeof option === "string" || option instanceof RegExp) return vitestExpect.objectContaining({ value: expectedText(option) });
          return vitestExpect.objectContaining({
            ...(option.value !== undefined ? { value: expectedText(option.value) } : {}),
            ...(option.label !== undefined ? { label: expectedText(option.label) } : {}),
            ...(option.index !== undefined ? { index: option.index } : {}),
          });
        });
        assertion(await locator.selectedOptions(command)).toEqual(wanted);
      }, options, defaults());
    },
    async toHaveSelectedText(expected, options = {}) {
      await retryExpectation(async command => assertion(await locator.selectedText(command)).toEqual(expectedText(expected)), options, defaults());
    },
    toBeChecked: common.toBeChecked,
    toHaveAttribute: common.toHaveAttribute,
    toHaveBoundingBox: common.toHaveBoundingBox,
  };
  return web;
}

export function createPageExpect(page: Page, defaults: Defaults, negated?: boolean): PageExpectMatchers;
export function createPageExpect(page: NativeApp, defaults: Defaults, negated?: boolean): MobilePageExpectMatchers;
export function createPageExpect(page: Page | NativeApp, defaults: Defaults, negated?: boolean): MobilePageExpectMatchers;
export function createPageExpect(page: Page | NativeApp, defaults: Defaults, negated = false): MobilePageExpectMatchers {
  const matcher = (selector: string) => createLocatorExpect(page.locator(selector), defaults, negated);
  const common: MobilePageExpectMatchers = {
    get not() { return callableMatchers(createPageExpect(page, defaults, !negated)); },
    toHaveText: (selector, expected, options) => matcher(selector).toHaveText(expected, options),
    toContainText: (selector, expected, options) => matcher(selector).toContainText(expected, options),
    toHaveCount: (selector, expected, options) => matcher(selector).toHaveCount(expected, options),
    toBeVisible: (selector, options) => matcher(selector).toBeVisible(options),
    toBeHidden: (selector, options) => matcher(selector).toBeHidden(options),
    toBeAttached: (selector, options) => matcher(selector).toBeAttached(options),
    toBeEnabled: (selector, options) => matcher(selector).toBeEnabled(options),
    toBeDisabled: (selector, options) => matcher(selector).toBeDisabled(options),
    toBeFocused: (selector, options) => matcher(selector).toBeFocused(options),
    toBeEditable: (selector, options) => matcher(selector).toBeEditable(options),
    toHaveValue: (selector, expected, options) => matcher(selector).toHaveValue(expected, options),
    toBeChecked: (selector, options) => matcher(selector).toBeChecked(options),
    toHaveAttribute: (selector, name, expected, options) => matcher(selector).toHaveAttribute(name, expected, options),
    toHaveBoundingBox: (selector, expected, options) => matcher(selector).toHaveBoundingBox(expected, options),
  };
  if (!("url" in page)) return common;
  const webMatcher = (selector: string) => createLocatorExpect(page.locator(selector), defaults, negated);
  const web: PageExpectMatchers = {
    toHaveText: common.toHaveText,
    toContainText: common.toContainText,
    toHaveCount: common.toHaveCount,
    toBeVisible: common.toBeVisible,
    toBeHidden: common.toBeHidden,
    toBeAttached: common.toBeAttached,
    toBeEnabled: common.toBeEnabled,
    toBeDisabled: common.toBeDisabled,
    toBeFocused: common.toBeFocused,
    toBeEditable: common.toBeEditable,
    get not() { return callableMatchers(createPageExpect(page, defaults, !negated)); },
    async toHaveURL(expected, options = {}) {
      await retryExpectation(async command => {
        const url = await page.url(command);
        (negated ? vitestExpect(url).not : vitestExpect(url)).toEqual(expectedText(expected));
      }, options, defaults());
    },
    toHaveValue: common.toHaveValue,
    toHaveSelectedOptions: (selector, expected, options) => webMatcher(selector).toHaveSelectedOptions(expected, options),
    toHaveSelectedText: (selector, expected, options) => webMatcher(selector).toHaveSelectedText(expected, options),
    toBeChecked: common.toBeChecked,
    toHaveAttribute: common.toHaveAttribute,
    toHaveBoundingBox: common.toHaveBoundingBox,
  };
  return web;
}
