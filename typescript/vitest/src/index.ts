import { createLocatorExpect, createPageExpect } from "./expectations.js";
import type { RetryExpectationOptions, PageExpectMatchers, LocatorExpectMatchers } from "./expectations.js";
export type { RetryExpectationOptions, TextExpectationOptions, VisibleExpectationOptions, PageExpectMatchers, LocatorExpectMatchers } from "./expectations.js";
import {
  launchConfiguredBrowser,
  mobile,
  resolveConfig,
  setServerAddr,
  type MobileAndroidConnectOptions,
  type MobileAndroidDevice,
  type MobileAndroidLaunchOptions,
  type MobileAndroidApp,
  type Browser,
  type BrowserKind,
  type CommandOptions,
  type Hook,
  type HookType,
  type LaunchOptions,
  type Locator,
  type MobileAndroidLocator,
  type Page,
  type ResolvedAllwrightConfig,
  type ScreenshotOptions,
  type WaitForSelectorOptions,
} from "@allwright.dev/core";
import { expect as vitestExpect, test as base, type Assertion } from "vitest";

export interface AllwrightVitestOptions {
  launchOptions?: LaunchOptions;
  serverAddr?: string;
  browser?: BrowserKind;
  browserBinary?: string;
  android?: {
    connectOptions?: MobileAndroidConnectOptions;
    launchOptions?: MobileAndroidLaunchOptions;
  };
  configFile?: string;
  suite?: string;
}

export interface AllwrightVitestFixtures {
  browser: Browser;
  page: Page;
  android: MobileAndroidDevice;
  androidApp: MobileAndroidApp;
}

const DEFAULT_ANDROID_CONNECT_TIMEOUT_MS = 30_000;
const DEFAULT_ANDROID_LAUNCH_TIMEOUT_MS = 60_000;

type AllwrightVitestContext = AllwrightVitestFixtures & {
  allwright: AllwrightVitestOptions;
  _browserResource: LazyResource<Browser>;
  _androidResource: LazyResource<MobileAndroidDevice>;
};

type AsyncFactory<T> = () => Promise<T>;

interface LazyResource<T> {
  peek(): T | null;
  get(): Promise<T>;
  realized(): boolean;
}

function createLazyResource<T>(factory: AsyncFactory<T>): LazyResource<T> {
  let value: T | null = null;
  let promise: Promise<T> | null = null;

  return {
    peek() {
      return value;
    },
    get() {
      if (value) {
        return Promise.resolve(value);
      }
      if (!promise) {
        promise = factory().then((resolved) => {
          value = resolved;
          return resolved;
        });
      }
      return promise;
    },
    realized() {
      return value !== null;
    },
  };
}

function getLazySyncProperty<T extends object, K extends keyof T>(
  resource: LazyResource<T>,
  property: K,
): T[K] {
  const value = resource.peek();
  if (value) {
    return value[property];
  }
  throw new Error(
    `fixture property ${String(property)} is not available before the lazy resource is initialized; use an async method first`,
  );
}

const lazyLocatorSources = new WeakMap<Locator, { page: LazyResource<Page>; selector: () => Promise<string> }>();
async function resolveFilterLocator(locator: Locator | undefined, page: LazyResource<Page>): Promise<Locator | undefined> {
  if (!locator) return undefined;
  const source = lazyLocatorSources.get(locator);
  if (!source) return locator;
  if (source.page !== page) throw new Error('Filter locators must belong to the same page');
  return (await page.get()).locator(await source.selector());
}

function createLazyLocator(
  pageResource: LazyResource<Page>,
  selectorFactory: () => Promise<string>,
): Locator {
  const lazyLocator = {
    get page() {
      return createLazyPage(pageResource);
    },
    get selector(): string {
      throw new Error("lazy locator selector is not available before the page fixture is initialized");
    },
    async frame(options?: CommandOptions) {
      return (await pageResource.get()).locator(await selectorFactory()).frame(options);
    },
    async Frame(options?: CommandOptions) {
      return (await pageResource.get()).locator(await selectorFactory()).Frame(options);
    },
    async click(options?: Parameters<Locator["click"]>[0]) {
      return (await pageResource.get()).locator(await selectorFactory()).click(options);
    },
    async count(options?: Parameters<Locator["count"]>[0]) {
      return (await pageResource.get()).locator(await selectorFactory()).count(options);
    },
    async highlight(options?: Parameters<Locator["highlight"]>[0]) {
      return (await pageResource.get()).locator(await selectorFactory()).highlight(options);
    },
    async focus(options?: Parameters<Locator["focus"]>[0]) {
      return (await pageResource.get()).locator(await selectorFactory()).focus(options);
    },
    async fill(value: string, options?: Parameters<Locator["fill"]>[1]) {
      return (await pageResource.get()).locator(await selectorFactory()).fill(value, options);
    },
    async hover(options?: Parameters<Locator["hover"]>[0]) {
      return (await pageResource.get()).locator(await selectorFactory()).hover(options);
    },
    async press(key: string, options?: Parameters<Locator["press"]>[1]) {
      return (await pageResource.get()).locator(await selectorFactory()).press(key, options);
    },
    async textContent(options?: Parameters<Locator["textContent"]>[0]) {
      return (await pageResource.get()).locator(await selectorFactory()).textContent(options);
    },
    async innerText(options?: Parameters<Locator["innerText"]>[0]) {
      return (await pageResource.get()).locator(await selectorFactory()).innerText(options);
    },
    async waitFor(options?: Parameters<Locator["waitFor"]>[0]) {
      return (await pageResource.get()).locator(await selectorFactory()).waitFor(options);
    },
    locator(selector: string, options?: Parameters<Locator["filter"]>[0]) {
      const result = createLazyLocator(pageResource, async () =>
        (await pageResource.get()).locator(await selectorFactory()).locator(selector).selector,
      );
      return options ? result.filter(options) : result;
    },
    getByRole(...args: Parameters<Locator["getByRole"]>) {
      return createLazyLocator(pageResource, async () => (await pageResource.get()).locator(await selectorFactory()).getByRole(...args).selector);
    },
    getByText(...args: Parameters<Locator["getByText"]>) {
      return createLazyLocator(pageResource, async () => (await pageResource.get()).locator(await selectorFactory()).getByText(...args).selector);
    },
    getByLabel(...args: Parameters<Locator["getByLabel"]>) {
      return createLazyLocator(pageResource, async () => (await pageResource.get()).locator(await selectorFactory()).getByLabel(...args).selector);
    },
    getByPlaceholder(...args: Parameters<Locator["getByPlaceholder"]>) {
      return createLazyLocator(pageResource, async () => (await pageResource.get()).locator(await selectorFactory()).getByPlaceholder(...args).selector);
    },
    getByAltText(...args: Parameters<Locator["getByAltText"]>) {
      return createLazyLocator(pageResource, async () => (await pageResource.get()).locator(await selectorFactory()).getByAltText(...args).selector);
    },
    getByTitle(...args: Parameters<Locator["getByTitle"]>) {
      return createLazyLocator(pageResource, async () => (await pageResource.get()).locator(await selectorFactory()).getByTitle(...args).selector);
    },
    getByTestId(...args: Parameters<Locator["getByTestId"]>) {
      return createLazyLocator(pageResource, async () => (await pageResource.get()).locator(await selectorFactory()).getByTestId(...args).selector);
    },
    not(other: Locator): Locator {
      return createLazyLocator(pageResource, async () =>
        (await pageResource.get()).locator(await selectorFactory()).not((await resolveFilterLocator(other, pageResource))!).selector);
    },
    filter(options: Parameters<Locator["filter"]>[0] = {}) {
      return createLazyLocator(pageResource, async () =>
        (await pageResource.get()).locator(await selectorFactory()).filter({ ...options,
          has: await resolveFilterLocator(options.has, pageResource),
          hasNot: await resolveFilterLocator(options.hasNot, pageResource),
        }).selector);
    },
    nth(index: number): Locator { return createLazyLocator(pageResource, async () => (await pageResource.get()).locator(await selectorFactory()).nth(index).selector); },
    first(): Locator { return this.nth(0); },
    last(): Locator { return this.nth(-1); },
  } satisfies Locator;

  lazyLocatorSources.set(lazyLocator, { page: pageResource, selector: selectorFactory });
  return lazyLocator;
}

function createLazyPage(pageResource: LazyResource<Page>): Page {
  const lazyPage = {
    get sessionId() {
      return getLazySyncProperty(pageResource, "sessionId");
    },
    get browserSessionId() {
      return getLazySyncProperty(pageResource, "browserSessionId");
    },
    locator(selector: string, options?: Parameters<Locator["filter"]>[0]) {
      const result = createLazyLocator(pageResource, async () => selector);
      return options ? result.filter(options) : result;
    },
    async frame(selector: string, options?: CommandOptions) {
      return (await pageResource.get()).frame(selector, options);
    },
    async goto(url: string, options?: CommandOptions) {
      return (await pageResource.get()).goto(url, options);
    },
    async navigate(url: string, options?: CommandOptions) {
      return (await pageResource.get()).navigate(url, options);
    },
    async click(selector: string, options?: CommandOptions) {
      return (await pageResource.get()).click(selector, options);
    },
    async count(selector: string, options?: CommandOptions) {
      return (await pageResource.get()).count(selector, options);
    },
    async highlight(selector: string, options?: Parameters<Page["highlight"]>[1]) {
      return (await pageResource.get()).highlight(selector, options);
    },
    async focus(selector: string, options?: CommandOptions) {
      return (await pageResource.get()).focus(selector, options);
    },
    async fill(selector: string, value: string, options?: CommandOptions) {
      return (await pageResource.get()).fill(selector, value, options);
    },
    async hover(selector: string, options?: CommandOptions) {
      return (await pageResource.get()).hover(selector, options);
    },
    async press(selector: string, key: string, options?: Parameters<Page["press"]>[2]) {
      return (await pageResource.get()).press(selector, key, options);
    },
    async textContent(selector: string, options?: CommandOptions) {
      return (await pageResource.get()).textContent(selector, options);
    },
    async innerText(selector: string, options?: CommandOptions) {
      return (await pageResource.get()).innerText(selector, options);
    },
    async waitForSelector(selector: string, options?: WaitForSelectorOptions) {
      return (await pageResource.get()).waitForSelector(selector, options);
    },
    async accessibilitySnapshot(options?: Parameters<Page["accessibilitySnapshot"]>[0]) {
      return (await pageResource.get()).accessibilitySnapshot(options);
    },
    async screenshot(options?: ScreenshotOptions) {
      return (await pageResource.get()).screenshot(options);
    },
    async registerHook<T>(type: HookType<T>): Promise<Hook<T>> {
      return (await pageResource.get()).registerHook(type);
    },
    async close() {
      await (await pageResource.get()).close();
    },
    async ping(message?: string) {
      return (await pageResource.get()).ping(message);
    },
    pageInfo() {
      const page = pageResource.peek();
      if (page) {
        return page.pageInfo();
      }
      return {
        sessionId: getLazySyncProperty(pageResource, "sessionId"),
        browserSessionId: getLazySyncProperty(pageResource, "browserSessionId"),
      };
    },
    getByRole(...args: Parameters<Page["getByRole"]>) {
      return createLazyLocator(pageResource, async () => (await pageResource.get()).getByRole(...args).selector);
    },
    getByText(...args: Parameters<Page["getByText"]>) {
      return createLazyLocator(pageResource, async () => (await pageResource.get()).getByText(...args).selector);
    },
    getByLabel(...args: Parameters<Page["getByLabel"]>) {
      return createLazyLocator(pageResource, async () => (await pageResource.get()).getByLabel(...args).selector);
    },
    getByPlaceholder(...args: Parameters<Page["getByPlaceholder"]>) {
      return createLazyLocator(pageResource, async () => (await pageResource.get()).getByPlaceholder(...args).selector);
    },
    getByAltText(...args: Parameters<Page["getByAltText"]>) {
      return createLazyLocator(pageResource, async () => (await pageResource.get()).getByAltText(...args).selector);
    },
    getByTitle(...args: Parameters<Page["getByTitle"]>) {
      return createLazyLocator(pageResource, async () => (await pageResource.get()).getByTitle(...args).selector);
    },
    getByTestId(...args: Parameters<Page["getByTestId"]>) {
      return createLazyLocator(pageResource, async () => (await pageResource.get()).getByTestId(...args).selector);
    },
  } satisfies Page;

  return lazyPage;
}

function createLazyBrowser(browserResource: LazyResource<Browser>): Browser {
  const lazyBrowser = {
    get sessionId() {
      return getLazySyncProperty(browserResource, "sessionId");
    },
    get browserName() {
      return getLazySyncProperty(browserResource, "browserName");
    },
    get launchNote() {
      return getLazySyncProperty(browserResource, "launchNote");
    },
    get cdpWebSocketURL() {
      return getLazySyncProperty(browserResource, "cdpWebSocketURL");
    },
    get userDataDir() {
      return getLazySyncProperty(browserResource, "userDataDir");
    },
    page() {
      return createLazyPage(createLazyResource(async () => (await browserResource.get()).page()));
    },
    initialPage() {
      return createLazyPage(createLazyResource(async () => (await browserResource.get()).initialPage()));
    },
    initialTab() {
      return createLazyPage(createLazyResource(async () => (await browserResource.get()).initialTab()));
    },
    pages() {
      const browser = browserResource.peek();
      if (!browser) {
        return [this.page()];
      }
      return browser.pages();
    },
    async newPage(options?: CommandOptions) {
      return (await browserResource.get()).newPage(options);
    },
    async newTab(options?: CommandOptions) {
      return (await browserResource.get()).newTab(options);
    },
    async close() {
      await (await browserResource.get()).close();
    },
    async ping(message?: string) {
      return (await browserResource.get()).ping(message);
    },
    browserInfo() {
      const browser = browserResource.peek();
      if (browser) {
        return browser.browserInfo();
      }
      return {
        sessionId: getLazySyncProperty(browserResource, "sessionId"),
        browserName: getLazySyncProperty(browserResource, "browserName"),
        launchNote: getLazySyncProperty(browserResource, "launchNote"),
        cdpWebSocketURL: getLazySyncProperty(browserResource, "cdpWebSocketURL"),
        userDataDir: getLazySyncProperty(browserResource, "userDataDir"),
      };
    },
  } satisfies Browser;

  return lazyBrowser;
}

function createLazyAndroidApp(appResource: LazyResource<MobileAndroidApp>): MobileAndroidApp {
  const lazyApp = {
    get sessionId() {
      return getLazySyncProperty(appResource, "sessionId");
    },
    locator(selector: string) {
      return createLazyAndroidLocator(appResource, async () => selector);
    },
    async registerHook<T>(type: HookType<T>): Promise<Hook<T>> {
      return (await appResource.get()).registerHook(type);
    },
    async click(selector: string, options?: CommandOptions) {
      return (await appResource.get()).click(selector, options);
    },
    async count(selector: string, options?: CommandOptions) {
      return (await appResource.get()).count(selector, options);
    },
    async focus(selector: string, options?: CommandOptions) {
      return (await appResource.get()).focus(selector, options);
    },
    async fill(selector: string, value: string, options?: CommandOptions) {
      return (await appResource.get()).fill(selector, value, options);
    },
    async press(selector: string, key: string, options?: Parameters<MobileAndroidApp["press"]>[2]) {
      return (await appResource.get()).press(selector, key, options);
    },
    async textContent(selector: string, options?: CommandOptions) {
      return (await appResource.get()).textContent(selector, options);
    },
    async innerText(selector: string, options?: CommandOptions) {
      return (await appResource.get()).innerText(selector, options);
    },
    async waitForSelector(selector: string, options?: WaitForSelectorOptions) {
      return (await appResource.get()).waitForSelector(selector, options);
    },
    async accessibilitySnapshot(options?: Parameters<MobileAndroidApp["accessibilitySnapshot"]>[0]) {
      return (await appResource.get()).accessibilitySnapshot(options);
    },
    async screenshot(options?: ScreenshotOptions) {
      return (await appResource.get()).screenshot(options);
    },
  } satisfies MobileAndroidApp;

  return lazyApp;
}

function createLazyAndroidLocator(
  appResource: LazyResource<MobileAndroidApp>,
  selectorFactory: () => Promise<string>,
): MobileAndroidLocator {
  const lazyLocator = {
    get page() {
      return createLazyAndroidApp(appResource);
    },
    get selector(): string {
      throw new Error("lazy Android locator selector is not available before the app fixture is initialized");
    },
    async click(options?: CommandOptions) {
      return (await appResource.get()).locator(await selectorFactory()).click(options);
    },
    async count(options?: Parameters<MobileAndroidLocator["count"]>[0]) {
      return (await appResource.get()).locator(await selectorFactory()).count(options);
    },
    async focus(options?: Parameters<MobileAndroidLocator["focus"]>[0]) {
      return (await appResource.get()).locator(await selectorFactory()).focus(options);
    },
    async fill(value: string, options?: CommandOptions) {
      return (await appResource.get()).locator(await selectorFactory()).fill(value, options);
    },
    async press(key: string, options?: Parameters<MobileAndroidLocator["press"]>[1]) {
      return (await appResource.get()).locator(await selectorFactory()).press(key, options);
    },
    async textContent(options?: Parameters<MobileAndroidLocator["textContent"]>[0]) {
      return (await appResource.get()).locator(await selectorFactory()).textContent(options);
    },
    async innerText(options?: Parameters<MobileAndroidLocator["innerText"]>[0]) {
      return (await appResource.get()).locator(await selectorFactory()).innerText(options);
    },
    async waitFor(options?: Parameters<MobileAndroidLocator["waitFor"]>[0]) {
      return (await appResource.get()).locator(await selectorFactory()).waitFor(options);
    },
    locator(selector: string) {
      return createLazyAndroidLocator(appResource, async () =>
        (await appResource.get()).locator(await selectorFactory()).locator(selector).selector,
      );
    },
  } satisfies MobileAndroidLocator;

  return lazyLocator;
}

function createLazyAndroidDevice(
  deviceResource: LazyResource<MobileAndroidDevice>,
): MobileAndroidDevice {
  const initialAppResource = createLazyResource(async () => (await deviceResource.get()).app());

  const lazyDevice = {
    get sessionId() {
      return getLazySyncProperty(deviceResource, "sessionId");
    },
    app() {
      return createLazyAndroidApp(initialAppResource);
    },
    initialApp() {
      return createLazyAndroidApp(initialAppResource);
    },
    async launch(options?: MobileAndroidLaunchOptions) {
      return (await deviceResource.get()).launch(options);
    },
  } satisfies MobileAndroidDevice;

  return lazyDevice;
}

export const test = base.extend<AllwrightVitestContext>({
  allwright: async ({}, use) => {
    await use({});
  },

  _browserResource: async ({ allwright }, use) => {
    const config = resolveVitestConfig(allwright);

    if (config.serverAddr) {
      setServerAddr(config.serverAddr);
    }

    activeExpectDefaults = config.expect;

    const browserResource = createLazyResource(async () => launchConfiguredBrowser(config));

    try {
      await use(browserResource);
    } finally {
      activeExpectDefaults = {};
      if (browserResource.realized()) {
        await browserResource.get().then((resolved) => resolved.close());
      }
    }
  },

  _androidResource: async ({ allwright }, use) => {
    const config = resolveVitestConfig(allwright);
    if (config.serverAddr) {
      setServerAddr(config.serverAddr);
    }

    const androidResource = createLazyResource(async () =>
      mobile.android.connect(resolveAndroidConnectOptions(config, allwright)),
    );

    await use(androidResource);
  },

  browser: async ({ _browserResource }, use) => {
    try {
      await use(createLazyBrowser(_browserResource));
    } finally {
      // resource lifecycle is owned by _browserResource
    }
  },

  page: async ({ browser }, use) => {
    await use(browser.page());
  },

  android: async ({ _androidResource }, use) => {
    await use(createLazyAndroidDevice(_androidResource));
  },

  androidApp: async ({ _androidResource, allwright }, use) => {
    const appResource = createLazyResource(async () => {
      const config = resolveVitestConfig(allwright);
      const launchOptions = resolveAndroidLaunchOptions(config, allwright);
      return (await _androidResource.get()).launch(launchOptions);
    });

    await use(createLazyAndroidApp(appResource));
  },
});

function isPage(value: unknown): value is Page {
  return !!value && typeof value === "object" && typeof (value as Page).locator === "function";
}

function isLocator(value: unknown): value is Locator {
  return (
    !!value &&
    typeof value === "object" &&
    typeof (value as Locator).page === "object" &&
    typeof (value as Locator).click === "function"
  );
}

function resolveVitestConfig(options: AllwrightVitestOptions): ResolvedAllwrightConfig {
  const config = resolveConfig({
    configFile: options.configFile,
    suite: options.suite,
  });

  return {
    ...config,
    serverAddr: options.serverAddr ?? config.serverAddr,
    browserName: options.browser ?? config.browserName,
    browserBinary: options.browserBinary ?? config.browserBinary,
    launchOptions: {
      ...config.launchOptions,
      ...(options.launchOptions ?? {}),
      browserBinary:
        options.launchOptions?.browserBinary ??
        options.browserBinary ??
        config.browserBinary ??
        config.launchOptions.browserBinary,
    },
    expect: config.expect,
  };
}

function resolveAndroidConnectOptions(
  config: ResolvedAllwrightConfig,
  options: AllwrightVitestOptions,
): MobileAndroidConnectOptions {
  return {
    device: options.android?.connectOptions?.device ?? config.mobile.android?.device,
    adbEndpoint: options.android?.connectOptions?.adbEndpoint,
    preserveAppState: options.android?.connectOptions?.preserveAppState ?? false,
    timeoutMs:
      options.android?.connectOptions?.timeoutMs ??
      DEFAULT_ANDROID_CONNECT_TIMEOUT_MS,
  };
}

function resolveAndroidLaunchOptions(
  config: ResolvedAllwrightConfig,
  options: AllwrightVitestOptions,
): MobileAndroidLaunchOptions {
  const launchOptions: MobileAndroidLaunchOptions = {
    apkPath: options.android?.launchOptions?.apkPath ?? config.mobile.android?.appBinary,
    appId: options.android?.launchOptions?.appId ?? config.mobile.android?.appId,
    launchActivity:
      options.android?.launchOptions?.launchActivity ?? config.mobile.android?.appActivity,
    stopBeforeLaunch: options.android?.launchOptions?.stopBeforeLaunch ?? false,
    timeoutMs:
      options.android?.launchOptions?.timeoutMs ??
      DEFAULT_ANDROID_LAUNCH_TIMEOUT_MS,
  };

  if (!launchOptions.apkPath && !launchOptions.appId) {
    throw new Error(
      "androidApp fixture requires android launch options with `apkPath` or `appId`, or config.mobile.android.app configured",
    );
  }

  return launchOptions;
}

let activeExpectDefaults: RetryExpectationOptions = {};

function currentExpectDefaults(): RetryExpectationOptions {
  return activeExpectDefaults;
}

type VitestExpect = typeof vitestExpect;

interface AllwrightExpect extends VitestExpect {
  (actual: Page): PageExpectMatchers;
  (actual: Locator): LocatorExpectMatchers;
  (actual: MobileAndroidApp): PageExpectMatchers;
  (actual: MobileAndroidLocator): LocatorExpectMatchers;
  <T>(actual: T): Assertion<T>;
}

const expectImpl = ((actual: unknown) => {
  if (isLocator(actual)) {
    return createLocatorExpect(actual, currentExpectDefaults);
  }
  if (isPage(actual)) {
    return createPageExpect(actual, currentExpectDefaults);
  }
  return vitestExpect(actual);
}) as AllwrightExpect;

const vitestExpectDescriptors = Object.getOwnPropertyDescriptors(vitestExpect);
for (const [key, descriptor] of Object.entries(vitestExpectDescriptors)) {
  Object.defineProperty(expectImpl, key, descriptor);
}

export const expect = expectImpl;
export { vitestExpect };

export type { Browser, LaunchOptions, Locator, Page };
