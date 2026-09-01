import { createBrowserSessionHandle, createPageHandle, getRuntime } from "./runtime.js";
import { writeFile } from "node:fs/promises";
import { chainMobileSelectorForTransport, normalizeMobileSelectorForTransport } from "./mobileSelectors.js";
import type {
  SurfaceSessionEvent,
  SurfaceSessionStream,
  ClickResult,
  CommandOptions,
  CountResult,
  ElementResult,
  EventQueue,
  FillResult,
  MobileAndroidConnectOptions,
  MobileAndroidDevice,
  MobileAndroidLaunchOptions,
  MobileAndroidLocator,
  MobileAndroidApp,
  MobileSurfaceNamespace,
  PageHandle,
  PressOptions,
  PressResult,
  RuntimeClient,
  ScreenshotOptions,
  ScreenshotResult,
  TextResult,
  WaitForSelectorOptions,
  WaitForSelectorResult,
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

  async count(selector: string, options: CommandOptions = {}): Promise<CountResult> {
    const handle = await this.#getHandle();
    this.#ensureOpen(handle);
    handle.stream.write({
      surfaceSessionId: this.#surfaceSessionId,
      contextSessionId: this.sessionId,
      countElements: {
        cssSelector: normalizeMobileSelectorForTransport(selector),
        retryOptions: retryOptions(options.timeoutMs),
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
        throw new Error(event.error.message);
      }
      if (event.closed) {
        handle.closed = true;
        throw new Error(`android app session ${this.sessionId} closed while counting elements`);
      }
    }
  }

  async focus(selector: string, options: CommandOptions = {}): Promise<ElementResult> {
    const handle = await this.#getHandle();
    this.#ensureOpen(handle);
    handle.stream.write({
      surfaceSessionId: this.#surfaceSessionId,
      contextSessionId: this.sessionId,
      focusElement: {
        cssSelector: normalizeMobileSelectorForTransport(selector),
        retryOptions: retryOptions(options.timeoutMs),
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
        throw new Error(event.error.message);
      }
      if (event.closed) {
        handle.closed = true;
        throw new Error(`android app session ${this.sessionId} closed while focusing`);
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

  async press(selector: string, key: string, options: PressOptions = {}): Promise<PressResult> {
    const handle = await this.#getHandle();
    this.#ensureOpen(handle);
    handle.stream.write({
      surfaceSessionId: this.#surfaceSessionId,
      contextSessionId: this.sessionId,
      pressKey: {
        cssSelector: normalizeMobileSelectorForTransport(selector),
        key,
        text: options.text,
        retryOptions: retryOptions(options.timeoutMs),
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
        throw new Error(event.error.message);
      }
      if (event.closed) {
        handle.closed = true;
        throw new Error(`android app session ${this.sessionId} closed while pressing key`);
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
      surfaceSessionId: this.#surfaceSessionId,
      contextSessionId: this.sessionId,
      waitForSelector: {
        cssSelector: normalizeMobileSelectorForTransport(selector),
        visible: options.visible,
        retryOptions: retryOptions(options.timeoutMs),
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
        throw new Error(event.error.message);
      }
      if (event.closed) {
        handle.closed = true;
        throw new Error(`android app session ${this.sessionId} closed while waiting for selector`);
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

  async #readText(
    selector: string,
    options: CommandOptions,
    textContent: boolean,
  ): Promise<TextResult> {
    const handle = await this.#getHandle();
    this.#ensureOpen(handle);
    const transportSelector = normalizeMobileSelectorForTransport(selector);
    handle.stream.write(
      textContent
        ? {
            surfaceSessionId: this.#surfaceSessionId,
            contextSessionId: this.sessionId,
            getTextContent: {
              cssSelector: transportSelector,
              retryOptions: retryOptions(options.timeoutMs),
            },
          }
        : {
            surfaceSessionId: this.#surfaceSessionId,
            contextSessionId: this.sessionId,
            getInnerText: {
              cssSelector: transportSelector,
              retryOptions: retryOptions(options.timeoutMs),
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
        throw new Error(event.error.message);
      }
      if (event.closed) {
        handle.closed = true;
        throw new Error(`android app session ${this.sessionId} closed while reading text`);
      }
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

  async count(options: CommandOptions = {}): Promise<CountResult> {
    return this.page.count(this.selector, options);
  }

  async focus(options: CommandOptions = {}): Promise<ElementResult> {
    return this.page.focus(this.selector, options);
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
