package dev.allwright.client;

import allwright.engine.v1.EngineServiceGrpc;
import allwright.engine.v1.Engine.*;
import io.grpc.ManagedChannel;
import io.grpc.ManagedChannelBuilder;
import io.grpc.StatusRuntimeException;
import io.grpc.stub.StreamObserver;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;
import java.util.Map;
import java.util.LinkedHashMap;
import java.util.Objects;
import java.util.concurrent.BlockingQueue;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.LinkedBlockingQueue;
import org.yaml.snakeyaml.Yaml;

public final class Allwright {
    public static final String DEFAULT_SERVER_ADDR = "127.0.0.1:50051";
    public static final String SERVER_ADDR_ENV_VAR = "ALLWRIGHT_SERVER_ADDR";
    private static final List<String> CONFIG_FILENAMES = List.of(
            "allwright.config.yaml",
            "allwright.config.yml",
            "allwright.config.json",
            ".allwright/config.yaml",
            ".allwright/config.yml",
            ".allwright/config.json"
    );

    private static final Object RUNTIME_LOCK = new Object();
    private static RuntimeClient runtimeClient;
    private static String serverAddrOverride;
    private static final BrowserType CHROMIUM = new BrowserType(BrowserKind.BROWSER_KIND_CHROMIUM);
    private static final BrowserType FIREFOX = new BrowserType(BrowserKind.BROWSER_KIND_FIREFOX);

    private Allwright() {}

    public static BrowserType chromium() {
        return CHROMIUM;
    }

    public static BrowserType firefox() {
        return FIREFOX;
    }

    public static Browser launchChrome() {
        return launchChrome(new LaunchOptions());
    }

    public static Browser launchChrome(LaunchOptions options) {
        return launchBrowser(BrowserKind.BROWSER_KIND_CHROMIUM, options);
    }

    public static Browser launchFirefox() {
        return launchFirefox(new LaunchOptions());
    }

    public static Browser launchFirefox(LaunchOptions options) {
        return launchBrowser(BrowserKind.BROWSER_KIND_FIREFOX, options);
    }

    public static Browser launchBrowser(BrowserKind browserKind, LaunchOptions options) {
        RuntimeClient runtime = getRuntime();
        StreamHandle<BrowserSessionCommand, BrowserSessionEvent> stream =
                new StreamHandle<>(runtime.asyncStub::browserSession);

        LaunchBrowserCommand.Builder launch = LaunchBrowserCommand.newBuilder().setBrowserKind(browserKind);
        if (options.browserBinary() != null && !options.browserBinary().isBlank()) {
            launch.setBrowserBinary(options.browserBinary());
        }
        if (options.timeoutMs() != null && options.timeoutMs() > 0) {
            launch.setRetryOptions(commandRetryOptions(options.timeoutMs()));
        }

        stream.send(BrowserSessionCommand.newBuilder().setLaunchBrowser(launch).build());

        while (true) {
            BrowserSessionEvent event = stream.recv("receive browser session event during launch");
            switch (event.getEventCase()) {
                case BROWSER_LAUNCHED -> {
                    BrowserLaunchedEvent launched = event.getBrowserLaunched();
                    Page initialPage = new Page(runtime, event.getSessionId(), launched.getInitialTabSessionId());
                    return new Browser(
                            runtime,
                            stream,
                            event.getSessionId(),
                            launched.getBrowser(),
                            launched.getNote(),
                            "",
                            launched.getUserDataDir(),
                            initialPage
                    );
                }
                case ERROR -> throw new AllwrightException(
                        "browser session error during launch: " + event.getError().getMessage()
                );
                default -> {
                }
            }
        }
    }

    public static String ping() {
        try {
            return getRuntime().blockingStub.ping(PingRequest.newBuilder().build()).getMessage();
        } catch (StatusRuntimeException exception) {
            throw new AllwrightException("ping engine server: " + exception.getStatus(), exception);
        }
    }

    public static void setServerAddr(String serverAddr) {
        synchronized (RUNTIME_LOCK) {
            serverAddrOverride = Objects.requireNonNull(serverAddr, "serverAddr").trim();
            shutdownLocked();
        }
    }

    public static void shutdown() {
        synchronized (RUNTIME_LOCK) {
            shutdownLocked();
        }
    }

    public static Path findConfigFile() {
        return findConfigFile(Path.of("").toAbsolutePath());
    }

    public static Path findConfigFile(Path startDir) {
        Path currentDir = Objects.requireNonNull(startDir, "startDir").toAbsolutePath().normalize();
        while (true) {
            for (String filename : CONFIG_FILENAMES) {
                Path candidate = currentDir.resolve(filename);
                if (Files.isRegularFile(candidate)) {
                    return candidate;
                }
            }
            Path parent = currentDir.getParent();
            if (parent == null) {
                return null;
            }
            currentDir = parent;
        }
    }

    public static AllwrightConfig loadConfigFile(Path configFile) {
        Path resolved = Objects.requireNonNull(configFile, "configFile").toAbsolutePath().normalize();
        try {
            Object loaded = new Yaml().load(Files.readString(resolved));
            if (!(loaded instanceof Map<?, ?> root)) {
                throw new AllwrightException("allwright config " + resolved + " must contain a top-level object");
            }
            validateConfigShape(root, resolved);
            return new AllwrightConfig(
                    integerValue(root.get("schemaVersion")),
                    mapValue(root.get("server")),
                    mapValue(root.get("browser")),
                    retryConfigValue(root.get("expect")),
                    suiteMapValue(root.get("suites"))
            );
        } catch (IOException exception) {
            throw new AllwrightException("read allwright config " + resolved + ": " + exception.getMessage(), exception);
        }
    }

    public static ResolvedConfig resolveConfig() {
        return resolveConfig(new ResolveConfigOptions());
    }

    public static ResolvedConfig resolveConfig(ResolveConfigOptions options) {
        ResolveConfigOptions resolvedOptions = options == null ? new ResolveConfigOptions() : options;
        Path configFilePath = resolvedOptions.configFile() != null
                ? resolvedOptions.configFile().toAbsolutePath().normalize()
                : findConfigFile(resolvedOptions.cwd() == null ? Path.of("").toAbsolutePath() : resolvedOptions.cwd());
        AllwrightConfig fileConfig = configFilePath != null ? loadConfigFile(configFilePath) : new AllwrightConfig();
        String suiteName = trimToNull(resolvedOptions.suite());
        Map<String, Object> suiteConfig = null;

        if (suiteName != null) {
            suiteConfig = fileConfig.suites() == null ? null : fileConfig.suites().get(suiteName);
            if (suiteConfig == null) {
                throw new AllwrightException(
                        "allwright config suite \"" + suiteName + "\" was not found in "
                                + (configFilePath == null ? "the resolved config file" : configFilePath)
                );
            }
        }

        String browserName = firstNonBlank(
                browserNameValue(mapValue(suiteConfig == null ? null : suiteConfig.get("browser"))),
                browserNameValue(fileConfig.browser()),
                "chromium"
        );
        String browserBinary = firstNonBlank(
                browserBinaryValue(mapValue(suiteConfig == null ? null : suiteConfig.get("browser"))),
                browserBinaryValue(fileConfig.browser())
        );
        String serverAddr = firstNonBlank(
                serverAddrValue(mapValue(suiteConfig == null ? null : suiteConfig.get("server"))),
                serverAddrValue(fileConfig.server())
        );
        LaunchOptions launchOptions = mergeLaunchOptions(
                launchOptionsValue(fileConfig.browser()),
                launchOptionsValue(mapValue(suiteConfig == null ? null : suiteConfig.get("browser")))
        );
        if (browserBinary != null) {
            launchOptions = new LaunchOptions(browserBinary, launchOptions.timeoutMs());
        }
        RetryConfig expect = mergeRetryConfig(
                fileConfig.expect(),
                retryConfigValue(suiteConfig == null ? null : suiteConfig.get("expect"))
        );

        return new ResolvedConfig(
                configFilePath,
                suiteName,
                serverAddr,
                browserName,
                browserBinary,
                launchOptions,
                expect
        );
    }

    public static Browser launchConfiguredBrowser(ResolvedConfig config) {
        Objects.requireNonNull(config, "config");
        if (config.serverAddr() != null && !config.serverAddr().isBlank()) {
            setServerAddr(config.serverAddr());
        }
        return switch (config.browserName()) {
            case "firefox" -> launchFirefox(config.launchOptions());
            case "chromium", "" -> launchChrome(config.launchOptions());
            default -> throw new AllwrightException(
                    "unsupported browser.name \"" + config.browserName() + "\"; use \"chromium\" or \"firefox\""
            );
        };
    }

    private static RuntimeClient getRuntime() {
        synchronized (RUNTIME_LOCK) {
            if (runtimeClient == null) {
                ManagedChannel channel = ManagedChannelBuilder.forTarget(resolveServerAddr())
                        .usePlaintext()
                        .build();
                runtimeClient = new RuntimeClient(
                        channel,
                        EngineServiceGrpc.newBlockingStub(channel),
                        EngineServiceGrpc.newStub(channel)
                );
            }
            return runtimeClient;
        }
    }

    private static String resolveServerAddr() {
        if (serverAddrOverride != null && !serverAddrOverride.isBlank()) {
            return serverAddrOverride;
        }
        String env = System.getenv(SERVER_ADDR_ENV_VAR);
        if (env != null && !env.isBlank()) {
            return env.trim();
        }
        return DEFAULT_SERVER_ADDR;
    }

    private static void shutdownLocked() {
        if (runtimeClient != null) {
            runtimeClient.channel.shutdownNow();
            runtimeClient = null;
        }
    }

    private static CommandRetryOptions commandRetryOptions(Integer timeoutMs) {
        return CommandRetryOptions.newBuilder().setTimeoutMs(timeoutMs).build();
    }

    public record LaunchOptions(String browserBinary, Integer timeoutMs) {
        public LaunchOptions() {
            this(null, null);
        }
    }

    public record RetryConfig(Integer timeoutMs, Integer intervalMs) {
        public RetryConfig() {
            this(null, null);
        }
    }

    public record AllwrightConfig(
            Integer schemaVersion,
            Map<String, Object> server,
            Map<String, Object> browser,
            RetryConfig expect,
            Map<String, Map<String, Object>> suites
    ) {
        public AllwrightConfig() {
            this(null, null, null, null, null);
        }
    }

    public record ResolveConfigOptions(Path cwd, Path configFile, String suite) {
        public ResolveConfigOptions() {
            this(null, null, null);
        }
    }

    public record ResolvedConfig(
            Path configFilePath,
            String suiteName,
            String serverAddr,
            String browserName,
            String browserBinary,
            LaunchOptions launchOptions,
            RetryConfig expect
    ) {}

    public record CommandOptions(Integer timeoutMs) {
        public CommandOptions() {
            this(null);
        }
    }

    public record HighlightOptions(Integer timeoutMs, Integer durationMs) {
        public HighlightOptions() {
            this(null, null);
        }
    }

    public record PressOptions(Integer timeoutMs, String text) {
        public PressOptions() {
            this(null, null);
        }
    }

    public record WaitForSelectorOptions(Integer timeoutMs, Boolean visible) {
        public WaitForSelectorOptions() {
            this(null, null);
        }
    }

    public record NavigateResult(
            String url,
            String note,
            String bidiSessionId,
            String mapperTargetId,
            String mapperSessionId,
            String packageVersion
    ) {}

    public record ClickResult(String selector, String note, String bidiSessionId) {}

    public record CountResult(String selector, int count, String note) {}

    public record HighlightResult(String selector, int count, String note) {}

    public record ElementResult(String selector, String note) {}

    public record FillResult(String selector, String value, String note) {}

    public record PressResult(String selector, String key, String note) {}

    public record TextResult(String selector, String text, String note) {}

    public record WaitForSelectorResult(String selector, boolean visible, String note) {}

    public static final class BrowserType {
        private final BrowserKind browserKind;

        private BrowserType(BrowserKind browserKind) {
            this.browserKind = browserKind;
        }

        public Browser launch() {
            return launch(new LaunchOptions());
        }

        public Browser launch(LaunchOptions options) {
            return Allwright.launchBrowser(browserKind, options);
        }
    }

    public static final class Browser {
        private final RuntimeClient runtime;
        private final StreamHandle<BrowserSessionCommand, BrowserSessionEvent> stream;
        private final Map<String, Page> pages = new ConcurrentHashMap<>();
        private final Page initialPage;
        private boolean closed;

        private final String sessionId;
        private final String browserName;
        private final String launchNote;
        private final String cdpWebSocketURL;
        private final String userDataDir;

        private Browser(
                RuntimeClient runtime,
                StreamHandle<BrowserSessionCommand, BrowserSessionEvent> stream,
                String sessionId,
                String browserName,
                String launchNote,
                String cdpWebSocketURL,
                String userDataDir,
                Page initialPage
        ) {
            this.runtime = runtime;
            this.stream = stream;
            this.sessionId = sessionId;
            this.browserName = browserName;
            this.launchNote = launchNote;
            this.cdpWebSocketURL = cdpWebSocketURL;
            this.userDataDir = userDataDir;
            this.initialPage = initialPage;
            this.pages.put(initialPage.sessionId(), initialPage);
        }

        public String sessionId() {
            return sessionId;
        }

        public Locator locator(String selector) {
            return new Locator(this, selector);
        }

        public String browserName() {
            return browserName;
        }

        public String launchNote() {
            return launchNote;
        }

        public String cdpWebSocketURL() {
            return cdpWebSocketURL;
        }

        public String userDataDir() {
            return userDataDir;
        }

        public Page page() {
            return initialPage;
        }

        public Page initialPage() {
            return initialPage;
        }

        public Page initialTab() {
            return initialPage;
        }

        public synchronized Page newPage() {
            ensureOpen();
            stream.send(BrowserSessionCommand.newBuilder().setOpenTab(OpenTabCommand.newBuilder().build()).build());

            while (true) {
                BrowserSessionEvent event = stream.recv("receive browser session event while opening page");
                switch (event.getEventCase()) {
                    case TAB_OPENED -> {
                        TabOpenedEvent opened = event.getTabOpened();
                        Page page = new Page(runtime, sessionId, opened.getTabSessionId());
                        pages.put(page.sessionId(), page);
                        return page;
                    }
                    case ERROR -> throw new AllwrightException(
                            "browser session error while opening page: " + event.getError().getMessage()
                    );
                    default -> {
                    }
                }
            }
        }

        public Page newTab() {
            return newPage();
        }

        public synchronized String ping(String message) {
            ensureOpen();
            stream.send(
                    BrowserSessionCommand.newBuilder()
                            .setPing(SessionPingCommand.newBuilder().setMessage(message).build())
                            .build()
            );

            while (true) {
                BrowserSessionEvent event = stream.recv("receive browser session event while pinging browser");
                switch (event.getEventCase()) {
                    case PONG -> {
                        return event.getPong().getMessage();
                    }
                    case ERROR -> throw new AllwrightException(
                            "browser session error while pinging: " + event.getError().getMessage()
                    );
                    default -> {
                    }
                }
            }
        }

        public synchronized void close() {
            if (closed) {
                return;
            }

            stream.send(
                    BrowserSessionCommand.newBuilder()
                            .setClose(CloseBrowserSessionCommand.newBuilder().build())
                            .build()
            );

            while (true) {
                BrowserSessionEvent event = stream.recv("receive browser session event while closing browser");
                switch (event.getEventCase()) {
                    case CLOSED -> {
                        BrowserSessionClosedEvent ignored = event.getClosed();
                        closed = true;
                        stream.closeSend();
                        return;
                    }
                    case ERROR -> throw new AllwrightException(
                            "browser session error while closing: " + event.getError().getMessage()
                    );
                    default -> {
                    }
                }
            }
        }

        private void ensureOpen() {
            if (closed) {
                throw new AllwrightException("browser session " + sessionId + " is closed");
            }
        }
    }

    public static final class Page {
        private final RuntimeClient runtime;
        private final String browserSessionId;
        private final String sessionId;
        private StreamHandle<TabSessionCommand, TabSessionEvent> stream;
        private boolean closed;

        private Page(RuntimeClient runtime, String browserSessionId, String sessionId) {
            this.runtime = runtime;
            this.browserSessionId = browserSessionId;
            this.sessionId = sessionId;
        }

        public String browserSessionId() {
            return browserSessionId;
        }

        public String sessionId() {
            return sessionId;
        }

        public synchronized NavigateResult goTo(String url) {
            StreamHandle<TabSessionCommand, TabSessionEvent> handle = ensureStream();
            ensureOpen();
            handle.send(
                    TabSessionCommand.newBuilder()
                            .setBrowserSessionId(browserSessionId)
                            .setTabSessionId(sessionId)
                            .setNavigate(NavigateTabCommand.newBuilder().setUrl(url).build())
                            .build()
            );

            String navigatedUrl = null;
            String navigatedNote = null;

            while (true) {
                TabSessionEvent event = handle.recv("receive tab session event while navigating");
                switch (event.getEventCase()) {
                    case NAVIGATED -> {
                        navigatedUrl = event.getNavigated().getUrl();
                        navigatedNote = event.getNavigated().getNote();
                    }
                    case CHROMIUM_BIDI_INJECTION -> {
                        if (navigatedUrl == null) {
                            continue;
                        }
                        return new NavigateResult(
                                navigatedUrl,
                                navigatedNote == null ? "" : navigatedNote,
                                event.getChromiumBidiInjection().getBidiSessionId(),
                                event.getChromiumBidiInjection().getMapperTargetId(),
                                event.getChromiumBidiInjection().getMapperSessionId(),
                                event.getChromiumBidiInjection().getPackageVersion()
                        );
                    }
                    case CLOSED -> {
                        closed = true;
                        throw new AllwrightException("page session " + sessionId + " closed while navigating");
                    }
                    case ERROR -> throw new AllwrightException(
                            "page session error while navigating: " + event.getError().getMessage()
                    );
                    default -> {
                    }
                }
            }
        }

        public synchronized NavigateResult navigate(String url) {
            return goTo(url);
        }

        public synchronized ClickResult click(String selector) {
            StreamHandle<TabSessionCommand, TabSessionEvent> handle = ensureStream();
            ensureOpen();
            handle.send(
                    TabSessionCommand.newBuilder()
                            .setBrowserSessionId(browserSessionId)
                            .setTabSessionId(sessionId)
                            .setClickElement(ClickElementCommand.newBuilder().setCssSelector(selector).build())
                            .build()
            );

            while (true) {
                TabSessionEvent event = handle.recv("receive tab session event while clicking");
                switch (event.getEventCase()) {
                    case ELEMENT_CLICKED -> {
                        return new ClickResult(
                                event.getElementClicked().getCssSelector(),
                                event.getElementClicked().getNote(),
                                event.getElementClicked().getBidiSessionId()
                        );
                    }
                    case CLOSED -> {
                        closed = true;
                        throw new AllwrightException("page session " + sessionId + " closed while clicking");
                    }
                    case ERROR -> throw new AllwrightException(
                            "page session error while clicking: " + event.getError().getMessage()
                    );
                    default -> {
                    }
                }
            }
        }

        public synchronized String ping(String message) {
            StreamHandle<TabSessionCommand, TabSessionEvent> handle = ensureStream();
            ensureOpen();
            handle.send(
                    TabSessionCommand.newBuilder()
                            .setBrowserSessionId(browserSessionId)
                            .setTabSessionId(sessionId)
                            .setPing(TabSessionPingCommand.newBuilder().setMessage(message).build())
                            .build()
            );

            while (true) {
                TabSessionEvent event = handle.recv("receive tab session event while pinging page");
                switch (event.getEventCase()) {
                    case PONG -> {
                        return event.getPong().getMessage();
                    }
                    case CLOSED -> {
                        closed = true;
                        throw new AllwrightException("page session " + sessionId + " closed while pinging");
                    }
                    case ERROR -> throw new AllwrightException(
                            "page session error while pinging: " + event.getError().getMessage()
                    );
                    default -> {
                    }
                }
            }
        }

        public synchronized void close() {
            StreamHandle<TabSessionCommand, TabSessionEvent> handle = ensureStream();
            if (closed) {
                return;
            }

            handle.send(
                    TabSessionCommand.newBuilder()
                            .setBrowserSessionId(browserSessionId)
                            .setTabSessionId(sessionId)
                            .setClose(CloseTabSessionCommand.newBuilder().build())
                            .build()
            );

            while (true) {
                TabSessionEvent event = handle.recv("receive tab session event while closing page");
                switch (event.getEventCase()) {
                    case CLOSED -> {
                        closed = true;
                        handle.closeSend();
                        return;
                    }
                    case ERROR -> throw new AllwrightException(
                            "page session error while closing: " + event.getError().getMessage()
                    );
                    default -> {
                    }
                }
            }
        }

        private void ensureOpen() {
            if (closed) {
                throw new AllwrightException("page session " + sessionId + " is closed");
            }
        }

        private StreamHandle<TabSessionCommand, TabSessionEvent> ensureStream() {
            if (stream == null) {
                stream = new StreamHandle<>(runtime.asyncStub::tabSession);
            }
            return stream;
        }
    }

    public static final class Locator {
        private final Page page;
        private final String selector;

        private Locator(Page page, String selector) {
            this.page = page;
            this.selector = selector;
        }

        public Page page() {
            return page;
        }

        public String selector() {
            return selector;
        }

        public Locator locator(String childSelector) {
            return new Locator(page, (selector + " " + childSelector).trim());
        }

        public ClickResult click() {
            return page.click(selector);
        }

        public CountResult count() {
            return page.count(selector);
        }

        public HighlightResult highlight() {
            return page.highlight(selector);
        }

        public ElementResult focus() {
            return page.focus(selector);
        }

        public FillResult fill(String value) {
            return page.fill(selector, value);
        }

        public ElementResult hover() {
            return page.hover(selector);
        }

        public PressResult press(String key) {
            return page.press(selector, key);
        }

        public TextResult textContent() {
            return page.textContent(selector);
        }

        public TextResult innerText() {
            return page.innerText(selector);
        }

        public WaitForSelectorResult waitFor() {
            return page.waitForSelector(selector);
        }
    }

    public static final class AllwrightException extends RuntimeException {
        public AllwrightException(String message) {
            super(message);
        }

        public AllwrightException(String message, Throwable cause) {
            super(message, cause);
        }
    }

    private static void validateConfigShape(Map<?, ?> root, Path source) {
        Integer schemaVersion = integerValue(root.get("schemaVersion"));
        if (schemaVersion != null && schemaVersion != 1) {
            throw new AllwrightException(
                    "allwright config " + source + " has unsupported schemaVersion " + schemaVersion + "; expected 1"
            );
        }

        String browserName = browserNameValue(mapValue(root.get("browser")));
        if (browserName != null && !browserName.equals("chromium") && !browserName.equals("firefox")) {
            throw new AllwrightException(
                    "allwright config " + source + " has unsupported browser.name \"" + browserName
                            + "\"; use \"chromium\" or \"firefox\""
            );
        }
    }

    @SuppressWarnings("unchecked")
    private static Map<String, Object> mapValue(Object value) {
        if (!(value instanceof Map<?, ?> map)) {
            return null;
        }
        Map<String, Object> converted = new LinkedHashMap<>();
        for (Map.Entry<?, ?> entry : map.entrySet()) {
            if (entry.getKey() != null) {
                converted.put(String.valueOf(entry.getKey()), entry.getValue());
            }
        }
        return converted;
    }

    @SuppressWarnings("unchecked")
    private static Map<String, Map<String, Object>> suiteMapValue(Object value) {
        if (!(value instanceof Map<?, ?> raw)) {
            return null;
        }
        Map<String, Map<String, Object>> suites = new LinkedHashMap<>();
        for (Map.Entry<?, ?> entry : raw.entrySet()) {
            if (entry.getKey() != null) {
                suites.put(String.valueOf(entry.getKey()), mapValue(entry.getValue()));
            }
        }
        return suites;
    }

    private static RetryConfig retryConfigValue(Object value) {
        Map<String, Object> map = mapValue(value);
        if (map == null) {
            return null;
        }
        return new RetryConfig(integerValue(map.get("timeoutMs")), integerValue(map.get("intervalMs")));
    }

    private static LaunchOptions launchOptionsValue(Map<String, Object> browser) {
        if (browser == null) {
            return new LaunchOptions();
        }
        Map<String, Object> launchOptions = mapValue(browser.get("launchOptions"));
        if (launchOptions == null) {
            return new LaunchOptions();
        }
        return new LaunchOptions(
                trimToNull(stringValue(launchOptions.get("browserBinary"))),
                integerValue(launchOptions.get("timeoutMs"))
        );
    }

    private static LaunchOptions mergeLaunchOptions(LaunchOptions base, LaunchOptions override) {
        if (override == null) {
            return base == null ? new LaunchOptions() : base;
        }
        LaunchOptions resolvedBase = base == null ? new LaunchOptions() : base;
        return new LaunchOptions(
                firstNonBlank(override.browserBinary(), resolvedBase.browserBinary()),
                override.timeoutMs() != null ? override.timeoutMs() : resolvedBase.timeoutMs()
        );
    }

    private static RetryConfig mergeRetryConfig(RetryConfig base, RetryConfig override) {
        RetryConfig resolvedBase = base == null ? new RetryConfig() : base;
        if (override == null) {
            return resolvedBase;
        }
        return new RetryConfig(
                override.timeoutMs() != null ? override.timeoutMs() : resolvedBase.timeoutMs(),
                override.intervalMs() != null ? override.intervalMs() : resolvedBase.intervalMs()
        );
    }

    private static String browserNameValue(Map<String, Object> browser) {
        return trimToNull(stringValue(browser == null ? null : browser.get("name")));
    }

    private static String browserBinaryValue(Map<String, Object> browser) {
        return trimToNull(stringValue(browser == null ? null : browser.get("binary")));
    }

    private static String serverAddrValue(Map<String, Object> server) {
        return trimToNull(stringValue(server == null ? null : server.get("addr")));
    }

    private static Integer integerValue(Object value) {
        if (value instanceof Number number) {
            return number.intValue();
        }
        if (value instanceof String string && !string.isBlank()) {
            try {
                return Integer.parseInt(string.trim());
            } catch (NumberFormatException ignored) {
                return null;
            }
        }
        return null;
    }

    private static String stringValue(Object value) {
        return value == null ? null : String.valueOf(value);
    }

    private static String trimToNull(String value) {
        if (value == null) {
            return null;
        }
        String trimmed = value.trim();
        return trimmed.isEmpty() ? null : trimmed;
    }

    private static String firstNonBlank(String... values) {
        for (String value : values) {
            if (value != null && !value.isBlank()) {
                return value.trim();
            }
        }
        return null;
    }

    private record RuntimeClient(
            ManagedChannel channel,
            EngineServiceGrpc.EngineServiceBlockingStub blockingStub,
            EngineServiceGrpc.EngineServiceStub asyncStub
    ) {}

    private static final class StreamHandle<RequestT, ResponseT> {
        private final EventQueue<ResponseT> events = new EventQueue<>();
        private final StreamObserver<RequestT> requests;
        private boolean sendClosed;

        private StreamHandle(StreamFactory<RequestT, ResponseT> streamFactory) {
            this.requests = streamFactory.open(new StreamObserver<>() {
                @Override
                public void onNext(ResponseT value) {
                    events.push(value);
                }

                @Override
                public void onError(Throwable throwable) {
                    events.fail(throwable);
                }

                @Override
                public void onCompleted() {
                    events.complete();
                }
            });
        }

        private void send(RequestT message) {
            if (sendClosed) {
                throw new AllwrightException("cannot send on a closed stream");
            }
            try {
                requests.onNext(message);
            } catch (RuntimeException exception) {
                throw new AllwrightException("send stream command: " + exception.getMessage(), exception);
            }
        }

        private ResponseT recv(String action) {
            return events.next(action);
        }

        private void closeSend() {
            if (sendClosed) {
                return;
            }
            sendClosed = true;
            requests.onCompleted();
        }
    }

    private static final class EventQueue<T> {
        private final BlockingQueue<EventOrFailure<T>> items = new LinkedBlockingQueue<>();

        private void push(T value) {
            items.add(EventOrFailure.event(value));
        }

        private void fail(Throwable throwable) {
            items.add(EventOrFailure.failure(throwable));
        }

        private void complete() {
            items.add(EventOrFailure.completed());
        }

        private T next(String action) {
            try {
                EventOrFailure<T> item = items.take();
                if (item.failure != null) {
                    throw new AllwrightException(action + ": " + item.failure.getMessage(), item.failure);
                }
                if (item.completed) {
                    throw new AllwrightException(action + ": stream ended unexpectedly");
                }
                return item.event;
            } catch (InterruptedException exception) {
                Thread.currentThread().interrupt();
                throw new AllwrightException(action + ": interrupted while waiting for stream event", exception);
            }
        }
    }

    private static final class EventOrFailure<T> {
        private final T event;
        private final Throwable failure;
        private final boolean completed;

        private EventOrFailure(T event, Throwable failure, boolean completed) {
            this.event = event;
            this.failure = failure;
            this.completed = completed;
        }

        private static <T> EventOrFailure<T> event(T value) {
            return new EventOrFailure<>(value, null, false);
        }

        private static <T> EventOrFailure<T> failure(Throwable throwable) {
            return new EventOrFailure<>(null, throwable, false);
        }

        private static <T> EventOrFailure<T> completed() {
            return new EventOrFailure<>(null, null, true);
        }
    }

    @FunctionalInterface
    private interface StreamFactory<RequestT, ResponseT> {
        StreamObserver<RequestT> open(StreamObserver<ResponseT> responseObserver);
    }
}
