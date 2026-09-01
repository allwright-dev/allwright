import { createBrowserSessionHandle, createPageHandle, getRuntime } from "./runtime.js";
import { writeFile } from "node:fs/promises";
import { chainMobileSelectorForTransport, normalizeMobileSelectorForTransport } from "./mobileSelectors.js";
import type {
  SurfaceSessionEvent,
  SurfaceSessionStream,
  ClickResult,
  CommandOptions,
  EventQueue,
  FillResult,
  MobileAndroidConnectOptions,
  MobileAndroidDevice,
  MobileAndroidLaunchOptions,
  MobileAndroidLocator,
  MobileAndroidApp,
  MobileSurfaceNamespace,
  PageHandle,
  RuntimeClient,
  ScreenshotOptions,
  ScreenshotResult,
} from "./types.js";

function retryOptions(timeoutMs?: number): { timeoutMs?: number } | undefined {
  return timeoutMs ? { timeoutMs } : undefined;
}

class MobileAndroidAppImpl implements MobileAndroidApp {
  #runtime: RuntimeClient;
  #surfaceSessionId: string;
  #handlePromise: Promise<PageHandle> | null = null;

  constructor(runtime: RuntimeClient, surfaceSessionId: string, readonly sessionId: string) {
    this.#runtime = runtime;
    this.#surfaceSessionId = surfaceSessionId;
  }

  locator(selector: string): MobileAndroidLocator {
    return new MobileAndroidLocatorImpl(this, normalizeMobileSelectorForTransport(selector));
  }

  async click(selector: string, options: CommandOptions = {}): Promise<ClickResult> {
    const handle = await this.#getHandle();
    this.#ensureOpen(handle);
    handle.stream.write({
      surfaceSessionId: this.#surfaceSessionId,
      contextSessionId: this.sessionId,
      clickElement: {
        cssSelector: normalizeMobileSelectorForTransport(selector),
        retryOptions: retryOptions(options.timeoutMs),
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
        throw new Error(event.error.message);
      }
      if (event.closed) {
        handle.closed = true;
        throw new Error(`android app session ${this.sessionId} closed while clicking`);
      }
    }
  }

  async fill(selector: string, value: string, options: CommandOptions = {}): Promise<FillResult> {
    const handle = await this.#getHandle();
    this.#ensureOpen(handle);
    handle.stream.write({
      surfaceSessionId: this.#surfaceSessionId,
      contextSessionId: this.sessionId,
      fillElement: {
        cssSelector: normalizeMobileSelectorForTransport(selector),
        value,
        retryOptions: retryOptions(options.timeoutMs),
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
        throw new Error(event.error.message);
      }
      if (event.closed) {
        handle.closed = true;
        throw new Error(`android app session ${this.sessionId} closed while filling`);
      }
    }
  }

  async screenshot(options: ScreenshotOptions = {}): Promise<ScreenshotResult> {
    const handle = await this.#getHandle();
    this.#ensureOpen(handle);
    handle.stream.write({
      surfaceSessionId: this.#surfaceSessionId,
      contextSessionId: this.sessionId,
      screenshot: {
        retryOptions: retryOptions(options.timeoutMs),
        fullPage: options.fullPage,
      },
    });

    while (true) {
      const event = await handle.queue.next();
      if (event.screenshotCaptured?.pngData) {
        const screenshot = {
          pngData: event.screenshotCaptured.pngData,
          note: event.screenshotCaptured.note ?? "",
        };
        if (options.path) {
          await writeFile(options.path, screenshot.pngData);
        }
        return screenshot;
      }
      if (event.error?.message) {
        throw new Error(event.error.message);
      }
      if (event.closed) {
        handle.closed = true;
        throw new Error(`android app session ${this.sessionId} closed while capturing screenshot`);
      }
    }
  }

  async #getHandle(): Promise<PageHandle> {
    if (!this.#handlePromise) {
      this.#handlePromise = createPageHandle(this.#runtime);
    }
    return this.#handlePromise;
  }

  #ensureOpen(handle: PageHandle): void {
    if (handle.closed) {
      throw new Error(`android app session ${this.sessionId} is closed`);
    }
  }
}

class MobileAndroidLocatorImpl implements MobileAndroidLocator {
  constructor(
    readonly page: MobileAndroidApp,
    readonly selector: string,
  ) {}

  async click(options: CommandOptions = {}): Promise<ClickResult> {
    return this.page.click(this.selector, options);
  }

  async fill(value: string, options: CommandOptions = {}): Promise<FillResult> {
    return this.page.fill(this.selector, value, options);
  }

  locator(selector: string): MobileAndroidLocator {
    return new MobileAndroidLocatorImpl(this.page, chainMobileSelectorForTransport(this.selector, selector));
  }
}

class MobileAndroidDeviceImpl implements MobileAndroidDevice {
  #stream: SurfaceSessionStream;
  #queue: EventQueue<SurfaceSessionEvent>;
  #closed = false;
  #currentApp: MobileAndroidApp;

  constructor(
    readonly sessionId: string,
    private readonly surfaceSessionId: string,
    private readonly runtime: RuntimeClient,
    stream: Awaited<ReturnType<typeof createBrowserSessionHandle>>["stream"],
    queue: Awaited<ReturnType<typeof createBrowserSessionHandle>>["queue"],
    initialAppSessionId: string,
  ) {
    this.#stream = stream;
    this.#queue = queue;
    this.#currentApp = new MobileAndroidAppImpl(runtime, surfaceSessionId, initialAppSessionId);
  }

  app(): MobileAndroidApp {
    return this.#currentApp;
  }

  initialApp(): MobileAndroidApp {
    return this.#currentApp;
  }

  async launch(options: MobileAndroidLaunchOptions = {}): Promise<MobileAndroidApp> {
    this.#ensureOpen();
    this.#stream.write({
      launchApp: {
        apkPath: options.apkPath,
        appId: options.appId,
        launchActivity: options.launchActivity,
        stopBeforeLaunch: options.stopBeforeLaunch ?? false,
        retryOptions: retryOptions(options.timeoutMs),
      },
    });

    while (true) {
      const event = await this.#queue.next();
      if (event.appLaunched?.appSessionId) {
        this.#currentApp = new MobileAndroidAppImpl(
          this.runtime,
          this.surfaceSessionId,
          event.appLaunched.appSessionId,
        );
        return this.#currentApp;
      }
      if (event.error?.message) {
        throw new Error(event.error.message);
      }
      if (event.closed) {
        this.#closed = true;
        throw new Error(`android device session ${this.sessionId} closed while launching app`);
      }
    }
  }

  #ensureOpen(): void {
    if (this.#closed) {
      throw new Error(`android device session ${this.sessionId} is closed`);
    }
  }
}

class MobileAndroidSurfaceImpl {
  async connect(options: MobileAndroidConnectOptions = {}): Promise<MobileAndroidDevice> {
    const runtime = await getRuntime();
    const { stream, queue } = await createBrowserSessionHandle(runtime);
    stream.write({
      connectMobile: {
        platform: 1,
        device: options.device,
        adbEndpoint: options.adbEndpoint,
        preserveAppState: options.preserveAppState ?? false,
        retryOptions: retryOptions(options.timeoutMs),
      },
    });

    while (true) {
      const event: SurfaceSessionEvent = await queue.next();
      if (event.mobileConnected?.initialAppSessionId) {
        return new MobileAndroidDeviceImpl(
          event.mobileConnected.deviceSessionId ?? event.sessionId ?? "",
          event.sessionId ?? "",
          runtime,
          stream,
          queue,
          event.mobileConnected.initialAppSessionId,
        );
      }
      if (event.error?.message) {
        throw new Error(event.error.message);
      }
    }
  }
}

export const mobile: MobileSurfaceNamespace = {
  android: new MobileAndroidSurfaceImpl(),
};
