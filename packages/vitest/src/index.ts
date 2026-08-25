import {
  chromium,
  setServerAddr,
  shutdown,
  Locator,
  Page,
  type Browser,
  type LaunchOptions,
  type WaitForSelectorOptions,
} from "@allwright.dev/core";
import { expect as vitestExpect, test as base } from "vitest";

export interface AllwrightVitestOptions {
  launchOptions?: LaunchOptions;
  serverAddr?: string;
}

export interface AllwrightVitestFixtures {
  browser: Browser;
  page: Page;
}

export interface RetryExpectationOptions {
  timeoutMs?: number;
  intervalMs?: number;
}

export interface TextExpectationOptions extends RetryExpectationOptions {
  command?: {
    timeoutMs?: number;
  };
}

export interface VisibleExpectationOptions extends RetryExpectationOptions {
  command?: WaitForSelectorOptions;
}

export interface PageExpectMatchers {
  toHaveText(selector: string, expected: string | RegExp, options?: TextExpectationOptions): Promise<void>;
  toContainText(selector: string, expected: string | RegExp, options?: TextExpectationOptions): Promise<void>;
  toHaveCount(selector: string, expected: number, options?: RetryExpectationOptions): Promise<void>;
  toBeVisible(selector: string, options?: VisibleExpectationOptions): Promise<void>;
}

export interface LocatorExpectMatchers {
  toHaveText(expected: string | RegExp, options?: TextExpectationOptions): Promise<void>;
  toContainText(expected: string | RegExp, options?: TextExpectationOptions): Promise<void>;
  toHaveCount(expected: number, options?: RetryExpectationOptions): Promise<void>;
  toBeVisible(options?: VisibleExpectationOptions): Promise<void>;
}

type AllwrightVitestContext = AllwrightVitestFixtures & {
  allwright: AllwrightVitestOptions;
};

export const test = base.extend<AllwrightVitestContext>({
  allwright: async ({}, use) => {
    await use({});
  },

  browser: async ({ allwright }, use) => {
    if (allwright.serverAddr) {
      setServerAddr(allwright.serverAddr);
    }

    const browser = await chromium.launch(allwright.launchOptions ?? {});

    try {
      await use(browser);
    } finally {
      await browser.close().catch(() => undefined);
      await shutdown();
    }
  },

  page: async ({ browser }, use) => {
    await use(browser.page());
  },
});

const DEFAULT_EXPECT_TIMEOUT_MS = 5_000;
const DEFAULT_EXPECT_INTERVAL_MS = 100;

function isPage(value: unknown): value is Page {
  return value instanceof Page;
}

function isLocator(value: unknown): value is Locator {
  return value instanceof Locator;
}

async function retryExpectation(
  callback: () => Promise<void>,
  options: RetryExpectationOptions = {},
): Promise<void> {
  const timeoutMs = options.timeoutMs ?? DEFAULT_EXPECT_TIMEOUT_MS;
  const intervalMs = options.intervalMs ?? DEFAULT_EXPECT_INTERVAL_MS;
  const deadline = Date.now() + timeoutMs;
  let lastError: unknown;

  while (Date.now() <= deadline) {
    try {
      await callback();
      return;
    } catch (error) {
      lastError = error;
      if (Date.now() + intervalMs > deadline) {
        break;
      }
      await new Promise((resolve) => setTimeout(resolve, intervalMs));
    }
  }

  throw lastError instanceof Error ? lastError : new Error(String(lastError));
}

function createPageExpect(page: Page): PageExpectMatchers {
  return {
    async toHaveText(selector, expected, options = {}) {
      await retryExpectation(async () => {
        const result = await page.textContent(selector, options.command ?? {});
        if (expected instanceof RegExp) {
          vitestExpect(result.text).toMatch(expected);
          return;
        }
        vitestExpect(result.text).toBe(expected);
      }, options);
    },

    async toContainText(selector, expected, options = {}) {
      await retryExpectation(async () => {
        const result = await page.textContent(selector, options.command ?? {});
        if (expected instanceof RegExp) {
          vitestExpect(result.text).toMatch(expected);
          return;
        }
        vitestExpect(result.text).toContain(expected);
      }, options);
    },

    async toHaveCount(selector, expected, options = {}) {
      await retryExpectation(async () => {
        const result = await page.count(selector, {});
        vitestExpect(result.count).toBe(expected);
      }, options);
    },

    async toBeVisible(selector, options = {}) {
      await retryExpectation(async () => {
        const result = await page.waitForSelector(selector, {
          visible: true,
          ...(options.command ?? {}),
        });
        vitestExpect(result.visible).toBe(true);
      }, options);
    },
  };
}

function createLocatorExpect(locator: Locator): LocatorExpectMatchers {
  return {
    async toHaveText(expected, options = {}) {
      await retryExpectation(async () => {
        const result = await locator.textContent(options.command ?? {});
        if (expected instanceof RegExp) {
          vitestExpect(result.text).toMatch(expected);
          return;
        }
        vitestExpect(result.text).toBe(expected);
      }, options);
    },

    async toContainText(expected, options = {}) {
      await retryExpectation(async () => {
        const result = await locator.textContent(options.command ?? {});
        if (expected instanceof RegExp) {
          vitestExpect(result.text).toMatch(expected);
          return;
        }
        vitestExpect(result.text).toContain(expected);
      }, options);
    },

    async toHaveCount(expected, options = {}) {
      await retryExpectation(async () => {
        const result = await locator.count({});
        vitestExpect(result.count).toBe(expected);
      }, options);
    },

    async toBeVisible(options = {}) {
      await retryExpectation(async () => {
        const result = await locator.waitFor({
          visible: true,
          ...(options.command ?? {}),
        });
        vitestExpect(result.visible).toBe(true);
      }, options);
    },
  };
}

type VitestExpect = typeof vitestExpect;
type VitestMatcherReturn = ReturnType<VitestExpect>;

interface AllwrightExpect extends VitestExpect {
  (actual: Page): PageExpectMatchers;
  (actual: Locator): LocatorExpectMatchers;
  <T>(actual: T): VitestMatcherReturn;
}

const expectImpl = ((actual: unknown) => {
  if (isLocator(actual)) {
    return createLocatorExpect(actual);
  }
  if (isPage(actual)) {
    return createPageExpect(actual);
  }
  return vitestExpect(actual);
}) as AllwrightExpect;

const vitestExpectDescriptors = Object.getOwnPropertyDescriptors(vitestExpect);
for (const [key, descriptor] of Object.entries(vitestExpectDescriptors)) {
  Object.defineProperty(expectImpl, key, descriptor);
}

export const expect = expectImpl;
export { vitestExpect };

export type { Browser, LaunchOptions, Locator, Page };
