import grpc from "@grpc/grpc-js";

export { AllwrightError } from "./errors.js";

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

export interface ScreenshotResult {
  pngData: Uint8Array;
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
  screenshot(options?: CommandOptions): Promise<ScreenshotResult>;
  close(): Promise<void>;
  ping(message?: string): Promise<string>;
  pageInfo(): PageInfo;
}

export interface MobileAndroidConnectOptions {
  device?: string;
  adbEndpoint?: string;
  preserveAppState?: boolean;
  timeoutMs?: number;
}

export interface MobileAndroidLaunchOptions {
  apkPath?: string;
  appId?: string;
  launchActivity?: string;
  stopBeforeLaunch?: boolean;
  timeoutMs?: number;
}

export interface MobileAndroidLocator {
  readonly page: MobileAndroidApp;
  readonly selector: string;
  click(options?: CommandOptions): Promise<ClickResult>;
  fill(value: string, options?: CommandOptions): Promise<FillResult>;
  locator(selector: string): MobileAndroidLocator;
}

export interface MobileAndroidApp {
  readonly sessionId: string;
  locator(selector: string): MobileAndroidLocator;
  click(selector: string, options?: CommandOptions): Promise<ClickResult>;
  fill(selector: string, value: string, options?: CommandOptions): Promise<FillResult>;
  screenshot(options?: CommandOptions): Promise<ScreenshotResult>;
}

export interface MobileAndroidDevice {
  readonly sessionId: string;
  app(): MobileAndroidApp;
  initialApp(): MobileAndroidApp;
  launch(options?: MobileAndroidLaunchOptions): Promise<MobileAndroidApp>;
}

export interface MobileSurfaceNamespace {
  android: {
    connect(options?: MobileAndroidConnectOptions): Promise<MobileAndroidDevice>;
  };
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
  web?: WebConfig;
  mobile?: MobileConfig;
  desktop?: DesktopConfig;
  expect?: RetryConfig;
  suites?: Record<string, AllwrightSuiteConfig>;
}

export interface AllwrightSuiteConfig {
  server?: {
    addr?: string;
  };
  web?: WebConfig;
  mobile?: MobileConfig;
  desktop?: DesktopConfig;
  expect?: RetryConfig;
}

export interface WebConfig {
  browser?: {
    name?: BrowserKind;
    binary?: string;
    launchOptions?: LaunchOptions;
  };
}

export interface SurfaceAppConfig {
  id?: string;
  binary?: string;
  activity?: string;
}

export interface SurfaceMobileTargetConfig {
  device?: string;
  app?: SurfaceAppConfig;
}

export interface SurfaceDesktopTargetConfig {
  app?: SurfaceAppConfig;
}

export interface MobileConfig {
  android?: SurfaceMobileTargetConfig;
  ios?: SurfaceMobileTargetConfig;
}

export interface DesktopConfig {
  mac?: SurfaceDesktopTargetConfig;
  windows?: SurfaceDesktopTargetConfig;
  linux?: SurfaceDesktopTargetConfig;
}

export interface RetryConfig {
  timeoutMs?: number;
  intervalMs?: number;
}

export interface ResolvedWebSurfaceConfig {
  browserName?: BrowserKind;
  browserBinary?: string;
  launchOptions: LaunchOptions;
}

export interface ResolvedMobileTargetConfig {
  device?: string;
  appId?: string;
  appBinary?: string;
  appActivity?: string;
}

export interface ResolvedDesktopTargetConfig {
  appId?: string;
  appBinary?: string;
  appActivity?: string;
}

export interface ResolvedAllwrightConfig {
  configFilePath: string | null;
  suiteName: string | null;
  serverAddr?: string;
  browserName?: BrowserKind;
  browserBinary?: string;
  launchOptions: LaunchOptions;
  expect: RetryConfig;
  web?: ResolvedWebSurfaceConfig;
  mobile: {
    android?: ResolvedMobileTargetConfig;
    ios?: ResolvedMobileTargetConfig;
  };
  desktop: {
    mac?: ResolvedDesktopTargetConfig;
    windows?: ResolvedDesktopTargetConfig;
    linux?: ResolvedDesktopTargetConfig;
  };
}

export interface ResolveConfigOptions {
  cwd?: string;
  configFile?: string;
  suite?: string;
}

export interface PingResponse {
  message?: string;
  version?: string;
}

export interface ChromeLaunchedPayload {
  browser?: string;
  note?: string;
  cdpWebsocketUrl?: string;
  userDataDir?: string;
  initialPageSessionId?: string;
}

export interface BrowserLaunchedPayload {
  browserKind?: number;
  browser?: string;
  note?: string;
  userDataDir?: string;
  initialPageSessionId?: string;
}

export interface SurfaceSessionEvent {
  sessionId?: string;
  event?: string;
  chromeLaunched?: ChromeLaunchedPayload;
  browserLaunched?: BrowserLaunchedPayload;
  mobileConnected?: {
    platform?: number | string;
    deviceName?: string;
    note?: string;
    deviceId?: string;
    connectionKind?: number | string;
    backend?: string;
    deviceSessionId?: string;
    initialAppSessionId?: string;
    packageName?: string;
    activityName?: string;
  };
  appLaunched?: {
    appSessionId?: string;
    note?: string;
    packageName?: string;
    activityName?: string;
    webviewContext?: string;
  };
  contextOpened?: {
    contextSessionId?: string;
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

export interface ContextSessionEvent {
  contextSessionId?: string;
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
  screenshotCaptured?: {
    pngData?: Uint8Array;
    note?: string;
  };
}

export interface LaunchChromeRequest {
  launchChrome: {
    chromeBinary?: string;
    retryOptions?: {
      timeoutMs?: number;
    };
  };
}

export interface LaunchBrowserRequest {
  launchBrowser: {
    browserKind: number;
    browserBinary?: string;
    retryOptions?: {
      timeoutMs?: number;
    };
  };
}

export interface OpenContextRequest {
  openContext: {
    retryOptions?: {
      timeoutMs?: number;
    };
  };
}

export interface ConnectMobileRequest {
  connectMobile: {
    platform: number;
    device?: string;
    adbEndpoint?: string;
    preserveAppState?: boolean;
    retryOptions?: {
      timeoutMs?: number;
    };
  };
}

export interface LaunchAppRequest {
  launchApp: {
    apkPath?: string;
    appId?: string;
    launchActivity?: string;
    stopBeforeLaunch?: boolean;
    retryOptions?: {
      timeoutMs?: number;
    };
  };
}

export interface SurfacePingRequest {
  ping: {
    message: string;
  };
}

export interface CloseSurfaceRequest {
  close: Record<string, never>;
}

export interface ContextPingRequest {
  surfaceSessionId: string;
  contextSessionId: string;
  ping: {
    message: string;
  };
}

export interface NavigateRequest {
  surfaceSessionId: string;
  contextSessionId: string;
  navigate: {
    url: string;
    retryOptions?: {
      timeoutMs?: number;
    };
  };
}

export interface ClickRequest {
  surfaceSessionId: string;
  contextSessionId: string;
  clickElement: {
    cssSelector: string;
    retryOptions?: {
      timeoutMs?: number;
    };
  };
}

export interface CountRequest {
  surfaceSessionId: string;
  contextSessionId: string;
  countElements: {
    cssSelector: string;
    retryOptions?: {
      timeoutMs?: number;
    };
  };
}

export interface HighlightRequest {
  surfaceSessionId: string;
  contextSessionId: string;
  highlightElements: {
    cssSelector: string;
    durationMs?: number;
    retryOptions?: {
      timeoutMs?: number;
    };
  };
}

export interface FocusRequest {
  surfaceSessionId: string;
  contextSessionId: string;
  focusElement: {
    cssSelector: string;
    retryOptions?: {
      timeoutMs?: number;
    };
  };
}

export interface FillRequest {
  surfaceSessionId: string;
  contextSessionId: string;
  fillElement: {
    cssSelector: string;
    value: string;
    retryOptions?: {
      timeoutMs?: number;
    };
  };
}

export interface HoverRequest {
  surfaceSessionId: string;
  contextSessionId: string;
  hoverElement: {
    cssSelector: string;
    retryOptions?: {
      timeoutMs?: number;
    };
  };
}

export interface PressRequest {
  surfaceSessionId: string;
  contextSessionId: string;
  pressKey: {
    cssSelector: string;
    key: string;
    text?: string;
    retryOptions?: {
      timeoutMs?: number;
    };
  };
}

export interface TextContentRequest {
  surfaceSessionId: string;
  contextSessionId: string;
  getTextContent: {
    cssSelector: string;
    retryOptions?: {
      timeoutMs?: number;
    };
  };
}

export interface InnerTextRequest {
  surfaceSessionId: string;
  contextSessionId: string;
  getInnerText: {
    cssSelector: string;
    retryOptions?: {
      timeoutMs?: number;
    };
  };
}

export interface WaitForSelectorRequest {
  surfaceSessionId: string;
  contextSessionId: string;
  waitForSelector: {
    cssSelector: string;
    visible?: boolean;
    retryOptions?: {
      timeoutMs?: number;
    };
  };
}

export interface ScreenshotRequest {
  surfaceSessionId: string;
  contextSessionId: string;
  screenshot: {
    retryOptions?: {
      timeoutMs?: number;
    };
  };
}

export interface CloseContextRequest {
  surfaceSessionId: string;
  contextSessionId: string;
  close: Record<string, never>;
}

export type SurfaceSessionRequest =
  | LaunchBrowserRequest
  | LaunchChromeRequest
  | OpenContextRequest
  | ConnectMobileRequest
  | LaunchAppRequest
  | SurfacePingRequest
  | CloseSurfaceRequest;

export type ContextSessionRequest =
  | ContextPingRequest
  | NavigateRequest
  | ClickRequest
  | CountRequest
  | HighlightRequest
  | FocusRequest
  | FillRequest
  | HoverRequest
  | PressRequest
  | TextContentRequest
  | InnerTextRequest
  | WaitForSelectorRequest
  | ScreenshotRequest
  | CloseContextRequest;

export type SurfaceSessionStream = grpc.ClientDuplexStream<SurfaceSessionRequest, SurfaceSessionEvent>;
export type ContextSessionStream = grpc.ClientDuplexStream<ContextSessionRequest, ContextSessionEvent>;

export interface EngineServiceClientShape {
  Ping(
    request: Record<string, never>,
    callback: (error: grpc.ServiceError | null, response: PingResponse) => void,
  ): void;
  SurfaceSession(): SurfaceSessionStream;
  ContextSession(): ContextSessionStream;
  close(): void;
}

export interface EngineProtoRoot {
  allwright: {
    engine: {
      v1: {
        EngineService: grpc.ServiceClientConstructor;
      };
    };
  };
}

export interface RuntimeClient {
  client: EngineServiceClientShape;
}

export interface BrowserLaunchState {
  runtime: RuntimeClient;
  stream: SurfaceSessionStream;
  queue: EventQueue<SurfaceSessionEvent>;
  sessionId: string;
  launched: BrowserLaunchedPayload;
}

export interface PageHandle {
  stream: ContextSessionStream;
  queue: EventQueue<ContextSessionEvent>;
  closed: boolean;
}

export class EventQueue<T> {
  #items: T[] = [];
  #waiters: Array<{
    resolve: (value: T) => void;
    reject: (error: Error) => void;
  }> = [];
  #endedError: Error | null = null;

  push(item: T): void {
    const waiter = this.#waiters.shift();
    if (waiter) {
      waiter.resolve(item);
      return;
    }
    this.#items.push(item);
  }

  fail(error: Error): void {
    if (this.#endedError) {
      return;
    }
    this.#endedError = error;
    while (this.#waiters.length > 0) {
      this.#waiters.shift()!.reject(error);
    }
  }

  async next(): Promise<T> {
    if (this.#items.length > 0) {
      return this.#items.shift() as T;
    }
    if (this.#endedError) {
      throw this.#endedError;
    }
    return new Promise<T>((resolve, reject) => {
      this.#waiters.push({ resolve, reject });
    });
  }
}
