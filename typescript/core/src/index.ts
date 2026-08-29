import { BrowserImpl, BrowserTypeImpl } from "./browser.js";
import { formatActionError } from "./errors.js";
import { findConfigFile, loadConfigFile, resolveConfig } from "./config.js";
import { mobile } from "./mobile.js";
import { PageImpl } from "./page.js";
import {
  createBrowserSessionHandle,
  getRuntime,
  launchConfiguredBrowser as launchConfiguredBrowserWithResolver,
  ping as runtimePing,
  resolveLaunchBrowserArgs,
  setServerAddr,
  shutdown,
} from "./runtime.js";
import type {
  Browser,
  BrowserKind,
  BrowserType,
  LaunchOptions,
  MobileSurfaceNamespace,
  Page,
  ResolveConfigOptions,
  ResolvedAllwrightConfig,
} from "./types.js";

export { findConfigFile, loadConfigFile, resolveConfig, setServerAddr, shutdown };
export type {
  AllwrightConfig,
  Browser,
  BrowserInfo,
  BrowserKind,
  BrowserType,
  ClickResult,
  CommandOptions,
  CountResult,
  ElementResult,
  FillResult,
  HighlightOptions,
  HighlightResult,
  LaunchOptions,
  MobileAndroidConnectOptions,
  MobileAndroidDevice,
  MobileAndroidLaunchOptions,
  MobileAndroidPage,
  Locator,
  LocatorInfo,
  MobileSurfaceNamespace,
  NavigateResult,
  Page,
  PageInfo,
  PressOptions,
  PressResult,
  ResolveConfigOptions,
  ResolvedAllwrightConfig,
  RetryConfig,
  TextResult,
  WaitForSelectorOptions,
  WaitForSelectorResult,
} from "./types.js";

export const chromium: BrowserType = new BrowserTypeImpl("chromium");
export const firefox: BrowserType = new BrowserTypeImpl("firefox");
export { mobile };

export type Tab = Page;

export async function ping(): Promise<string> {
  return runtimePing();
}

export async function launchChrome(options: LaunchOptions = {}): Promise<Browser> {
  return launchBrowser("chromium", options);
}

export async function launchFirefox(options: LaunchOptions = {}): Promise<Browser> {
  return launchBrowser("firefox", options);
}

export async function launchConfiguredBrowser(config: ResolvedAllwrightConfig): Promise<Browser> {
  return launchConfiguredBrowserWithResolver(config, (browserKind, launchOptions) =>
    launchBrowser(browserKind, launchOptions),
  );
}

export async function launchBrowser(): Promise<Browser>;
export async function launchBrowser(options: ResolveConfigOptions): Promise<Browser>;
export async function launchBrowser(
  browserKind: BrowserKind,
  options?: LaunchOptions,
): Promise<Browser>;
export async function launchBrowser(
  browserKindOrOptions?: BrowserKind | ResolveConfigOptions,
  options: LaunchOptions = {},
): Promise<Browser> {
  if (resolveLaunchBrowserArgs(browserKindOrOptions)) {
    return launchConfiguredBrowser(resolveConfig(browserKindOrOptions ?? {}));
  }

  const runtime = await getRuntime();
  const { stream, queue } = await createBrowserSessionHandle(runtime);

  stream.write({
    launchBrowser: {
      browserKind: browserKindOrOptions === "firefox" ? 2 : 1,
      browserBinary: options.browserBinary,
      retryOptions: options.timeoutMs ? { timeoutMs: options.timeoutMs } : undefined,
    },
  });

  while (true) {
    const event = await queue.next();
    if (event.browserLaunched) {
      return new BrowserImpl({
        runtime,
        stream,
        queue,
        sessionId: event.sessionId ?? "",
        launched: event.browserLaunched,
      });
    }
    if (event.error?.message) {
      throw formatActionError("launch browser", event.error.message);
    }
  }
}

export { BrowserImpl, BrowserTypeImpl, PageImpl };
