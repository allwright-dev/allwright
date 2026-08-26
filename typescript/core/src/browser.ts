import { PageImpl } from "./page.js";
import type {
  Browser,
  BrowserInfo,
  BrowserKind,
  BrowserLaunchState,
  BrowserSessionEvent,
  BrowserSessionStream,
  BrowserType,
  CommandOptions,
  LaunchOptions,
  Page,
  RuntimeClient,
} from "./types.js";
import { EventQueue } from "./types.js";

export class BrowserTypeImpl implements BrowserType {
  #browserKind: BrowserKind;

  constructor(browserKind: BrowserKind = "chromium") {
    this.#browserKind = browserKind;
  }

  async launch(options: LaunchOptions = {}): Promise<Browser> {
    const { launchBrowser } = await import("./index.js");
    return launchBrowser(this.#browserKind, options);
  }
}

export class BrowserImpl implements Browser {
  #closed = false;
  #runtime: RuntimeClient;
  #stream: BrowserSessionStream;
  #queue: EventQueue<BrowserSessionEvent>;
  #pages = new Map<string, Page>();
  #initialPage: Page;

  constructor(state: BrowserLaunchState) {
    const browserInfo: BrowserInfo = {
      sessionId: state.sessionId,
      browserName: state.launched.browser ?? "",
      launchNote: state.launched.note ?? "",
      cdpWebSocketURL: "",
      userDataDir: state.launched.userDataDir ?? "",
    };

    this.#runtime = state.runtime;
    this.#stream = state.stream;
    this.#queue = state.queue;
    this.sessionId = browserInfo.sessionId;
    this.browserName = browserInfo.browserName;
    this.launchNote = browserInfo.launchNote;
    this.cdpWebSocketURL = browserInfo.cdpWebSocketURL;
    this.userDataDir = browserInfo.userDataDir;

    this.#initialPage = this.#createPage(state.launched.initialTabSessionId ?? "");
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
    const page = new PageImpl({
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
