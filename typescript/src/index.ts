import path from "node:path";
import { fileURLToPath } from "node:url";

import grpc from "@grpc/grpc-js";
import protoLoader from "@grpc/proto-loader";

const DEFAULT_SERVER_ADDR = "127.0.0.1:50051";
const SERVER_ADDR_ENV_VAR = "ALLWRIGHT_SERVER_ADDR";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const REPO_ROOT = path.resolve(__dirname, "..", "..");
const ENGINE_PROTO_PATH = path.join(REPO_ROOT, "proto", "engine", "v1", "engine.proto");

let runtimePromise: Promise<RuntimeClient> | null = null;
let serverAddrOverride: string | null = null;

export interface LaunchOptions {
  chromeBinary?: string;
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
}

interface LaunchChromeRequest {
  launchChrome: {
    chromeBinary?: string;
  };
}

interface OpenTabRequest {
  openTab: Record<string, never>;
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
  };
}

interface ClickRequest {
  browserSessionId: string;
  tabSessionId: string;
  clickElement: {
    cssSelector: string;
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

type TabSessionRequest = TabPingRequest | NavigateRequest | ClickRequest | CloseTabRequest;

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

  async newPage(): Promise<Page> {
    this.#ensureOpen();
    this.#stream.write({
      openTab: {},
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

  async newTab(): Promise<Page> {
    return this.newPage();
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

  async goto(url: string): Promise<NavigateResult> {
    const handle = await this.#getHandle();
    this.#ensureOpen(handle);
    handle.stream.write({
      browserSessionId: this.browserSessionId,
      tabSessionId: this.sessionId,
      navigate: {
        url,
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

  async click(selector: string): Promise<ClickResult> {
    const handle = await this.#getHandle();
    this.#ensureOpen(handle);
    handle.stream.write({
      browserSessionId: this.browserSessionId,
      tabSessionId: this.sessionId,
      clickElement: {
        cssSelector: selector,
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

  async navigate(url: string): Promise<NavigateResult> {
    return this.goto(url);
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
