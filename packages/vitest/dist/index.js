import { launchConfiguredBrowser, resolveConfig, setServerAddr, shutdown, } from "@allwright.dev/core";
import { expect as vitestExpect, test as base } from "vitest";
export const test = base.extend({
    allwright: async ({}, use) => {
        await use({});
    },
    browser: async ({ allwright }, use) => {
        const config = resolveVitestConfig(allwright);
        if (config.serverAddr) {
            setServerAddr(config.serverAddr);
        }
        activeExpectDefaults = config.expect;
        const browser = await launchConfiguredBrowser(config);
        try {
            await use(browser);
        }
        finally {
            activeExpectDefaults = {};
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
function isPage(value) {
    return !!value && typeof value === "object" && typeof value.locator === "function";
}
function isLocator(value) {
    return (!!value &&
        typeof value === "object" &&
        typeof value.page === "object" &&
        typeof value.click === "function");
}
function resolveVitestConfig(options) {
    const config = resolveConfig({
        configFile: options.configFile,
        suite: options.suite,
    });
    return {
        ...config,
        serverAddr: options.serverAddr ?? config.serverAddr,
        browserName: options.browser ?? config.browserName,
        browserBinary: options.browserBinary ?? config.browserBinary,
        launchOptions: {
            ...config.launchOptions,
            ...(options.launchOptions ?? {}),
            browserBinary: options.launchOptions?.browserBinary ??
                options.browserBinary ??
                config.browserBinary ??
                config.launchOptions.browserBinary,
        },
        expect: config.expect,
    };
}
async function retryExpectation(callback, options = {}, defaults = {}) {
    const timeoutMs = options.timeoutMs ?? defaults.timeoutMs ?? DEFAULT_EXPECT_TIMEOUT_MS;
    const intervalMs = options.intervalMs ?? defaults.intervalMs ?? DEFAULT_EXPECT_INTERVAL_MS;
    const deadline = Date.now() + timeoutMs;
    let lastError;
    while (Date.now() <= deadline) {
        try {
            await callback();
            return;
        }
        catch (error) {
            lastError = error;
            if (Date.now() + intervalMs > deadline) {
                break;
            }
            await new Promise((resolve) => setTimeout(resolve, intervalMs));
        }
    }
    throw lastError instanceof Error ? lastError : new Error(String(lastError));
}
function createPageExpect(page) {
    return {
        async toHaveText(selector, expected, options = {}) {
            await retryExpectation(async () => {
                const result = await page.textContent(selector, options.command ?? {});
                if (expected instanceof RegExp) {
                    vitestExpect(result.text).toMatch(expected);
                    return;
                }
                vitestExpect(result.text).toBe(expected);
            }, options, currentExpectDefaults());
        },
        async toContainText(selector, expected, options = {}) {
            await retryExpectation(async () => {
                const result = await page.textContent(selector, options.command ?? {});
                if (expected instanceof RegExp) {
                    vitestExpect(result.text).toMatch(expected);
                    return;
                }
                vitestExpect(result.text).toContain(expected);
            }, options, currentExpectDefaults());
        },
        async toHaveCount(selector, expected, options = {}) {
            await retryExpectation(async () => {
                const result = await page.count(selector, {});
                vitestExpect(result.count).toBe(expected);
            }, options, currentExpectDefaults());
        },
        async toBeVisible(selector, options = {}) {
            await retryExpectation(async () => {
                const result = await page.waitForSelector(selector, {
                    visible: true,
                    ...(options.command ?? {}),
                });
                vitestExpect(result.visible).toBe(true);
            }, options, currentExpectDefaults());
        },
    };
}
function createLocatorExpect(locator) {
    return {
        async toHaveText(expected, options = {}) {
            await retryExpectation(async () => {
                const result = await locator.textContent(options.command ?? {});
                if (expected instanceof RegExp) {
                    vitestExpect(result.text).toMatch(expected);
                    return;
                }
                vitestExpect(result.text).toBe(expected);
            }, options, currentExpectDefaults());
        },
        async toContainText(expected, options = {}) {
            await retryExpectation(async () => {
                const result = await locator.textContent(options.command ?? {});
                if (expected instanceof RegExp) {
                    vitestExpect(result.text).toMatch(expected);
                    return;
                }
                vitestExpect(result.text).toContain(expected);
            }, options, currentExpectDefaults());
        },
        async toHaveCount(expected, options = {}) {
            await retryExpectation(async () => {
                const result = await locator.count({});
                vitestExpect(result.count).toBe(expected);
            }, options, currentExpectDefaults());
        },
        async toBeVisible(options = {}) {
            await retryExpectation(async () => {
                const result = await locator.waitFor({
                    visible: true,
                    ...(options.command ?? {}),
                });
                vitestExpect(result.visible).toBe(true);
            }, options, currentExpectDefaults());
        },
    };
}
let activeExpectDefaults = {};
function currentExpectDefaults() {
    return activeExpectDefaults;
}
const expectImpl = ((actual) => {
    if (isLocator(actual)) {
        return createLocatorExpect(actual);
    }
    if (isPage(actual)) {
        return createPageExpect(actual);
    }
    return vitestExpect(actual);
});
const vitestExpectDescriptors = Object.getOwnPropertyDescriptors(vitestExpect);
for (const [key, descriptor] of Object.entries(vitestExpectDescriptors)) {
    Object.defineProperty(expectImpl, key, descriptor);
}
export const expect = expectImpl;
export { vitestExpect };
