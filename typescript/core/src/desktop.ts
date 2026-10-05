import { createBrowserSessionHandle, getRuntime } from "./runtime.js";
import { NativeAppImpl } from "./mobile.js";
import type {
  DesktopMacConnectOptions,
  DesktopMacDesktop,
  DesktopMacLaunchOptions,
  DesktopSurfaceNamespace,
  EventQueue,
  NativeApp,
  RuntimeClient,
  SurfaceSessionEvent,
  SurfaceSessionStream,
} from "./types.js";

function retryOptions(timeoutMs?: number): { timeoutMs?: number } | undefined {
  return timeoutMs === undefined ? undefined : { timeoutMs };
}

class DesktopMacImpl implements DesktopMacDesktop {
  #closed = false;
  #currentApp: NativeApp;

  constructor(
    readonly sessionId: string,
    private readonly surfaceSessionId: string,
    private readonly runtime: RuntimeClient,
    private readonly stream: SurfaceSessionStream,
    private readonly queue: EventQueue<SurfaceSessionEvent>,
    initialAppSessionId: string,
  ) {
    this.#currentApp = new NativeAppImpl(runtime, surfaceSessionId, initialAppSessionId);
  }

  app(): NativeApp {
    return this.#currentApp;
  }

  async launch(options: DesktopMacLaunchOptions): Promise<NativeApp> {
    if (this.#closed) throw new Error(`macOS desktop session ${this.sessionId} is closed`);
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
        this.#currentApp = new NativeAppImpl(
          this.runtime,
          this.surfaceSessionId,
          event.desktopAppLaunched.appSessionId,
        );
        return this.#currentApp;
      }
      if (event.error?.message) throw new Error(event.error.message);
      if (event.closed) {
        this.#closed = true;
        throw new Error(`macOS desktop session ${this.sessionId} closed while launching app`);
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
        );
      }
      if (event.error?.message) throw new Error(event.error.message);
    }
  }
}

export const desktop: DesktopSurfaceNamespace = {
  mac: new DesktopMacSurfaceImpl(),
};
