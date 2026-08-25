import { Locator, Page, type Browser, type LaunchOptions, type WaitForSelectorOptions } from "@allwright.dev/core";
import { expect as vitestExpect } from "vitest";
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
export declare const test: import("vitest").TestAPI<{
    browser: Browser;
    page: Page;
    allwright: AllwrightVitestOptions;
}>;
type VitestExpect = typeof vitestExpect;
type VitestMatcherReturn = ReturnType<VitestExpect>;
interface AllwrightExpect extends VitestExpect {
    (actual: Page): PageExpectMatchers;
    (actual: Locator): LocatorExpectMatchers;
    <T>(actual: T): VitestMatcherReturn;
}
export declare const expect: AllwrightExpect;
export { vitestExpect };
export type { Browser, LaunchOptions, Locator, Page };
