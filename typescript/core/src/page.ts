import { LocatorImpl } from "./locator.js";
import { writeFile } from "node:fs/promises";
import { formatActionError } from "./errors.js";
import { normalizeSelectorForTransport } from "./selectors.js";
import { createPageHandle } from "./runtime.js";
import type {
  ClickResult,
  CommandOptions,
  CountResult,
  ElementResult,
  FillResult,
  HighlightOptions,
  HighlightResult,
  Locator,
  NavigateResult,
  Page,
  PageHandle,
  PageInfo,
  PressOptions,
  PressResult,
  RuntimeClient,
  ScreenshotOptions,
  ScreenshotResult,
  ContextSessionEvent,
  TextResult,
  WaitForSelectorOptions,
  WaitForSelectorResult,
} from "./types.js";

export class PageImpl implements Page {
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
    return new LocatorImpl({ page: this, selector: normalizeSelectorForTransport(selector) });
  }

  async goto(url: string, options: CommandOptions = {}): Promise<NavigateResult> {
    const handle = await this.#getHandle();
    this.#ensureOpen(handle);
    handle.stream.write({
      surfaceSessionId: this.browserSessionId,
      contextSessionId: this.sessionId,
      navigate: {
        url,
        retryOptions: options.timeoutMs ? { timeoutMs: options.timeoutMs } : undefined,
      },
    });

    let navigated: ContextSessionEvent["navigated"] | null = null;
    let injection: ContextSessionEvent["chromiumBidiInjection"] | null = null;

    while (true) {
      const event = await handle.queue.next();
      if (event.navigated) {
        navigated = event.navigated;
      }
      if (event.chromiumBidiInjection) {
        injection = event.chromiumBidiInjection;
      }
      if (event.error?.message) {
        throw formatActionError("navigate", event.error.message);
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
    const transportSelector = normalizeSelectorForTransport(selector);
    handle.stream.write({
      surfaceSessionId: this.browserSessionId,
      contextSessionId: this.sessionId,
      clickElement: {
        cssSelector: transportSelector,
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
        throw formatActionError("click", event.error.message, selector);
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
    const transportSelector = normalizeSelectorForTransport(selector);
    handle.stream.write({
      surfaceSessionId: this.browserSessionId,
      contextSessionId: this.sessionId,
      countElements: {
        cssSelector: transportSelector,
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
        throw formatActionError("count elements", event.error.message, selector);
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
    const transportSelector = normalizeSelectorForTransport(selector);
    handle.stream.write({
      surfaceSessionId: this.browserSessionId,
      contextSessionId: this.sessionId,
      highlightElements: {
        cssSelector: transportSelector,
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
        throw formatActionError("highlight elements", event.error.message, selector);
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
    const transportSelector = normalizeSelectorForTransport(selector);
    handle.stream.write({
      surfaceSessionId: this.browserSessionId,
      contextSessionId: this.sessionId,
      focusElement: {
        cssSelector: transportSelector,
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
        throw formatActionError("focus", event.error.message, selector);
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
    const transportSelector = normalizeSelectorForTransport(selector);
    handle.stream.write({
      surfaceSessionId: this.browserSessionId,
      contextSessionId: this.sessionId,
      fillElement: {
        cssSelector: transportSelector,
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
        throw formatActionError("fill", event.error.message, selector);
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
    const transportSelector = normalizeSelectorForTransport(selector);
    handle.stream.write({
      surfaceSessionId: this.browserSessionId,
      contextSessionId: this.sessionId,
      hoverElement: {
        cssSelector: transportSelector,
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
        throw formatActionError("hover", event.error.message, selector);
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
    const transportSelector = normalizeSelectorForTransport(selector);
    handle.stream.write({
      surfaceSessionId: this.browserSessionId,
      contextSessionId: this.sessionId,
      pressKey: {
        cssSelector: transportSelector,
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
        throw formatActionError("press key", event.error.message, selector);
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
    const transportSelector = normalizeSelectorForTransport(selector);
    handle.stream.write({
      surfaceSessionId: this.browserSessionId,
      contextSessionId: this.sessionId,
      waitForSelector: {
        cssSelector: transportSelector,
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
        throw formatActionError("wait for selector", event.error.message, selector);
      }
      if (event.closed) {
        handle.closed = true;
        throw new Error(`page session ${this.sessionId} closed while waiting for selector result`);
      }
    }
  }

  async screenshot(options: ScreenshotOptions = {}): Promise<ScreenshotResult> {
    const handle = await this.#getHandle();
    this.#ensureOpen(handle);
    handle.stream.write({
      surfaceSessionId: this.browserSessionId,
      contextSessionId: this.sessionId,
      screenshot: {
        retryOptions: options.timeoutMs ? { timeoutMs: options.timeoutMs } : undefined,
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
        throw formatActionError("screenshot", event.error.message);
      }
      if (event.closed) {
        handle.closed = true;
        throw new Error(`page session ${this.sessionId} closed while waiting for screenshot`);
      }
    }
  }

  async close(): Promise<void> {
    const handle = await this.#getHandle();
    if (handle.closed) {
      return;
    }

    handle.stream.write({
      surfaceSessionId: this.browserSessionId,
      contextSessionId: this.sessionId,
      close: {},
    });

    while (true) {
      const event = await handle.queue.next();
      if (event.closed) {
        handle.closed = true;
        this.dispose();
        return;
      }
      if (event.error?.message) {
        throw formatActionError("close page", event.error.message);
      }
    }
  }

  async ping(message = "ping"): Promise<string> {
    const handle = await this.#getHandle();
    this.#ensureOpen(handle);
    handle.stream.write({
      surfaceSessionId: this.browserSessionId,
      contextSessionId: this.sessionId,
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
        throw formatActionError("ping page", event.error.message);
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
    const transportSelector = normalizeSelectorForTransport(selector);
    handle.stream.write(
      textContent
        ? {
            surfaceSessionId: this.browserSessionId,
            contextSessionId: this.sessionId,
            getTextContent: {
              cssSelector: transportSelector,
              retryOptions: options.timeoutMs ? { timeoutMs: options.timeoutMs } : undefined,
            },
          }
        : {
            surfaceSessionId: this.browserSessionId,
            contextSessionId: this.sessionId,
            getInnerText: {
              cssSelector: transportSelector,
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
        throw formatActionError("read text", event.error.message, selector);
      }
      if (event.closed) {
        handle.closed = true;
        throw new Error(`page session ${this.sessionId} closed while waiting for text result`);
      }
    }
  }

  #ensureOpen(handle: PageHandle): void {
    if (handle.closed) {
      throw formatActionError("use page", `page session ${this.sessionId} is closed`);
    }
  }

  async #getHandle(): Promise<PageHandle> {
    if (!this.#handlePromise) {
      this.#handlePromise = createPageHandle(this.#runtime);
    }
    return this.#handlePromise;
  }

  dispose(): void {
    const handlePromise = this.#handlePromise;
    if (!handlePromise) {
      return;
    }
    void handlePromise.then((handle) => {
      if (handle.closed) {
        return;
      }
      handle.closed = true;
      try {
        handle.stream.end();
      } catch {
        // Best-effort local teardown during page disposal.
      }
      try {
        handle.stream.cancel();
      } catch {
        // grpc-js may reject cancel/end ordering depending on stream state.
      }
    });
  }
}
