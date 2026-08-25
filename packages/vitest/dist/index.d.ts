import { type Browser, type LaunchOptions, type Page } from "@allwright.dev/core";
import { expect } from "vitest";
export interface AllwrightVitestOptions {
    launchOptions?: LaunchOptions;
    serverAddr?: string;
}
export interface AllwrightVitestFixtures {
    browser: Browser;
    page: Page;
}
export declare const test: import("vitest").TestAPI<{
    browser: Browser;
    page: Page;
    allwright: AllwrightVitestOptions;
}>;
export { expect };
export type { Browser, LaunchOptions, Page };
