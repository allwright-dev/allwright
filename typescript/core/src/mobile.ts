import { invokePlugin } from "./bootstrap.js";
import { chainMobileSelectorForTransport, normalizeMobileSelectorForTransport } from "./mobileSelectors.js";
import type {
  ClickResult,
  CommandOptions,
  FillResult,
  MobileAndroidConnectOptions,
  MobileAndroidDevice,
  MobileAndroidLaunchOptions,
  MobileAndroidLocator,
  MobileAndroidPage,
  MobileSurfaceNamespace,
} from "./types.js";

type MobilePlatform = "android" | "ios";
type DeviceConnectionKind = "usb" | "emulator" | "remote_adb";

type MobileBrowserSessionHandle = {
  platform: MobilePlatform;
  automation: {
    backend: string;
    session_id: string;
    note: string;
  };
  device: {
    platform: MobilePlatform;
    device_id: string;
    connection_kind: DeviceConnectionKind;
  };
};

type MobilePageSessionHandle = {
  page_id: string;
  package_name?: string | null;
  activity_name?: string | null;
  webview_context?: string | null;
};

type MobilePageInfo = {
  note: string;
  page_session: MobilePageSessionHandle;
};

type MobileConnectInfo = {
  browser: string;
  note: string;
  browser_session: MobileBrowserSessionHandle;
  initial_page: MobilePageInfo;
};

type MobilePluginEnvelope<T> = {
  ok: boolean;
  result?: T;
  error?: string;
};

function timeoutMsOf(options?: CommandOptions | MobileAndroidConnectOptions | MobileAndroidLaunchOptions): number | undefined {
  return options?.timeoutMs;
}

async function invokeAndroidExpected<T>(commandName: string, request: unknown): Promise<T> {
  const envelope = await invokePlugin<MobilePluginEnvelope<T>>("mobile-android", request);
  if (!envelope.ok) {
    throw new Error(envelope.error ?? `mobile-android plugin ${commandName} failed`);
  }
  if (envelope.result === undefined) {
    throw new Error(`mobile-android plugin ${commandName} returned no result`);
  }
  return envelope.result;
}

class MobileAndroidPageImpl implements MobileAndroidPage {
  constructor(
    private readonly browserSession: MobileBrowserSessionHandle,
    private pageSession: MobilePageSessionHandle,
  ) {}

  get sessionId(): string {
    return this.pageSession.page_id;
  }

  locator(selector: string): MobileAndroidLocator {
    return new MobileAndroidLocatorImpl(this, normalizeMobileSelectorForTransport(selector));
  }

  async click(selector: string, options: CommandOptions = {}): Promise<ClickResult> {
    const result = await invokeAndroidExpected<{
      result: "click_element";
      selector: string;
      note: string;
      session_id: string;
    }>("click", {
      command: "click_element",
      browser_session: this.browserSession,
      page_session: this.pageSession,
      selector: normalizeMobileSelectorForTransport(selector),
      timeout_ms: timeoutMsOf(options),
    });
    return {
      selector: result.selector,
      note: result.note,
      bidiSessionId: result.session_id,
    };
  }

  async fill(selector: string, value: string, options: CommandOptions = {}): Promise<FillResult> {
    const result = await invokeAndroidExpected<{
      result: "fill_element";
      selector: string;
      value: string;
      note: string;
    }>("fill", {
      command: "fill_element",
      browser_session: this.browserSession,
      page_session: this.pageSession,
      selector: normalizeMobileSelectorForTransport(selector),
      value,
      timeout_ms: timeoutMsOf(options),
    });
    return {
      selector: result.selector,
      value: result.value,
      note: result.note,
    };
  }
}

class MobileAndroidLocatorImpl implements MobileAndroidLocator {
  constructor(
    readonly page: MobileAndroidPage,
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
  #initialPage: MobileAndroidPageImpl;

  constructor(
    private readonly connectInfoRaw: MobileConnectInfo,
  ) {
    this.#initialPage = new MobileAndroidPageImpl(
      this.connectInfoRaw.browser_session,
      this.connectInfoRaw.initial_page.page_session,
    );
  }

  get sessionId(): string {
    return this.connectInfoRaw.browser_session.automation.session_id;
  }

  page(): MobileAndroidPage {
    return this.#initialPage;
  }

  initialPage(): MobileAndroidPage {
    return this.#initialPage;
  }

  async launch(options: MobileAndroidLaunchOptions = {}): Promise<MobileAndroidPage> {
    const page = await invokeAndroidExpected<MobilePageInfo>("launch", {
      command: "launch_app",
      browser_session: this.connectInfoRaw.browser_session,
      options: {
        apk_path: options.apkPath,
        app_id: options.appId,
        launch_activity: options.launchActivity,
        stop_before_launch: options.stopBeforeLaunch ?? false,
        timeout_ms: timeoutMsOf(options),
      },
    });
    this.#initialPage = new MobileAndroidPageImpl(this.connectInfoRaw.browser_session, page.page_session);
    return this.#initialPage;
  }
}

class MobileAndroidSurfaceImpl {
  async connect(options: MobileAndroidConnectOptions = {}): Promise<MobileAndroidDevice> {
    const connectInfo = await invokeAndroidExpected<MobileConnectInfo>("connect", {
      command: "connect",
      platform: "android",
      device: options.device,
      adb_endpoint: options.adbEndpoint,
      preserve_app_state: options.preserveAppState ?? false,
      timeout_ms: timeoutMsOf(options),
    });
    return new MobileAndroidDeviceImpl(connectInfo);
  }
}

export const mobile: MobileSurfaceNamespace = {
  android: new MobileAndroidSurfaceImpl(),
};
