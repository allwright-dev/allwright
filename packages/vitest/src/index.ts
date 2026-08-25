import {
  chromium,
  setServerAddr,
  shutdown,
  type Browser,
  type LaunchOptions,
  type Page,
} from "@allwright.dev/core";
import { expect, test as base } from "vitest";

export interface AllwrightVitestOptions {
  launchOptions?: LaunchOptions;
  serverAddr?: string;
}

export interface AllwrightVitestFixtures {
  browser: Browser;
  page: Page;
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

export { expect };

export type { Browser, LaunchOptions, Page };
