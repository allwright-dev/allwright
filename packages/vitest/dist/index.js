import { chromium, setServerAddr, shutdown, } from "@allwright.dev/core";
import { expect, test as base } from "vitest";
export const test = base.extend({
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
        }
        finally {
            await browser.close().catch(() => undefined);
            await shutdown();
        }
    },
    page: async ({ browser }, use) => {
        await use(browser.page());
    },
});
export { expect };
