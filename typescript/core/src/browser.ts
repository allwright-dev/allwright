import { PageImpl } from "./page.js";
import { formatActionError } from "./errors.js";
import type {
  Browser,
  BrowserInfo,
  BrowserKind,
  BrowserLaunchState,
  SurfaceSessionEvent,
  SurfaceSessionStream,
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
  #stream: SurfaceSessionStream;
  #queue: EventQueue<SurfaceSessionEvent>;
  #pages = new Map<string, PageImpl>();
  #initialPage: PageImpl;

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

    this.#initialPage = this.#createPage(state.launched.initialPageSessionId ?? "");
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
      openContext: {
        retryOptions: options.timeoutMs ? { timeoutMs: options.timeoutMs } : undefined,
      },
    });

    while (true) {
      const event = await this.#queue.next();
      if (event.contextOpened?.contextSessionId) {
        return this.#createPage(event.contextOpened.contextSessionId);
      }
      if (event.error?.message) {
        throw formatActionError("open page", event.error.message);
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
        for (const page of this.#pages.values()) {
          page.dispose();
        }
        this.#stream.end();
        return;
      }
      if (event.error?.message) {
        throw formatActionError("close browser", event.error.message);
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
        throw formatActionError("ping browser", event.error.message);
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

  #createPage(sessionId: string): PageImpl {
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
      throw formatActionError("use browser", `browser session ${this.sessionId} is closed`);
    }
  }
}
