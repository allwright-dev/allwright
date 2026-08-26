export interface LaunchOptions {
    browserBinary?: string;
    timeoutMs?: number;
}
export type BrowserKind = "chromium" | "firefox";
export interface CommandOptions {
    timeoutMs?: number;
}
export interface NavigateResult {
    url: string;
    note: string;
    bidiSessionId: string;
    mapperTargetId: string;
    mapperSessionId: string;
    packageVersion: string;
}
export interface ClickResult {
    selector: string;
    note: string;
    bidiSessionId: string;
}
export interface CountResult {
    selector: string;
    count: number;
    note: string;
}
export interface HighlightOptions {
    timeoutMs?: number;
    durationMs?: number;
}
export interface HighlightResult {
    selector: string;
    count: number;
    note: string;
}
export interface ElementResult {
    selector: string;
    note: string;
}
export interface FillResult {
    selector: string;
    value: string;
    note: string;
}
export interface PressOptions {
    timeoutMs?: number;
    text?: string;
}
export interface PressResult {
    selector: string;
    key: string;
    note: string;
}
export interface TextResult {
    selector: string;
    text: string;
    note: string;
}
export interface WaitForSelectorOptions {
    timeoutMs?: number;
    visible?: boolean;
}
export interface WaitForSelectorResult {
    selector: string;
    visible: boolean;
    note: string;
}
export interface BrowserInfo {
    sessionId: string;
    browserName: string;
    launchNote: string;
    cdpWebSocketURL: string;
    userDataDir: string;
}
export interface PageInfo {
    sessionId: string;
    browserSessionId: string;
}
export interface LocatorInfo {
    page: Page;
    selector: string;
}
export interface BrowserType {
    launch(options?: LaunchOptions): Promise<Browser>;
}
export interface Browser extends BrowserInfo {
    page(): Page;
    initialPage(): Page;
    initialTab(): Page;
    pages(): Page[];
    newPage(options?: CommandOptions): Promise<Page>;
    newTab(options?: CommandOptions): Promise<Page>;
    close(): Promise<void>;
    ping(message?: string): Promise<string>;
    browserInfo(): BrowserInfo;
}
export interface Page extends PageInfo {
    locator(selector: string): Locator;
    goto(url: string, options?: CommandOptions): Promise<NavigateResult>;
    navigate(url: string, options?: CommandOptions): Promise<NavigateResult>;
    click(selector: string, options?: CommandOptions): Promise<ClickResult>;
    count(selector: string, options?: CommandOptions): Promise<CountResult>;
    highlight(selector: string, options?: HighlightOptions): Promise<HighlightResult>;
    focus(selector: string, options?: CommandOptions): Promise<ElementResult>;
    fill(selector: string, value: string, options?: CommandOptions): Promise<FillResult>;
    hover(selector: string, options?: CommandOptions): Promise<ElementResult>;
    press(selector: string, key: string, options?: PressOptions): Promise<PressResult>;
    textContent(selector: string, options?: CommandOptions): Promise<TextResult>;
    innerText(selector: string, options?: CommandOptions): Promise<TextResult>;
    waitForSelector(selector: string, options?: WaitForSelectorOptions): Promise<WaitForSelectorResult>;
    close(): Promise<void>;
    ping(message?: string): Promise<string>;
    pageInfo(): PageInfo;
}
export interface Locator {
    readonly page: Page;
    readonly selector: string;
    click(options?: CommandOptions): Promise<ClickResult>;
    count(options?: CommandOptions): Promise<CountResult>;
    highlight(options?: HighlightOptions): Promise<HighlightResult>;
    focus(options?: CommandOptions): Promise<ElementResult>;
    fill(value: string, options?: CommandOptions): Promise<FillResult>;
    hover(options?: CommandOptions): Promise<ElementResult>;
    press(key: string, options?: PressOptions): Promise<PressResult>;
    textContent(options?: CommandOptions): Promise<TextResult>;
    innerText(options?: CommandOptions): Promise<TextResult>;
    waitFor(options?: WaitForSelectorOptions): Promise<WaitForSelectorResult>;
    locator(selector: string): Locator;
}
export interface AllwrightConfig {
    schemaVersion?: 1;
    server?: {
        addr?: string;
    };
    browser?: {
        name?: BrowserKind;
        binary?: string;
        launchOptions?: LaunchOptions;
    };
    expect?: RetryConfig;
    suites?: Record<string, AllwrightSuiteConfig>;
}
export interface AllwrightSuiteConfig {
    server?: {
        addr?: string;
    };
    browser?: {
        name?: BrowserKind;
        binary?: string;
        launchOptions?: LaunchOptions;
    };
    expect?: RetryConfig;
}
export interface RetryConfig {
    timeoutMs?: number;
    intervalMs?: number;
}
export interface ResolvedAllwrightConfig {
    configFilePath: string | null;
    suiteName: string | null;
    serverAddr?: string;
    browserName: BrowserKind;
    browserBinary?: string;
    launchOptions: LaunchOptions;
    expect: RetryConfig;
}
export interface ResolveConfigOptions {
    cwd?: string;
    configFile?: string;
    suite?: string;
}
export declare const chromium: BrowserType;
export declare const firefox: BrowserType;
export type Tab = Page;
export declare function ping(): Promise<string>;
export declare function launchChrome(options?: LaunchOptions): Promise<Browser>;
export declare function launchConfiguredBrowser(config: ResolvedAllwrightConfig): Promise<Browser>;
export declare function launchBrowser(browserKind: BrowserKind, options?: LaunchOptions): Promise<Browser>;
export declare function setServerAddr(serverAddr: string): void;
export declare function shutdown(): Promise<void>;
export declare function findConfigFile(startDir?: string): string | null;
export declare function loadConfigFile(configFile: string): AllwrightConfig;
export declare function resolveConfig(options?: ResolveConfigOptions): ResolvedAllwrightConfig;
