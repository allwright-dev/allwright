import { createBrowserSessionHandle, getRuntime } from "./runtime.js";
import { NativeAppImpl } from "./mobile.js";
import type {
  CommandOptions,
  DesktopApp,
  DesktopMacConnectOptions,
  DesktopMacDesktop,
  DesktopMacLaunchOptions,
  DesktopSurfaceNamespace,
  DesktopWindowsConnectOptions,
  DesktopWindowsDesktop,
  EventQueue,
  Hook,
  HookType,
  NativeApp,
  RuntimeClient,
  SurfaceSessionEvent,
  SurfaceSessionStream,
} from "./types.js";

function retryOptions(timeoutMs?: number): { timeoutMs?: number } | undefined {
  return timeoutMs === undefined ? undefined : { timeoutMs };
}

export class DesktopWindowImpl extends NativeAppImpl {
  constructor(
    runtime: RuntimeClient,
    surfaceSessionId: string,
    sessionId: string,
    readonly windowId: string,
    readonly title: string,
    readonly isMain: boolean,
  ) {
    super(runtime, surfaceSessionId, sessionId);
  }

  async close(): Promise<void> {
    const handle = await this.contextHandle();
    this.assertContextOpen(handle);
    handle.stream.write({
      surfaceSessionId: this.nativeSurfaceSessionId,
      contextSessionId: this.sessionId,
      closeDesktopWindow: {},
    });
    while (true) {
      const event = await handle.queue.next();
      if (event.desktopWindowClosed?.windowId === this.windowId) return;
      if (event.error?.message) throw new Error(event.error.message);
    }
  }
}

export class DesktopAppImpl extends NativeAppImpl implements DesktopApp {
  async windows(): Promise<DesktopWindowImpl[]> {
    const handle = await this.contextHandle();
    this.assertContextOpen(handle);
    handle.stream.write({
      surfaceSessionId: this.nativeSurfaceSessionId,
      contextSessionId: this.sessionId,
      listDesktopWindows: {},
    });
    while (true) {
      const event = await handle.queue.next();
      if (event.desktopWindowsResolved) {
        return (event.desktopWindowsResolved.windows ?? []).map((window) =>
          this.windowFromResult(window),
        );
      }
      if (event.error?.message) throw new Error(event.error.message);
    }
  }

  async mainWindow(): Promise<DesktopWindowImpl> {
    const windows = await this.windows();
    const main = windows.find((window) => window.isMain);
    if (!main) throw new Error("desktop application does not currently expose a main window");
    return main;
  }

  override async registerHook<T>(type: HookType<T>): Promise<Hook<T>> {
    if (type.name !== "window") return super.registerHook(type);
    const handle = await this.contextHandle();
    this.assertContextOpen(handle);
    handle.stream.write({
      surfaceSessionId: this.nativeSurfaceSessionId,
      contextSessionId: this.sessionId,
      registerHook: { desktopWindow: {} },
    });
    while (true) {
      const event = await handle.queue.next();
      if (event.hookRegistered?.hookId) {
        const hookId = event.hookRegistered.hookId;
        return {
          id: hookId,
          type,
          wait: (options: CommandOptions = {}) => this.waitForWindow(hookId, options) as Promise<T>,
        };
      }
      if (event.error?.message) throw new Error(event.error.message);
    }
  }

  private async waitForWindow(hookId: string, options: CommandOptions): Promise<DesktopWindowImpl> {
    const handle = await this.contextHandle();
    this.assertContextOpen(handle);
    handle.stream.write({
      surfaceSessionId: this.nativeSurfaceSessionId,
      contextSessionId: this.sessionId,
      waitForHook: { hookId, retryOptions: retryOptions(options.timeoutMs) },
    });
    while (true) {
      const event = await handle.queue.next();
      if (event.hookCompleted?.hookId === hookId) {
        const window = event.hookCompleted.desktopWindow?.window;
        if (!window) throw new Error("desktop window hook returned an invalid result");
        return this.windowFromResult(window);
      }
      if (event.error?.message) throw new Error(event.error.message);
    }
  }

  private windowFromResult(window: {
    contextSessionId?: string;
    windowId?: string;
    title?: string;
    isMain?: boolean;
  }): DesktopWindowImpl {
    if (!window.contextSessionId || !window.windowId) {
      throw new Error("desktop window result is missing its context or window id");
    }
    return new DesktopWindowImpl(
      this.runtimeClient,
      this.nativeSurfaceSessionId,
      window.contextSessionId,
      window.windowId,
      window.title ?? "",
      window.isMain ?? false,
    );
  }
}

class DesktopMacImpl implements DesktopMacDesktop {
  #closed = false;
  #currentApp: DesktopApp;

  constructor(
    readonly sessionId: string,
    private readonly surfaceSessionId: string,
    private readonly runtime: RuntimeClient,
    private readonly stream: SurfaceSessionStream,
    private readonly queue: EventQueue<SurfaceSessionEvent>,
    initialAppSessionId: string,
    private readonly platformLabel = "macOS",
  ) {
    this.#currentApp = new DesktopAppImpl(runtime, surfaceSessionId, initialAppSessionId);
  }

  app(): DesktopApp {
    return this.#currentApp;
  }

  async launch(options: DesktopMacLaunchOptions): Promise<DesktopApp> {
    if (this.#closed) throw new Error(`${this.platformLabel} desktop session ${this.sessionId} is closed`);
    this.stream.write({
      launchDesktopApp: {
        appId: options.appId,
        terminateRunning: options.terminateRunning ?? false,
        retryOptions: retryOptions(options.timeoutMs),
      },
    });
    while (true) {
      const event = await this.queue.next();
      if (event.desktopAppLaunched?.appSessionId) {
        this.#currentApp = new DesktopAppImpl(
          this.runtime,
          this.surfaceSessionId,
          event.desktopAppLaunched.appSessionId,
        );
        return this.#currentApp;
      }
      if (event.error?.message) throw new Error(event.error.message);
      if (event.closed) {
        this.#closed = true;
        throw new Error(`${this.platformLabel} desktop session ${this.sessionId} closed while launching app`);
      }
    }
  }
}

class DesktopMacSurfaceImpl {
  async connect(options: DesktopMacConnectOptions = {}): Promise<DesktopMacDesktop> {
    const runtime = await getRuntime();
    const { stream, queue } = await createBrowserSessionHandle(runtime);
    stream.write({
      connectDesktop: {
        platform: 1,
        agentEndpoint: options.agentEndpoint,
        retryOptions: retryOptions(options.timeoutMs),
      },
    });
    while (true) {
      const event = await queue.next();
      if (event.desktopConnected?.initialAppSessionId) {
        return new DesktopMacImpl(
          event.desktopConnected.desktopSessionId ?? event.sessionId ?? "",
          event.sessionId ?? "",
          runtime,
          stream,
          queue,
          event.desktopConnected.initialAppSessionId,
          "macOS",
        );
      }
      if (event.error?.message) throw new Error(event.error.message);
    }
  }
}

class DesktopWindowsSurfaceImpl {
  async connect(options: DesktopWindowsConnectOptions = {}): Promise<DesktopWindowsDesktop> {
    const runtime = await getRuntime();
    const { stream, queue } = await createBrowserSessionHandle(runtime);
    stream.write({
      connectDesktop: {
        platform: 2,
        agentEndpoint: options.agentEndpoint,
        retryOptions: retryOptions(options.timeoutMs),
      },
    });
    while (true) {
      const event = await queue.next();
      if (event.desktopConnected?.initialAppSessionId) {
        return new DesktopMacImpl(
          event.desktopConnected.desktopSessionId ?? event.sessionId ?? "",
          event.sessionId ?? "",
          runtime,
          stream,
          queue,
          event.desktopConnected.initialAppSessionId,
          "Windows",
        );
      }
      if (event.error?.message) throw new Error(event.error.message);
    }
  }
}

export const desktop: DesktopSurfaceNamespace = {
  mac: new DesktopMacSurfaceImpl(),
  windows: new DesktopWindowsSurfaceImpl(),
};
