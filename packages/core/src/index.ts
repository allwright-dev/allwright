import path from "node:path";
import { fileURLToPath } from "node:url";

import grpc from "@grpc/grpc-js";
import protoLoader from "@grpc/proto-loader";

const DEFAULT_SERVER_ADDR = "127.0.0.1:50051";
const SERVER_ADDR_ENV_VAR = "ALLWRIGHT_SERVER_ADDR";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const PACKAGE_ROOT = path.resolve(__dirname, "..");
const PROTO_ROOT = path.join(PACKAGE_ROOT, "proto");
const ENGINE_PROTO_PATH = path.join(PROTO_ROOT, "engine", "v1", "engine.proto");

let runtimePromise: Promise<RuntimeClient> | null = null;
let serverAddrOverride: string | null = null;

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

type BrowserSessionRequest =
  | LaunchChromeRequest
  | OpenTabRequest
  | BrowserPingRequest
  | CloseBrowserRequest;

type TabSessionRequest =
  | TabPingRequest
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
  | CloseTabRequest;

type BrowserSessionStream = grpc.ClientDuplexStream<BrowserSessionRequest, BrowserSessionEvent>;
type PageSessionStream = grpc.ClientDuplexStream<TabSessionRequest, TabSessionEvent>;

interface EngineServiceClientShape {
  Ping(
    request: Record<string, never>,
    callback: (error: grpc.ServiceError | null, response: PingResponse) => void,
  ): void;
  BrowserSession(): BrowserSessionStream;
  TabSession(): PageSessionStream;
  close(): void;
}

interface EngineProtoRoot {
  allwright: {
    engine: {
      v1: {
        EngineService: grpc.ServiceClientConstructor;
      };
    };
  };
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

interface PageHandle {
  stream: PageSessionStream;
  queue: EventQueue<TabSessionEvent>;
  closed: boolean;
}

class EventQueue<T> {
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

export class BrowserType {
  async launch(options: LaunchOptions = {}): Promise<Browser> {
    return launchChrome(options);
  }
}

export class Browser {
  #closed = false;
  #runtime: RuntimeClient;
  #stream: BrowserSessionStream;
  #queue: EventQueue<BrowserSessionEvent>;
  #pages = new Map<string, Page>();
  #initialPage: Page;

  constructor(state: BrowserLaunchState) {
    const browserInfo: BrowserInfo = {
      sessionId: state.sessionId,
      browserName: state.chromeLaunched.browser ?? "",
      launchNote: state.chromeLaunched.note ?? "",
      cdpWebSocketURL: state.chromeLaunched.cdpWebsocketUrl ?? "",
      userDataDir: state.chromeLaunched.userDataDir ?? "",
    };

    this.#runtime = state.runtime;
    this.#stream = state.stream;
    this.#queue = state.queue;
    this.sessionId = browserInfo.sessionId;
    this.browserName = browserInfo.browserName;
    this.launchNote = browserInfo.launchNote;
    this.cdpWebSocketURL = browserInfo.cdpWebSocketURL;
    this.userDataDir = browserInfo.userDataDir;

    this.#initialPage = this.#createPage(state.chromeLaunched.initialTabSessionId ?? "");
  }

  readonly sessionId: string;
  readonly browserName: string;
  readonly launchNote: string;
  readonly cdpWebSocketURL: string;
  readonly userDataDir: string;

  page(): Page {
    return this.#initialPage;
  }

  initialPage(): Page {
    return this.#initialPage;
  }

  pages(): Page[] {
    return [...this.#pages.values()];
  }

  async newPage(options: CommandOptions = {}): Promise<Page> {
    this.#ensureOpen();
    this.#stream.write({
      openTab: {
        retryOptions: options.timeoutMs ? { timeoutMs: options.timeoutMs } : undefined,
      },
    });

    while (true) {
      const event = await this.#queue.next();
      if (event.tabOpened?.tabSessionId) {
        return this.#createPage(event.tabOpened.tabSessionId);
      }
      if (event.error?.message) {
        throw new Error(`browser session error while opening tab: ${event.error.message}`);
      }
    }
  }

  async close(): Promise<void> {
    if (this.#closed) {
      return;
    }

    this.#stream.write({
      close: {},
    });

    while (true) {
      const event = await this.#queue.next();
      if (event.closed) {
        this.#closed = true;
        this.#stream.end();
        return;
      }
      if (event.error?.message) {
        throw new Error(`browser session error while closing: ${event.error.message}`);
      }
    }
  }

  async ping(message = "ping"): Promise<string> {
    this.#ensureOpen();
    this.#stream.write({
      ping: {
        message,
      },
    });

    while (true) {
      const event = await this.#queue.next();
      if (event.pong?.message) {
        return event.pong.message;
      }
      if (event.error?.message) {
        throw new Error(`browser session error while pinging: ${event.error.message}`);
      }
    }
  }

  browserInfo(): BrowserInfo {
    return {
      sessionId: this.sessionId,
      browserName: this.browserName,
      launchNote: this.launchNote,
      cdpWebSocketURL: this.cdpWebSocketURL,
      userDataDir: this.userDataDir,
    };
  }

  initialTab(): Page {
    return this.initialPage();
  }

  async newTab(options: CommandOptions = {}): Promise<Page> {
    return this.newPage(options);
  }

  #createPage(sessionId: string): Page {
    const existing = this.#pages.get(sessionId);
    if (existing) {
      return existing;
    }
    const page = new Page({
      runtime: this.#runtime,
      browserSessionId: this.sessionId,
      sessionId,
    });
    this.#pages.set(sessionId, page);
    return page;
  }

  #ensureOpen(): void {
    if (this.#closed) {
      throw new Error(`browser session ${this.sessionId} is closed`);
    }
  }
}

export class Page {
  #runtime: RuntimeClient;
  #handlePromise: Promise<PageHandle> | null = null;

  constructor(input: PageInfo & { runtime: RuntimeClient }) {
    this.#runtime = input.runtime;
    this.sessionId = input.sessionId;
    this.browserSessionId = input.browserSessionId;
  }

  readonly sessionId: string;
  readonly browserSessionId: string;

  locator(selector: string): Locator {
    return new Locator({ page: this, selector });
  }

  async goto(url: string, options: CommandOptions = {}): Promise<NavigateResult> {
    const handle = await this.#getHandle();
    this.#ensureOpen(handle);
    handle.stream.write({
      browserSessionId: this.browserSessionId,
      tabSessionId: this.sessionId,
      navigate: {
        url,
        retryOptions: options.timeoutMs ? { timeoutMs: options.timeoutMs } : undefined,
      },
    });

    let navigated: TabSessionEvent["navigated"] | null = null;
    let injection: TabSessionEvent["chromiumBidiInjection"] | null = null;

    while (true) {
      const event = await handle.queue.next();
      if (event.navigated) {
        navigated = event.navigated;
      }
      if (event.chromiumBidiInjection) {
        injection = event.chromiumBidiInjection;
      }
      if (event.error?.message) {
        throw new Error(`page session error while navigating: ${event.error.message}`);
      }
      if (event.closed) {
        handle.closed = true;
        throw new Error(`page session ${this.sessionId} closed while navigating`);
      }

      if (navigated && injection) {
        return {
          url: navigated.url ?? "",
          note: navigated.note ?? "",
          bidiSessionId: injection.bidiSessionId ?? "",
          mapperTargetId: injection.mapperTargetId ?? "",
          mapperSessionId: injection.mapperSessionId ?? "",
          packageVersion: injection.packageVersion ?? "",
        };
      }
    }
  }

  async click(selector: string, options: CommandOptions = {}): Promise<ClickResult> {
    const handle = await this.#getHandle();
    this.#ensureOpen(handle);
    handle.stream.write({
      browserSessionId: this.browserSessionId,
      tabSessionId: this.sessionId,
      clickElement: {
        cssSelector: selector,
        retryOptions: options.timeoutMs ? { timeoutMs: options.timeoutMs } : undefined,
      },
    });

    while (true) {
      const event = await handle.queue.next();
      if (event.elementClicked) {
        return {
          selector: event.elementClicked.cssSelector ?? "",
          note: event.elementClicked.note ?? "",
          bidiSessionId: event.elementClicked.bidiSessionId ?? "",
        };
      }
      if (event.error?.message) {
        throw new Error(`page session error while clicking: ${event.error.message}`);
      }
      if (event.closed) {
        handle.closed = true;
        throw new Error(`page session ${this.sessionId} closed while waiting for click result`);
      }
    }
  }

  async count(selector: string, options: CommandOptions = {}): Promise<CountResult> {
    const handle = await this.#getHandle();
    this.#ensureOpen(handle);
    handle.stream.write({
      browserSessionId: this.browserSessionId,
      tabSessionId: this.sessionId,
      countElements: {
        cssSelector: selector,
        retryOptions: options.timeoutMs ? { timeoutMs: options.timeoutMs } : undefined,
      },
    });

    while (true) {
      const event = await handle.queue.next();
      if (event.elementCounted) {
        return {
          selector: event.elementCounted.cssSelector ?? "",
          count: event.elementCounted.count ?? 0,
          note: event.elementCounted.note ?? "",
        };
      }
      if (event.error?.message) {
        throw new Error(`page session error while counting elements: ${event.error.message}`);
      }
      if (event.closed) {
        handle.closed = true;
        throw new Error(`page session ${this.sessionId} closed while waiting for count result`);
      }
    }
  }

  async highlight(selector: string, options: HighlightOptions = {}): Promise<HighlightResult> {
    const handle = await this.#getHandle();
    this.#ensureOpen(handle);
    handle.stream.write({
      browserSessionId: this.browserSessionId,
      tabSessionId: this.sessionId,
      highlightElements: {
        cssSelector: selector,
        durationMs: options.durationMs,
        retryOptions: options.timeoutMs ? { timeoutMs: options.timeoutMs } : undefined,
      },
    });

    while (true) {
      const event = await handle.queue.next();
      if (event.elementsHighlighted) {
        return {
          selector: event.elementsHighlighted.cssSelector ?? "",
          count: event.elementsHighlighted.count ?? 0,
          note: event.elementsHighlighted.note ?? "",
        };
      }
      if (event.error?.message) {
        throw new Error(`page session error while highlighting elements: ${event.error.message}`);
      }
      if (event.closed) {
        handle.closed = true;
        throw new Error(`page session ${this.sessionId} closed while waiting for highlight result`);
      }
    }
  }

  async focus(selector: string, options: CommandOptions = {}): Promise<ElementResult> {
    const handle = await this.#getHandle();
    this.#ensureOpen(handle);
    handle.stream.write({
      browserSessionId: this.browserSessionId,
      tabSessionId: this.sessionId,
      focusElement: {
        cssSelector: selector,
        retryOptions: options.timeoutMs ? { timeoutMs: options.timeoutMs } : undefined,
      },
    });

    while (true) {
      const event = await handle.queue.next();
      if (event.elementFocused) {
        return {
          selector: event.elementFocused.cssSelector ?? "",
          note: event.elementFocused.note ?? "",
        };
      }
      if (event.error?.message) {
        throw new Error(`page session error while focusing: ${event.error.message}`);
      }
      if (event.closed) {
        handle.closed = true;
        throw new Error(`page session ${this.sessionId} closed while waiting for focus result`);
      }
    }
  }

  async fill(selector: string, value: string, options: CommandOptions = {}): Promise<FillResult> {
    const handle = await this.#getHandle();
    this.#ensureOpen(handle);
    handle.stream.write({
      browserSessionId: this.browserSessionId,
      tabSessionId: this.sessionId,
      fillElement: {
        cssSelector: selector,
        value,
        retryOptions: options.timeoutMs ? { timeoutMs: options.timeoutMs } : undefined,
      },
    });

    while (true) {
      const event = await handle.queue.next();
      if (event.elementFilled) {
        return {
          selector: event.elementFilled.cssSelector ?? "",
          value: event.elementFilled.value ?? "",
          note: event.elementFilled.note ?? "",
        };
      }
      if (event.error?.message) {
        throw new Error(`page session error while filling: ${event.error.message}`);
      }
      if (event.closed) {
        handle.closed = true;
        throw new Error(`page session ${this.sessionId} closed while waiting for fill result`);
      }
    }
  }

  async hover(selector: string, options: CommandOptions = {}): Promise<ElementResult> {
    const handle = await this.#getHandle();
    this.#ensureOpen(handle);
    handle.stream.write({
      browserSessionId: this.browserSessionId,
      tabSessionId: this.sessionId,
      hoverElement: {
        cssSelector: selector,
        retryOptions: options.timeoutMs ? { timeoutMs: options.timeoutMs } : undefined,
      },
    });

    while (true) {
      const event = await handle.queue.next();
      if (event.elementHovered) {
        return {
          selector: event.elementHovered.cssSelector ?? "",
          note: event.elementHovered.note ?? "",
        };
      }
      if (event.error?.message) {
        throw new Error(`page session error while hovering: ${event.error.message}`);
      }
      if (event.closed) {
        handle.closed = true;
        throw new Error(`page session ${this.sessionId} closed while waiting for hover result`);
      }
    }
  }

  async press(selector: string, key: string, options: PressOptions = {}): Promise<PressResult> {
    const handle = await this.#getHandle();
    this.#ensureOpen(handle);
    handle.stream.write({
      browserSessionId: this.browserSessionId,
      tabSessionId: this.sessionId,
      pressKey: {
        cssSelector: selector,
        key,
        text: options.text,
        retryOptions: options.timeoutMs ? { timeoutMs: options.timeoutMs } : undefined,
      },
    });

    while (true) {
      const event = await handle.queue.next();
      if (event.keyPressed) {
        return {
          selector: event.keyPressed.cssSelector ?? "",
          key: event.keyPressed.key ?? "",
          note: event.keyPressed.note ?? "",
        };
      }
      if (event.error?.message) {
        throw new Error(`page session error while pressing key: ${event.error.message}`);
      }
      if (event.closed) {
        handle.closed = true;
        throw new Error(`page session ${this.sessionId} closed while waiting for press result`);
      }
    }
  }

  async textContent(selector: string, options: CommandOptions = {}): Promise<TextResult> {
    return this.#readText(selector, options, true);
  }

  async innerText(selector: string, options: CommandOptions = {}): Promise<TextResult> {
    return this.#readText(selector, options, false);
  }

  async waitForSelector(
    selector: string,
    options: WaitForSelectorOptions = {},
  ): Promise<WaitForSelectorResult> {
    const handle = await this.#getHandle();
    this.#ensureOpen(handle);
    handle.stream.write({
      browserSessionId: this.browserSessionId,
      tabSessionId: this.sessionId,
      waitForSelector: {
        cssSelector: selector,
        visible: options.visible,
        retryOptions: options.timeoutMs ? { timeoutMs: options.timeoutMs } : undefined,
      },
    });

    while (true) {
      const event = await handle.queue.next();
      if (event.selectorWaitSatisfied) {
        return {
          selector: event.selectorWaitSatisfied.cssSelector ?? "",
          visible: event.selectorWaitSatisfied.visible ?? false,
          note: event.selectorWaitSatisfied.note ?? "",
        };
      }
      if (event.error?.message) {
        throw new Error(`page session error while waiting for selector: ${event.error.message}`);
      }
      if (event.closed) {
        handle.closed = true;
        throw new Error(`page session ${this.sessionId} closed while waiting for selector result`);
      }
    }
  }

  async close(): Promise<void> {
    const handle = await this.#getHandle();
    if (handle.closed) {
      return;
    }

    handle.stream.write({
      browserSessionId: this.browserSessionId,
      tabSessionId: this.sessionId,
      close: {},
    });

    while (true) {
      const event = await handle.queue.next();
      if (event.closed) {
        handle.closed = true;
        handle.stream.end();
        return;
      }
      if (event.error?.message) {
        throw new Error(`page session error while closing: ${event.error.message}`);
      }
    }
  }

  async ping(message = "ping"): Promise<string> {
    const handle = await this.#getHandle();
    this.#ensureOpen(handle);
    handle.stream.write({
      browserSessionId: this.browserSessionId,
      tabSessionId: this.sessionId,
      ping: {
        message,
      },
    });

    while (true) {
      const event = await handle.queue.next();
      if (event.pong?.message) {
        return event.pong.message;
      }
      if (event.error?.message) {
        throw new Error(`page session error while pinging: ${event.error.message}`);
      }
      if (event.closed) {
        handle.closed = true;
        throw new Error(`page session ${this.sessionId} closed while waiting for pong`);
      }
    }
  }

  pageInfo(): PageInfo {
    return {
      sessionId: this.sessionId,
      browserSessionId: this.browserSessionId,
    };
  }

  async navigate(url: string, options: CommandOptions = {}): Promise<NavigateResult> {
    return this.goto(url, options);
  }

  async #readText(
    selector: string,
    options: CommandOptions,
    textContent: boolean,
  ): Promise<TextResult> {
    const handle = await this.#getHandle();
    this.#ensureOpen(handle);
    handle.stream.write(
      textContent
        ? {
            browserSessionId: this.browserSessionId,
            tabSessionId: this.sessionId,
            getTextContent: {
              cssSelector: selector,
              retryOptions: options.timeoutMs ? { timeoutMs: options.timeoutMs } : undefined,
            },
          }
        : {
            browserSessionId: this.browserSessionId,
            tabSessionId: this.sessionId,
            getInnerText: {
              cssSelector: selector,
              retryOptions: options.timeoutMs ? { timeoutMs: options.timeoutMs } : undefined,
            },
          },
    );

    while (true) {
      const event = await handle.queue.next();
      if (event.textContentResolved) {
        return {
          selector: event.textContentResolved.cssSelector ?? "",
          text: event.textContentResolved.text ?? "",
          note: event.textContentResolved.note ?? "",
        };
      }
      if (event.innerTextResolved) {
        return {
          selector: event.innerTextResolved.cssSelector ?? "",
          text: event.innerTextResolved.text ?? "",
          note: event.innerTextResolved.note ?? "",
        };
      }
      if (event.error?.message) {
        throw new Error(`page session error while reading text: ${event.error.message}`);
      }
      if (event.closed) {
        handle.closed = true;
        throw new Error(`page session ${this.sessionId} closed while waiting for text result`);
      }
    }
  }

  #ensureOpen(handle: PageHandle): void {
    if (handle.closed) {
      throw new Error(`page session ${this.sessionId} is closed`);
    }
  }

  async #getHandle(): Promise<PageHandle> {
    if (!this.#handlePromise) {
      this.#handlePromise = createPageHandle(this.#runtime);
    }
    return this.#handlePromise;
  }
}

export class Locator {
  readonly page: Page;
  readonly selector: string;

  constructor(input: LocatorInfo) {
    this.page = input.page;
    this.selector = input.selector;
  }

  async click(options: CommandOptions = {}): Promise<ClickResult> {
    return this.page.click(this.selector, options);
  }

  async count(options: CommandOptions = {}): Promise<CountResult> {
    return this.page.count(this.selector, options);
  }

  async highlight(options: HighlightOptions = {}): Promise<HighlightResult> {
    return this.page.highlight(this.selector, options);
  }

  async focus(options: CommandOptions = {}): Promise<ElementResult> {
    return this.page.focus(this.selector, options);
  }

  async fill(value: string, options: CommandOptions = {}): Promise<FillResult> {
    return this.page.fill(this.selector, value, options);
  }

  async hover(options: CommandOptions = {}): Promise<ElementResult> {
    return this.page.hover(this.selector, options);
  }

  async press(key: string, options: PressOptions = {}): Promise<PressResult> {
    return this.page.press(this.selector, key, options);
  }

  async textContent(options: CommandOptions = {}): Promise<TextResult> {
    return this.page.textContent(this.selector, options);
  }

  async innerText(options: CommandOptions = {}): Promise<TextResult> {
    return this.page.innerText(this.selector, options);
  }

  async waitFor(options: WaitForSelectorOptions = {}): Promise<WaitForSelectorResult> {
    return this.page.waitForSelector(this.selector, options);
  }

  locator(selector: string): Locator {
    return new Locator({
      page: this.page,
      selector: `${this.selector} ${selector}`,
    });
  }
}

export const chromium = new BrowserType();

export type Tab = Page;

export async function ping(): Promise<string> {
  const runtime = await getRuntime();
  return new Promise<string>((resolve, reject) => {
    runtime.client.Ping({}, (error, response) => {
      if (error) {
        reject(new Error(`ping engine server: ${error.message}`));
        return;
      }
      resolve(response.message ?? "");
    });
  });
}

export async function launchChrome(options: LaunchOptions = {}): Promise<Browser> {
  const runtime = await getRuntime();
  const stream = runtime.client.BrowserSession();
  const queue = bindStreamQueue(stream);

  stream.write({
    launchChrome: {
      chromeBinary: options.chromeBinary,
      retryOptions: options.timeoutMs ? { timeoutMs: options.timeoutMs } : undefined,
    },
  });

  while (true) {
    const event = await queue.next();
    if (event.chromeLaunched) {
      return new Browser({
        runtime,
        stream,
        queue,
        sessionId: event.sessionId ?? "",
        chromeLaunched: event.chromeLaunched,
      });
    }
    if (event.error?.message) {
      throw new Error(`browser session error during launch: ${event.error.message}`);
    }
  }
}

export function setServerAddr(serverAddr: string): void {
  serverAddrOverride = normalizeServerAddr(serverAddr);
  runtimePromise = null;
}

export async function shutdown(): Promise<void> {
  if (!runtimePromise) {
    return;
  }
  const runtime = await runtimePromise;
  runtime.client.close();
  runtimePromise = null;
}

async function getRuntime(): Promise<RuntimeClient> {
  if (!runtimePromise) {
    runtimePromise = Promise.resolve(createRuntime());
  }
  return runtimePromise;
}

function createRuntime(): RuntimeClient {
  const loaded = protoLoader.loadSync(ENGINE_PROTO_PATH, {
    includeDirs: [PROTO_ROOT],
    keepCase: false,
    longs: String,
    enums: String,
    defaults: true,
    oneofs: true,
  });
  const proto = grpc.loadPackageDefinition(loaded) as unknown as EngineProtoRoot;
  const ClientCtor = proto.allwright.engine.v1.EngineService;
  const client = new ClientCtor(
    configuredServerAddr(),
    grpc.credentials.createInsecure(),
  ) as unknown as EngineServiceClientShape;
  return { client };
}

function configuredServerAddr(): string {
  if (serverAddrOverride) {
    return serverAddrOverride;
  }
  return normalizeServerAddr(process.env[SERVER_ADDR_ENV_VAR] ?? DEFAULT_SERVER_ADDR);
}

function normalizeServerAddr(raw: string): string {
  const trimmed = raw.trim();
  if (trimmed.startsWith("dns:") || trimmed.startsWith("unix:")) {
    return trimmed;
  }
  if (trimmed.includes("://")) {
    const parsed = new URL(trimmed);
    return parsed.host;
  }
  return trimmed;
}

function bindStreamQueue<TEvent>(stream: grpc.ClientReadableStream<TEvent>): EventQueue<TEvent> {
  const queue = new EventQueue<TEvent>();
  stream.on("data", (event) => {
    queue.push(event);
  });
  stream.on("error", (error) => {
    queue.fail(new Error(`grpc stream error: ${error.message}`));
  });
  stream.on("end", () => {
    queue.fail(new Error("grpc stream ended"));
  });
  return queue;
}

async function createPageHandle(runtime: RuntimeClient): Promise<PageHandle> {
  const stream = runtime.client.TabSession();
  const queue = bindStreamQueue(stream);
  return {
    stream,
    queue,
    closed: false,
  };
}
