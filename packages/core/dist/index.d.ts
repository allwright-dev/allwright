import grpc from "@grpc/grpc-js";
export interface LaunchOptions {
    chromeBinary?: string;
    timeoutMs?: number;
}
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
interface PingResponse {
    message?: string;
}
interface ChromeLaunchedPayload {
    browser?: string;
    note?: string;
    cdpWebsocketUrl?: string;
    userDataDir?: string;
    initialTabSessionId?: string;
}
interface BrowserSessionEvent {
    sessionId?: string;
    event?: string;
    chromeLaunched?: ChromeLaunchedPayload;
    tabOpened?: {
        tabSessionId?: string;
        note?: string;
    };
    pong?: {
        message?: string;
    };
    closed?: {
        reason?: string;
    };
    error?: {
        message?: string;
    };
}
interface TabSessionEvent {
    tabSessionId?: string;
    event?: string;
    attached?: {
        note?: string;
    };
    pong?: {
        message?: string;
    };
    closed?: {
        reason?: string;
    };
    error?: {
        message?: string;
    };
    navigated?: {
        url?: string;
        note?: string;
    };
    chromiumBidiInjection?: {
        note?: string;
        bidiSessionId?: string;
        mapperTargetId?: string;
        mapperSessionId?: string;
        packageVersion?: string;
    };
    elementClicked?: {
        cssSelector?: string;
        note?: string;
        bidiSessionId?: string;
    };
    elementCounted?: {
        cssSelector?: string;
        count?: number;
        note?: string;
    };
    elementsHighlighted?: {
        cssSelector?: string;
        count?: number;
        note?: string;
    };
    elementFocused?: {
        cssSelector?: string;
        note?: string;
    };
    elementFilled?: {
        cssSelector?: string;
        value?: string;
        note?: string;
    };
    elementHovered?: {
        cssSelector?: string;
        note?: string;
    };
    keyPressed?: {
        cssSelector?: string;
        key?: string;
        note?: string;
    };
    textContentResolved?: {
        cssSelector?: string;
        text?: string;
        note?: string;
    };
    innerTextResolved?: {
        cssSelector?: string;
        text?: string;
        note?: string;
    };
    selectorWaitSatisfied?: {
        cssSelector?: string;
        visible?: boolean;
        note?: string;
    };
}
interface LaunchChromeRequest {
    launchChrome: {
        chromeBinary?: string;
        retryOptions?: {
            timeoutMs?: number;
        };
    };
}
interface OpenTabRequest {
    openTab: {
        retryOptions?: {
            timeoutMs?: number;
        };
    };
}
interface BrowserPingRequest {
    ping: {
        message: string;
    };
}
interface CloseBrowserRequest {
    close: Record<string, never>;
}
interface TabPingRequest {
    browserSessionId: string;
    tabSessionId: string;
    ping: {
        message: string;
    };
}
interface NavigateRequest {
    browserSessionId: string;
    tabSessionId: string;
    navigate: {
        url: string;
        retryOptions?: {
            timeoutMs?: number;
        };
    };
}
interface ClickRequest {
    browserSessionId: string;
    tabSessionId: string;
    clickElement: {
        cssSelector: string;
        retryOptions?: {
            timeoutMs?: number;
        };
    };
}
interface CountRequest {
    browserSessionId: string;
    tabSessionId: string;
    countElements: {
        cssSelector: string;
        retryOptions?: {
            timeoutMs?: number;
        };
    };
}
interface HighlightRequest {
    browserSessionId: string;
    tabSessionId: string;
    highlightElements: {
        cssSelector: string;
        durationMs?: number;
        retryOptions?: {
            timeoutMs?: number;
        };
    };
}
interface FocusRequest {
    browserSessionId: string;
    tabSessionId: string;
    focusElement: {
        cssSelector: string;
        retryOptions?: {
            timeoutMs?: number;
        };
    };
}
interface FillRequest {
    browserSessionId: string;
    tabSessionId: string;
    fillElement: {
        cssSelector: string;
        value: string;
        retryOptions?: {
            timeoutMs?: number;
        };
    };
}
interface HoverRequest {
    browserSessionId: string;
    tabSessionId: string;
    hoverElement: {
        cssSelector: string;
        retryOptions?: {
            timeoutMs?: number;
        };
    };
}
interface PressRequest {
    browserSessionId: string;
    tabSessionId: string;
    pressKey: {
        cssSelector: string;
        key: string;
        text?: string;
        retryOptions?: {
            timeoutMs?: number;
        };
    };
}
interface TextContentRequest {
    browserSessionId: string;
    tabSessionId: string;
    getTextContent: {
        cssSelector: string;
        retryOptions?: {
            timeoutMs?: number;
        };
    };
}
interface InnerTextRequest {
    browserSessionId: string;
    tabSessionId: string;
    getInnerText: {
        cssSelector: string;
        retryOptions?: {
            timeoutMs?: number;
        };
    };
}
interface WaitForSelectorRequest {
    browserSessionId: string;
    tabSessionId: string;
    waitForSelector: {
        cssSelector: string;
        visible?: boolean;
        retryOptions?: {
            timeoutMs?: number;
        };
    };
}
interface CloseTabRequest {
    browserSessionId: string;
    tabSessionId: string;
    close: Record<string, never>;
}
type BrowserSessionRequest = LaunchChromeRequest | OpenTabRequest | BrowserPingRequest | CloseBrowserRequest;
type TabSessionRequest = TabPingRequest | NavigateRequest | ClickRequest | CountRequest | HighlightRequest | FocusRequest | FillRequest | HoverRequest | PressRequest | TextContentRequest | InnerTextRequest | WaitForSelectorRequest | CloseTabRequest;
type BrowserSessionStream = grpc.ClientDuplexStream<BrowserSessionRequest, BrowserSessionEvent>;
type PageSessionStream = grpc.ClientDuplexStream<TabSessionRequest, TabSessionEvent>;
interface EngineServiceClientShape {
    Ping(request: Record<string, never>, callback: (error: grpc.ServiceError | null, response: PingResponse) => void): void;
    BrowserSession(): BrowserSessionStream;
    TabSession(): PageSessionStream;
    close(): void;
}
interface RuntimeClient {
    client: EngineServiceClientShape;
}
interface BrowserLaunchState {
    runtime: RuntimeClient;
    stream: BrowserSessionStream;
    queue: EventQueue<BrowserSessionEvent>;
    sessionId: string;
    chromeLaunched: ChromeLaunchedPayload;
}
declare class EventQueue<T> {
    #private;
    push(item: T): void;
    fail(error: Error): void;
    next(): Promise<T>;
}
export declare class BrowserType {
    launch(options?: LaunchOptions): Promise<Browser>;
}
export declare class Browser {
    #private;
    constructor(state: BrowserLaunchState);
    readonly sessionId: string;
    readonly browserName: string;
    readonly launchNote: string;
    readonly cdpWebSocketURL: string;
    readonly userDataDir: string;
    page(): Page;
    initialPage(): Page;
    pages(): Page[];
    newPage(options?: CommandOptions): Promise<Page>;
    close(): Promise<void>;
    ping(message?: string): Promise<string>;
    browserInfo(): BrowserInfo;
    initialTab(): Page;
    newTab(options?: CommandOptions): Promise<Page>;
}
export declare class Page {
    #private;
    constructor(input: PageInfo & {
        runtime: RuntimeClient;
    });
    readonly sessionId: string;
    readonly browserSessionId: string;
    locator(selector: string): Locator;
    goto(url: string, options?: CommandOptions): Promise<NavigateResult>;
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
    navigate(url: string, options?: CommandOptions): Promise<NavigateResult>;
}
export declare class Locator {
    readonly page: Page;
    readonly selector: string;
    constructor(input: LocatorInfo);
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
export declare const chromium: BrowserType;
export type Tab = Page;
export declare function ping(): Promise<string>;
export declare function launchChrome(options?: LaunchOptions): Promise<Browser>;
export declare function setServerAddr(serverAddr: string): void;
export declare function shutdown(): Promise<void>;
export {};
