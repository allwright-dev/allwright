import { createBrowserSessionHandle, createPageHandle, getRuntime } from "./runtime.js";
import { open, rename, unlink, writeFile } from "node:fs/promises";
import { randomUUID } from "node:crypto";
import { basename } from "node:path";
import { chainMobileSelectorForTransport, normalizeMobileSelectorForTransport } from "./mobileSelectors.js";
import type {
  AccessibilitySnapshotOptions,
  SurfaceSessionEvent,
  SurfaceSessionStream,
  ClickResult,
  CommandOptions,
  CountResult,
  ElementResult,
  EventQueue,
  FillResult,
  FileChooser,
  Download,
  Hook,
  HookType,
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

  async registerHook<T>(type: HookType<T>): Promise<Hook<T>> {
    if (type.name !== "fileChooser" && type.name !== "download") {
      throw new Error(`hook type ${type.name} is not supported by Android apps`);
    }
    const handle = await this.#getHandle();
    this.#ensureOpen(handle);
    handle.stream.write({
      surfaceSessionId: this.#surfaceSessionId,
      contextSessionId: this.sessionId,
      registerHook: type.name === "fileChooser"
        ? { mobileFileChooser: {} }
        : { mobileDownload: {} },
    });
    while (true) {
      const event = await handle.queue.next();
      if (event.hookRegistered?.hookId) {
        const hookId = event.hookRegistered.hookId;
        return {
          id: hookId,
          type,
          wait: (options: CommandOptions = {}) => this.#waitForHook(hookId, type, options),
        };
      }
      if (event.error?.message) throw new Error(event.error.message);
    }
  }

  async #waitForHook<T>(hookId: string, type: HookType<T>, options: CommandOptions): Promise<T> {
    const handle = await this.#getHandle();
    this.#ensureOpen(handle);
    handle.stream.write({
      surfaceSessionId: this.#surfaceSessionId,
      contextSessionId: this.sessionId,
      waitForHook: { hookId, retryOptions: retryOptions(options.timeoutMs) },
    });
    while (true) {
      const event = await handle.queue.next();
      if (event.hookCompleted?.hookId === hookId) {
        if (type.name === "fileChooser" && event.hookCompleted.mobileFileChooser?.fileChooserId) {
          return new MobileFileChooserImpl(
            this,
            event.hookCompleted.mobileFileChooser.fileChooserId,
            event.hookCompleted.mobileFileChooser.isMultiple ?? false,
          ) as T;
        }
        if (type.name === "download" && event.hookCompleted.mobileDownload?.downloadId) {
          return new MobileDownloadImpl(
            this,
            event.hookCompleted.mobileDownload.downloadId,
            event.hookCompleted.mobileDownload.suggestedFilename ?? "download",
          ) as T;
        }
        throw new Error(`Android ${type.name} hook returned an invalid result`);
      }
      if (event.error?.message) throw new Error(event.error.message);
    }
  }

  async setChooserFiles(
    fileChooserId: string,
    files: string[],
    options: CommandOptions,
  ): Promise<void> {
    const handle = await this.#getHandle();
    this.#ensureOpen(handle);
    const fileIds = [];
    for (const file of files) fileIds.push(await this.#uploadLocalFile(handle, file));
    handle.stream.write({
      surfaceSessionId: this.#surfaceSessionId,
      contextSessionId: this.sessionId,
      setMobileFileChooserFiles: {
        fileChooserId,
        fileIds,
        retryOptions: retryOptions(options.timeoutMs),
      },
    });
    while (true) {
      const event = await handle.queue.next();
      if (event.mobileFileChooserFilesSet?.fileChooserId === fileChooserId) return;
      if (event.error?.message) throw new Error(event.error.message);
    }
  }

  async saveMobileDownload(
    downloadId: string,
    path: string,
    options: CommandOptions,
  ): Promise<void> {
    const handle = await this.#getHandle();
    this.#ensureOpen(handle);
    handle.stream.write({
      surfaceSessionId: this.#surfaceSessionId,
      contextSessionId: this.sessionId,
      saveMobileDownload: { downloadId, retryOptions: retryOptions(options.timeoutMs) },
    });
    while (true) {
      const event = await handle.queue.next();
      if (event.mobileDownloadSaved?.downloadId === downloadId && event.mobileDownloadSaved.fileId) {
        await this.#downloadToLocalFile(handle, event.mobileDownloadSaved.fileId, path);
        return;
      }
      if (event.error?.message) throw new Error(event.error.message);
    }
  }

  async #uploadLocalFile(handle: PageHandle, path: string): Promise<string> {
    const source = await open(path, "r");
    const transferId = randomUUID();
    let offset = 0;
    try {
      const size = (await source.stat()).size;
      do {
        const buffer = Buffer.alloc(Math.min(256 * 1024, Math.max(0, size - offset)) || 1);
        const { bytesRead } = await source.read(buffer, 0, buffer.length, offset);
        const last = offset + bytesRead >= size;
        handle.stream.write({
          surfaceSessionId: this.#surfaceSessionId,
          contextSessionId: this.sessionId,
          uploadFileChunk: {
            transferId,
            name: basename(path),
            offset: String(offset),
            data: buffer.subarray(0, bytesRead),
            last,
          },
        });
        offset += bytesRead;
      } while (offset < size);
    } finally {
      await source.close();
    }
    while (true) {
      const event = await handle.queue.next();
      if (event.fileUploaded?.transferId === transferId && event.fileUploaded.fileId) {
        return event.fileUploaded.fileId;
      }
      if (event.error?.message) throw new Error(event.error.message);
    }
  }

  async #downloadToLocalFile(handle: PageHandle, fileId: string, path: string): Promise<void> {
    const temporaryPath = `${path}.allwright-${randomUUID()}.tmp`;
    const destination = await open(temporaryPath, "wx");
    let offset = 0;
    try {
      while (true) {
        handle.stream.write({
          surfaceSessionId: this.#surfaceSessionId,
          contextSessionId: this.sessionId,
          readFileChunk: { fileId, offset: String(offset), maxBytes: 256 * 1024 },
        });
        const event = await handle.queue.next();
        if (event.error?.message) throw new Error(event.error.message);
        if (event.fileChunk?.fileId !== fileId || Number(event.fileChunk.offset ?? -1) !== offset) continue;
        const data = event.fileChunk.data ?? new Uint8Array();
        await destination.write(data, 0, data.length, offset);
        offset += data.length;
        if (event.fileChunk.last) break;
      }
      await destination.close();
      await rename(temporaryPath, path);
    } catch (error) {
      await destination.close().catch(() => undefined);
      await unlink(temporaryPath).catch(() => undefined);
      throw error;
    }
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

  async accessibilitySnapshot(options: AccessibilitySnapshotOptions = {}): Promise<string> {
    const mode = options.mode ?? "default";
    if (!["default", "ai", "autoexpect", "codegen"].includes(mode)) {
      throw new Error("invalid accessibility snapshot mode");
    }
    const format = options.format ?? "json";
    if (format !== "json" && format !== "yaml") {
      throw new Error("accessibility snapshot format must be 'json' or 'yaml'");
    }
    const handle = await this.#getHandle();
    this.#ensureOpen(handle);
    handle.stream.write({
      surfaceSessionId: this.#surfaceSessionId,
      contextSessionId: this.sessionId,
      accessibilitySnapshot: {
        format,
        mode,
        retryOptions: options.timeoutMs ? { timeoutMs: options.timeoutMs } : undefined,
      },
    });
    while (true) {
      const event = await handle.queue.next();
      if (event.accessibilitySnapshotCaptured) return event.accessibilitySnapshotCaptured.snapshot ?? "";
      if (event.error?.message) throw new Error(event.error.message);
      if (event.closed) {
        handle.closed = true;
        throw new Error(`android app session ${this.sessionId} closed while waiting for accessibility snapshot`);
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

class MobileFileChooserImpl implements FileChooser {
  constructor(
    readonly page: MobileAndroidAppImpl,
    readonly id: string,
    private readonly multiple: boolean,
  ) {}

  isMultiple(): boolean {
    return this.multiple;
  }

  async setFiles(files: string | string[], options: CommandOptions = {}): Promise<void> {
    const paths = typeof files === "string" ? [files] : files;
    if (!this.multiple && paths.length > 1) {
      throw new Error("Android file chooser does not accept multiple files");
    }
    await this.page.setChooserFiles(this.id, paths, options);
  }
}

class MobileDownloadImpl implements Download {
  readonly url = "";

  constructor(
    readonly page: MobileAndroidAppImpl,
    readonly id: string,
    readonly suggestedFilename: string,
  ) {}

  async saveAs(path: string, options: CommandOptions = {}): Promise<void> {
    await this.page.saveMobileDownload(this.id, path, options);
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
