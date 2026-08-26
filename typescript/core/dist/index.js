import path from "node:path";
import fs from "node:fs";
import { fileURLToPath } from "node:url";
import grpc from "@grpc/grpc-js";
import protoLoader from "@grpc/proto-loader";
const DEFAULT_SERVER_ADDR = "127.0.0.1:50051";
const SERVER_ADDR_ENV_VAR = "ALLWRIGHT_SERVER_ADDR";
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const PACKAGE_ROOT = path.resolve(__dirname, "..");
const PROTO_ROOT = path.join(PACKAGE_ROOT, "proto");
const ENGINE_PROTO_PATH = path.join(PROTO_ROOT, "engine", "v1", "engine.proto");
let runtimePromise = null;
let serverAddrOverride = null;
const CONFIG_FILENAMES = [
    "allwright.config.yaml",
    "allwright.config.yml",
    "allwright.config.json",
    ".allwright/config.yaml",
    ".allwright/config.yml",
    ".allwright/config.json",
];
class EventQueue {
    #items = [];
    #waiters = [];
    #endedError = null;
    push(item) {
        const waiter = this.#waiters.shift();
        if (waiter) {
            waiter.resolve(item);
            return;
        }
        this.#items.push(item);
    }
    fail(error) {
        if (this.#endedError) {
            return;
        }
        this.#endedError = error;
        while (this.#waiters.length > 0) {
            this.#waiters.shift().reject(error);
        }
    }
    async next() {
        if (this.#items.length > 0) {
            return this.#items.shift();
        }
        if (this.#endedError) {
            throw this.#endedError;
        }
        return new Promise((resolve, reject) => {
            this.#waiters.push({ resolve, reject });
        });
    }
}
class BrowserTypeImpl {
    #browserKind;
    constructor(browserKind = "chromium") {
        this.#browserKind = browserKind;
    }
    async launch(options = {}) {
        return launchBrowser(this.#browserKind, options);
    }
}
class BrowserImpl {
    #closed = false;
    #runtime;
    #stream;
    #queue;
    #pages = new Map();
    #initialPage;
    constructor(state) {
        const browserInfo = {
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
        this.#initialPage = this.#createPage(state.launched.initialTabSessionId ?? "");
    }
    sessionId;
    browserName;
    launchNote;
    cdpWebSocketURL;
    userDataDir;
    page() {
        return this.#initialPage;
    }
    initialPage() {
        return this.#initialPage;
    }
    pages() {
        return [...this.#pages.values()];
    }
    async newPage(options = {}) {
        this.#ensureOpen();
        this.#stream.write({
            openTab: {
                retryOptions: options.timeoutMs ? { timeoutMs: options.timeoutMs } : undefined,
            },
        });
        while (true) {
            const event = await this.#queue.next();
            if (event.tabOpened?.tabSessionId) {
                return this.#createPage(event.tabOpened.tabSessionId);
            }
            if (event.error?.message) {
                throw new Error(`browser session error while opening tab: ${event.error.message}`);
            }
        }
    }
    async close() {
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
                this.#stream.end();
                return;
            }
            if (event.error?.message) {
                throw new Error(`browser session error while closing: ${event.error.message}`);
            }
        }
    }
    async ping(message = "ping") {
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
                throw new Error(`browser session error while pinging: ${event.error.message}`);
            }
        }
    }
    browserInfo() {
        return {
            sessionId: this.sessionId,
            browserName: this.browserName,
            launchNote: this.launchNote,
            cdpWebSocketURL: this.cdpWebSocketURL,
            userDataDir: this.userDataDir,
        };
    }
    initialTab() {
        return this.initialPage();
    }
    async newTab(options = {}) {
        return this.newPage(options);
    }
    #createPage(sessionId) {
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
    #ensureOpen() {
        if (this.#closed) {
            throw new Error(`browser session ${this.sessionId} is closed`);
        }
    }
}
class PageImpl {
    #runtime;
    #handlePromise = null;
    constructor(input) {
        this.#runtime = input.runtime;
        this.sessionId = input.sessionId;
        this.browserSessionId = input.browserSessionId;
    }
    sessionId;
    browserSessionId;
    locator(selector) {
        return new LocatorImpl({ page: this, selector });
    }
    async goto(url, options = {}) {
        const handle = await this.#getHandle();
        this.#ensureOpen(handle);
        handle.stream.write({
            browserSessionId: this.browserSessionId,
            tabSessionId: this.sessionId,
            navigate: {
                url,
                retryOptions: options.timeoutMs ? { timeoutMs: options.timeoutMs } : undefined,
            },
        });
        let navigated = null;
        let injection = null;
        while (true) {
            const event = await handle.queue.next();
            if (event.navigated) {
                navigated = event.navigated;
            }
            if (event.chromiumBidiInjection) {
                injection = event.chromiumBidiInjection;
            }
            if (event.error?.message) {
                throw new Error(`page session error while navigating: ${event.error.message}`);
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
    async click(selector, options = {}) {
        const handle = await this.#getHandle();
        this.#ensureOpen(handle);
        handle.stream.write({
            browserSessionId: this.browserSessionId,
            tabSessionId: this.sessionId,
            clickElement: {
                cssSelector: selector,
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
                throw new Error(`page session error while clicking: ${event.error.message}`);
            }
            if (event.closed) {
                handle.closed = true;
                throw new Error(`page session ${this.sessionId} closed while waiting for click result`);
            }
        }
    }
    async count(selector, options = {}) {
        const handle = await this.#getHandle();
        this.#ensureOpen(handle);
        handle.stream.write({
            browserSessionId: this.browserSessionId,
            tabSessionId: this.sessionId,
            countElements: {
                cssSelector: selector,
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
                throw new Error(`page session error while counting elements: ${event.error.message}`);
            }
            if (event.closed) {
                handle.closed = true;
                throw new Error(`page session ${this.sessionId} closed while waiting for count result`);
            }
        }
    }
    async highlight(selector, options = {}) {
        const handle = await this.#getHandle();
        this.#ensureOpen(handle);
        handle.stream.write({
            browserSessionId: this.browserSessionId,
            tabSessionId: this.sessionId,
            highlightElements: {
                cssSelector: selector,
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
                throw new Error(`page session error while highlighting elements: ${event.error.message}`);
            }
            if (event.closed) {
                handle.closed = true;
                throw new Error(`page session ${this.sessionId} closed while waiting for highlight result`);
            }
        }
    }
    async focus(selector, options = {}) {
        const handle = await this.#getHandle();
        this.#ensureOpen(handle);
        handle.stream.write({
            browserSessionId: this.browserSessionId,
            tabSessionId: this.sessionId,
            focusElement: {
                cssSelector: selector,
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
                throw new Error(`page session error while focusing: ${event.error.message}`);
            }
            if (event.closed) {
                handle.closed = true;
                throw new Error(`page session ${this.sessionId} closed while waiting for focus result`);
            }
        }
    }
    async fill(selector, value, options = {}) {
        const handle = await this.#getHandle();
        this.#ensureOpen(handle);
        handle.stream.write({
            browserSessionId: this.browserSessionId,
            tabSessionId: this.sessionId,
            fillElement: {
                cssSelector: selector,
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
                throw new Error(`page session error while filling: ${event.error.message}`);
            }
            if (event.closed) {
                handle.closed = true;
                throw new Error(`page session ${this.sessionId} closed while waiting for fill result`);
            }
        }
    }
    async hover(selector, options = {}) {
        const handle = await this.#getHandle();
        this.#ensureOpen(handle);
        handle.stream.write({
            browserSessionId: this.browserSessionId,
            tabSessionId: this.sessionId,
            hoverElement: {
                cssSelector: selector,
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
                throw new Error(`page session error while hovering: ${event.error.message}`);
            }
            if (event.closed) {
                handle.closed = true;
                throw new Error(`page session ${this.sessionId} closed while waiting for hover result`);
            }
        }
    }
    async press(selector, key, options = {}) {
        const handle = await this.#getHandle();
        this.#ensureOpen(handle);
        handle.stream.write({
            browserSessionId: this.browserSessionId,
            tabSessionId: this.sessionId,
            pressKey: {
                cssSelector: selector,
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
                throw new Error(`page session error while pressing key: ${event.error.message}`);
            }
            if (event.closed) {
                handle.closed = true;
                throw new Error(`page session ${this.sessionId} closed while waiting for press result`);
            }
        }
    }
    async textContent(selector, options = {}) {
        return this.#readText(selector, options, true);
    }
    async innerText(selector, options = {}) {
        return this.#readText(selector, options, false);
    }
    async waitForSelector(selector, options = {}) {
        const handle = await this.#getHandle();
        this.#ensureOpen(handle);
        handle.stream.write({
            browserSessionId: this.browserSessionId,
            tabSessionId: this.sessionId,
            waitForSelector: {
                cssSelector: selector,
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
                throw new Error(`page session error while waiting for selector: ${event.error.message}`);
            }
            if (event.closed) {
                handle.closed = true;
                throw new Error(`page session ${this.sessionId} closed while waiting for selector result`);
            }
        }
    }
    async close() {
        const handle = await this.#getHandle();
        if (handle.closed) {
            return;
        }
        handle.stream.write({
            browserSessionId: this.browserSessionId,
            tabSessionId: this.sessionId,
            close: {},
        });
        while (true) {
            const event = await handle.queue.next();
            if (event.closed) {
                handle.closed = true;
                handle.stream.end();
                return;
            }
            if (event.error?.message) {
                throw new Error(`page session error while closing: ${event.error.message}`);
            }
        }
    }
    async ping(message = "ping") {
        const handle = await this.#getHandle();
        this.#ensureOpen(handle);
        handle.stream.write({
            browserSessionId: this.browserSessionId,
            tabSessionId: this.sessionId,
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
                throw new Error(`page session error while pinging: ${event.error.message}`);
            }
            if (event.closed) {
                handle.closed = true;
                throw new Error(`page session ${this.sessionId} closed while waiting for pong`);
            }
        }
    }
    pageInfo() {
        return {
            sessionId: this.sessionId,
            browserSessionId: this.browserSessionId,
        };
    }
    async navigate(url, options = {}) {
        return this.goto(url, options);
    }
    async #readText(selector, options, textContent) {
        const handle = await this.#getHandle();
        this.#ensureOpen(handle);
        handle.stream.write(textContent
            ? {
                browserSessionId: this.browserSessionId,
                tabSessionId: this.sessionId,
                getTextContent: {
                    cssSelector: selector,
                    retryOptions: options.timeoutMs ? { timeoutMs: options.timeoutMs } : undefined,
                },
            }
            : {
                browserSessionId: this.browserSessionId,
                tabSessionId: this.sessionId,
                getInnerText: {
                    cssSelector: selector,
                    retryOptions: options.timeoutMs ? { timeoutMs: options.timeoutMs } : undefined,
                },
            });
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
                throw new Error(`page session error while reading text: ${event.error.message}`);
            }
            if (event.closed) {
                handle.closed = true;
                throw new Error(`page session ${this.sessionId} closed while waiting for text result`);
            }
        }
    }
    #ensureOpen(handle) {
        if (handle.closed) {
            throw new Error(`page session ${this.sessionId} is closed`);
        }
    }
    async #getHandle() {
        if (!this.#handlePromise) {
            this.#handlePromise = createPageHandle(this.#runtime);
        }
        return this.#handlePromise;
    }
}
class LocatorImpl {
    page;
    selector;
    constructor(input) {
        this.page = input.page;
        this.selector = input.selector;
    }
    async click(options = {}) {
        return this.page.click(this.selector, options);
    }
    async count(options = {}) {
        return this.page.count(this.selector, options);
    }
    async highlight(options = {}) {
        return this.page.highlight(this.selector, options);
    }
    async focus(options = {}) {
        return this.page.focus(this.selector, options);
    }
    async fill(value, options = {}) {
        return this.page.fill(this.selector, value, options);
    }
    async hover(options = {}) {
        return this.page.hover(this.selector, options);
    }
    async press(key, options = {}) {
        return this.page.press(this.selector, key, options);
    }
    async textContent(options = {}) {
        return this.page.textContent(this.selector, options);
    }
    async innerText(options = {}) {
        return this.page.innerText(this.selector, options);
    }
    async waitFor(options = {}) {
        return this.page.waitForSelector(this.selector, options);
    }
    locator(selector) {
        return new LocatorImpl({
            page: this.page,
            selector: `${this.selector} ${selector}`,
        });
    }
}
export const chromium = new BrowserTypeImpl("chromium");
export const firefox = new BrowserTypeImpl("firefox");
export async function ping() {
    const runtime = await getRuntime();
    return new Promise((resolve, reject) => {
        runtime.client.Ping({}, (error, response) => {
            if (error) {
                reject(new Error(`ping engine server: ${error.message}`));
                return;
            }
            resolve(response.message ?? "");
        });
    });
}
export async function launchChrome(options = {}) {
    return launchBrowser("chromium", options);
}
export async function launchConfiguredBrowser(config) {
    return launchBrowser(config.browserName, {
        ...config.launchOptions,
        browserBinary: config.browserBinary ?? config.launchOptions.browserBinary,
    });
}
export async function launchBrowser(browserKind, options = {}) {
    const runtime = await getRuntime();
    const stream = runtime.client.BrowserSession();
    const queue = bindStreamQueue(stream);
    stream.write({
        launchBrowser: {
            browserKind: browserKind === "firefox" ? 2 : 1,
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
            throw new Error(`browser session error during launch: ${event.error.message}`);
        }
    }
}
export function setServerAddr(serverAddr) {
    serverAddrOverride = normalizeServerAddr(serverAddr);
    runtimePromise = null;
}
export async function shutdown() {
    if (!runtimePromise) {
        return;
    }
    const runtime = await runtimePromise;
    runtime.client.close();
    runtimePromise = null;
}
export function findConfigFile(startDir = process.cwd()) {
    let currentDir = path.resolve(startDir);
    while (true) {
        for (const filename of CONFIG_FILENAMES) {
            const candidate = path.join(currentDir, filename);
            if (fs.existsSync(candidate) && fs.statSync(candidate).isFile()) {
                return candidate;
            }
        }
        const parentDir = path.dirname(currentDir);
        if (parentDir === currentDir) {
            return null;
        }
        currentDir = parentDir;
    }
}
export function loadConfigFile(configFile) {
    const resolved = path.resolve(configFile);
    const raw = fs.readFileSync(resolved, "utf8");
    const parsed = parseConfigContents(raw, resolved);
    validateConfigShape(parsed, resolved);
    return parsed;
}
export function resolveConfig(options = {}) {
    const configFilePath = options.configFile ? path.resolve(options.configFile) : findConfigFile(options.cwd);
    const fileConfig = configFilePath ? loadConfigFile(configFilePath) : {};
    const suiteName = options.suite?.trim() || null;
    const suiteConfig = suiteName ? fileConfig.suites?.[suiteName] : undefined;
    if (suiteName && !suiteConfig) {
        throw new Error(`allwright config suite "${suiteName}" was not found in ${configFilePath ?? "the resolved config file"}`);
    }
    const serverAddr = suiteConfig?.server?.addr ?? fileConfig.server?.addr;
    const browserName = suiteConfig?.browser?.name ?? fileConfig.browser?.name ?? "chromium";
    const browserBinary = suiteConfig?.browser?.binary ?? fileConfig.browser?.binary;
    const launchOptions = mergeLaunchOptions(fileConfig.browser?.launchOptions, suiteConfig?.browser?.launchOptions);
    const expect = {
        ...(fileConfig.expect ?? {}),
        ...(suiteConfig?.expect ?? {}),
    };
    return {
        configFilePath,
        suiteName,
        serverAddr,
        browserName,
        browserBinary,
        launchOptions: browserBinary ? { ...launchOptions, browserBinary } : launchOptions,
        expect,
    };
}
async function getRuntime() {
    if (!runtimePromise) {
        runtimePromise = Promise.resolve(createRuntime());
    }
    return runtimePromise;
}
function createRuntime() {
    const loaded = protoLoader.loadSync(ENGINE_PROTO_PATH, {
        includeDirs: [PROTO_ROOT],
        keepCase: false,
        longs: String,
        enums: String,
        defaults: true,
        oneofs: true,
    });
    const proto = grpc.loadPackageDefinition(loaded);
    const ClientCtor = proto.allwright.engine.v1.EngineService;
    const client = new ClientCtor(configuredServerAddr(), grpc.credentials.createInsecure());
    return { client };
}
function configuredServerAddr() {
    if (serverAddrOverride) {
        return serverAddrOverride;
    }
    return normalizeServerAddr(process.env[SERVER_ADDR_ENV_VAR] ?? DEFAULT_SERVER_ADDR);
}
function mergeLaunchOptions(base, override) {
    return {
        ...(base ?? {}),
        ...(override ?? {}),
    };
}
function validateConfigShape(value, source) {
    if (!value || typeof value !== "object" || Array.isArray(value)) {
        throw new Error(`allwright config ${source} must contain a top-level object`);
    }
    const config = value;
    if (config.schemaVersion !== undefined && config.schemaVersion !== 1) {
        throw new Error(`allwright config ${source} has unsupported schemaVersion ${String(config.schemaVersion)}; expected 1`);
    }
    const browserName = config.browser?.name;
    if (browserName !== undefined && browserName !== "chromium" && browserName !== "firefox") {
        throw new Error(`allwright config ${source} has unsupported browser.name ${String(browserName)}; use "chromium" or "firefox"`);
    }
}
function parseConfigContents(raw, source) {
    const extension = path.extname(source).toLowerCase();
    if (extension === ".json") {
        return JSON.parse(raw);
    }
    if (extension === ".yaml" || extension === ".yml") {
        return parseSimpleYaml(raw, source);
    }
    throw new Error(`unsupported allwright config file extension ${extension || "<none>"} for ${source}`);
}
function parseSimpleYaml(raw, source) {
    const root = {};
    const stack = [
        { indent: -1, value: root },
    ];
    for (const [index, originalLine] of raw.split(/\r?\n/).entries()) {
        const lineNumber = index + 1;
        const line = stripYamlComment(originalLine);
        if (!line.trim()) {
            continue;
        }
        const indent = countLeadingSpaces(line);
        if (indent % 2 !== 0) {
            throw new Error(`invalid YAML indentation in ${source}:${lineNumber}; use multiples of 2 spaces`);
        }
        while (stack.length > 1 && indent <= stack[stack.length - 1].indent) {
            stack.pop();
        }
        const current = stack[stack.length - 1];
        const trimmed = line.trim();
        const separatorIndex = trimmed.indexOf(":");
        if (separatorIndex <= 0) {
            throw new Error(`invalid YAML mapping in ${source}:${lineNumber}`);
        }
        const key = trimmed.slice(0, separatorIndex).trim();
        const rawValue = trimmed.slice(separatorIndex + 1).trim();
        if (!key) {
            throw new Error(`empty YAML key in ${source}:${lineNumber}`);
        }
        if (!rawValue) {
            const child = {};
            current.value[key] = child;
            stack.push({ indent, value: child });
            continue;
        }
        current.value[key] = parseYamlScalar(rawValue, source, lineNumber);
    }
    return root;
}
function stripYamlComment(line) {
    let inSingleQuote = false;
    let inDoubleQuote = false;
    for (let index = 0; index < line.length; index += 1) {
        const char = line[index];
        if (char === "'" && !inDoubleQuote) {
            inSingleQuote = !inSingleQuote;
            continue;
        }
        if (char === "\"" && !inSingleQuote) {
            inDoubleQuote = !inDoubleQuote;
            continue;
        }
        if (char === "#" && !inSingleQuote && !inDoubleQuote) {
            return line.slice(0, index);
        }
    }
    return line;
}
function countLeadingSpaces(line) {
    let count = 0;
    while (count < line.length && line[count] === " ") {
        count += 1;
    }
    return count;
}
function parseYamlScalar(value, source, lineNumber) {
    if ((value.startsWith("\"") && value.endsWith("\"")) || (value.startsWith("'") && value.endsWith("'"))) {
        return value.slice(1, -1);
    }
    if (value === "true") {
        return true;
    }
    if (value === "false") {
        return false;
    }
    if (value === "null") {
        return null;
    }
    if (/^-?\d+$/.test(value)) {
        return Number.parseInt(value, 10);
    }
    if (/^-?\d+\.\d+$/.test(value)) {
        return Number.parseFloat(value);
    }
    if (value.startsWith("[") || value.startsWith("{")) {
        throw new Error(`unsupported YAML collection syntax in ${source}:${lineNumber}; use nested mappings instead`);
    }
    return value;
}
function normalizeServerAddr(raw) {
    const trimmed = raw.trim();
    if (trimmed.startsWith("dns:") || trimmed.startsWith("unix:")) {
        return trimmed;
    }
    if (trimmed.includes("://")) {
        const parsed = new URL(trimmed);
        return parsed.host;
    }
    return trimmed;
}
function bindStreamQueue(stream) {
    const queue = new EventQueue();
    stream.on("data", (event) => {
        queue.push(event);
    });
    stream.on("error", (error) => {
        queue.fail(new Error(`grpc stream error: ${error.message}`));
    });
    stream.on("end", () => {
        queue.fail(new Error("grpc stream ended"));
    });
    return queue;
}
async function createPageHandle(runtime) {
    const stream = runtime.client.TabSession();
    const queue = bindStreamQueue(stream);
    return {
        stream,
        queue,
        closed: false,
    };
}
