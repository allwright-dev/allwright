package dev.allwright.client;

import dev.allwright.engine.v1.BrowserLaunchedEvent;
import dev.allwright.engine.v1.BrowserKind;
import dev.allwright.engine.v1.BrowserSessionClosedEvent;
import dev.allwright.engine.v1.BrowserSessionCommand;
import dev.allwright.engine.v1.BrowserSessionEvent;
import dev.allwright.engine.v1.ClickElementCommand;
import dev.allwright.engine.v1.CloseBrowserSessionCommand;
import dev.allwright.engine.v1.CloseTabSessionCommand;
import dev.allwright.engine.v1.CommandRetryOptions;
import dev.allwright.engine.v1.CountElementsCommand;
import dev.allwright.engine.v1.EngineServiceGrpc;
import dev.allwright.engine.v1.FillElementCommand;
import dev.allwright.engine.v1.FocusElementCommand;
import dev.allwright.engine.v1.GetInnerTextCommand;
import dev.allwright.engine.v1.GetTextContentCommand;
import dev.allwright.engine.v1.HighlightElementsCommand;
import dev.allwright.engine.v1.HoverElementCommand;
import dev.allwright.engine.v1.LaunchBrowserCommand;
import dev.allwright.engine.v1.NavigateTabCommand;
import dev.allwright.engine.v1.OpenTabCommand;
import dev.allwright.engine.v1.PingRequest;
import dev.allwright.engine.v1.PressKeyCommand;
import dev.allwright.engine.v1.SessionPingCommand;
import dev.allwright.engine.v1.TabOpenedEvent;
import dev.allwright.engine.v1.TabSessionCommand;
import dev.allwright.engine.v1.TabSessionEvent;
import dev.allwright.engine.v1.TabSessionPingCommand;
import dev.allwright.engine.v1.WaitForSelectorCommand;
import io.grpc.ManagedChannel;
import io.grpc.ManagedChannelBuilder;
import io.grpc.StatusRuntimeException;
import io.grpc.stub.StreamObserver;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.LinkedHashMap;
import java.util.Objects;
import java.util.concurrent.BlockingQueue;
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

    private static SelectorTransport parseSelectorForTransport(String selector) {
        String trimmed = selector == null ? "" : selector.trim();
        String lowered = trimmed.toLowerCase();
        if (lowered.startsWith("xpath=") || lowered.startsWith("xpath:")) {
            return new SelectorTransport("xpath", decodeSelectorBody(trimmed.substring(6)));
        }
        if (lowered.startsWith("css=") || lowered.startsWith("css:")) {
            return new SelectorTransport("css", decodeSelectorBody(trimmed.substring(4)));
        }
        if (
                trimmed.startsWith("//")
                        || trimmed.startsWith(".//")
                        || trimmed.startsWith("../")
                        || trimmed.startsWith("/")
                        || trimmed.startsWith("(")
        ) {
            return new SelectorTransport("xpath", trimmed);
        }
        return new SelectorTransport("css", trimmed);
    }

    private static String decodeSelectorBody(String body) {
        String candidate = body == null ? "" : body.trim();
        if (candidate.length() >= 2 && candidate.charAt(0) == '"' && candidate.charAt(candidate.length() - 1) == '"') {
            try {
                return unescapeJsonString(candidate.substring(1, candidate.length() - 1));
            } catch (IllegalArgumentException ignored) {
                return candidate;
            }
        }
        return candidate;
    }

    private static String normalizeSelectorForTransport(String selector) {
        SelectorTransport parsed = parseSelectorForTransport(selector);
        return parsed.kind() + "=" + quoteJson(parsed.body());
    }

    private static String chainSelectorForTransport(String parent, String child) {
        String normalizedParent = parent == null || parent.trim().isEmpty()
                ? ""
                : normalizeSelectorForTransport(parent);
        String normalizedChild = child == null || child.trim().isEmpty()
                ? ""
                : normalizeSelectorForTransport(child);
        if (normalizedParent.isEmpty()) {
            return normalizedChild;
        }
        if (normalizedChild.isEmpty()) {
            return normalizedParent;
        }
        return normalizedParent + " " + normalizedChild;
    }

    private static String quoteJson(String value) {
        StringBuilder builder = new StringBuilder(value.length() + 2);
        builder.append('"');
        for (int index = 0; index < value.length(); index++) {
            char ch = value.charAt(index);
            switch (ch) {
                case '"' -> builder.append("\\\"");
                case '\\' -> builder.append("\\\\");
                case '\b' -> builder.append("\\b");
                case '\f' -> builder.append("\\f");
                case '\n' -> builder.append("\\n");
                case '\r' -> builder.append("\\r");
                case '\t' -> builder.append("\\t");
                default -> {
                    if (ch < 0x20) {
                        builder.append(String.format("\\u%04x", (int) ch));
                    } else {
                        builder.append(ch);
                    }
                }
            }
        }
        builder.append('"');
        return builder.toString();
    }

    private static String unescapeJsonString(String value) {
        StringBuilder builder = new StringBuilder(value.length());
        for (int index = 0; index < value.length(); index++) {
            char ch = value.charAt(index);
            if (ch != '\\') {
                builder.append(ch);
                continue;
            }
            if (index + 1 >= value.length()) {
                throw new IllegalArgumentException("unterminated escape");
            }
            char escaped = value.charAt(++index);
            switch (escaped) {
                case '"', '\\', '/' -> builder.append(escaped);
                case 'b' -> builder.append('\b');
                case 'f' -> builder.append('\f');
                case 'n' -> builder.append('\n');
                case 'r' -> builder.append('\r');
                case 't' -> builder.append('\t');
                case 'u' -> {
                    if (index + 4 >= value.length()) {
                        throw new IllegalArgumentException("invalid unicode escape");
                    }
                    String hex = value.substring(index + 1, index + 5);
                    builder.append((char) Integer.parseInt(hex, 16));
                    index += 4;
                }
                default -> throw new IllegalArgumentException("unsupported escape");
            }
        }
        return builder.toString();
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

    private static boolean hasTimeout(Integer timeoutMs) {
        return timeoutMs != null && timeoutMs > 0;
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

    public static final class Browser implements AutoCloseable {
        private final RuntimeClient runtime;
        private final StreamHandle<BrowserSessionCommand, BrowserSessionEvent> stream;
        private final Map<String, Page> pages = new LinkedHashMap<>();
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
            return initialPage.locator(selector);
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

        public synchronized List<Page> pages() {
            return List.copyOf(new ArrayList<>(pages.values()));
        }

        public synchronized Page newPage() {
            return newPage(new CommandOptions());
        }

        public synchronized Page newPage(CommandOptions options) {
            ensureOpen();
            CommandOptions resolvedOptions = options == null ? new CommandOptions() : options;
            OpenTabCommand.Builder openTab = OpenTabCommand.newBuilder();
            if (resolvedOptions.timeoutMs() != null && resolvedOptions.timeoutMs() > 0) {
                openTab.setRetryOptions(commandRetryOptions(resolvedOptions.timeoutMs()));
            }
            stream.send(BrowserSessionCommand.newBuilder().setOpenTab(openTab).build());

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
            return newPage(new CommandOptions());
        }

        public Page newTab(CommandOptions options) {
            return newPage(options);
        }

        public synchronized String ping() {
            return ping("ping");
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

    public static final class Page implements AutoCloseable {
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

        public Locator locator(String selector) {
            return new Locator(this, normalizeSelectorForTransport(selector));
        }

        public synchronized NavigateResult goTo(String url) {
            return goTo(url, new CommandOptions());
        }

        public synchronized NavigateResult goTo(String url, CommandOptions options) {
            StreamHandle<TabSessionCommand, TabSessionEvent> handle = ensureStream();
            ensureOpen();
            CommandOptions resolvedOptions = options == null ? new CommandOptions() : options;
            NavigateTabCommand.Builder navigate = NavigateTabCommand.newBuilder().setUrl(url);
            if (hasTimeout(resolvedOptions.timeoutMs())) {
                navigate.setRetryOptions(commandRetryOptions(resolvedOptions.timeoutMs()));
            }
            handle.send(
                    TabSessionCommand.newBuilder()
                            .setBrowserSessionId(browserSessionId)
                            .setTabSessionId(sessionId)
                            .setNavigate(navigate)
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
            return goTo(url, new CommandOptions());
        }

        public synchronized NavigateResult navigate(String url, CommandOptions options) {
            return goTo(url, options);
        }

        public synchronized ClickResult click(String selector) {
            return click(selector, new CommandOptions());
        }

        public synchronized ClickResult click(String selector, CommandOptions options) {
            StreamHandle<TabSessionCommand, TabSessionEvent> handle = ensureStream();
            ensureOpen();
            CommandOptions resolvedOptions = options == null ? new CommandOptions() : options;
            String transportSelector = normalizeSelectorForTransport(selector);
            ClickElementCommand.Builder click = ClickElementCommand.newBuilder().setCssSelector(transportSelector);
            if (hasTimeout(resolvedOptions.timeoutMs())) {
                click.setRetryOptions(commandRetryOptions(resolvedOptions.timeoutMs()));
            }
            handle.send(
                    TabSessionCommand.newBuilder()
                            .setBrowserSessionId(browserSessionId)
                            .setTabSessionId(sessionId)
                            .setClickElement(click)
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

        public synchronized CountResult count(String selector) {
            return count(selector, new CommandOptions());
        }

        public synchronized CountResult count(String selector, CommandOptions options) {
            StreamHandle<TabSessionCommand, TabSessionEvent> handle = ensureStream();
            ensureOpen();
            CommandOptions resolvedOptions = options == null ? new CommandOptions() : options;
            String transportSelector = normalizeSelectorForTransport(selector);
            CountElementsCommand.Builder count = CountElementsCommand.newBuilder().setCssSelector(transportSelector);
            if (hasTimeout(resolvedOptions.timeoutMs())) {
                count.setRetryOptions(commandRetryOptions(resolvedOptions.timeoutMs()));
            }
            handle.send(
                    TabSessionCommand.newBuilder()
                            .setBrowserSessionId(browserSessionId)
                            .setTabSessionId(sessionId)
                            .setCountElements(count)
                            .build()
            );

            while (true) {
                TabSessionEvent event = handle.recv("receive tab session event while counting elements");
                switch (event.getEventCase()) {
                    case ELEMENT_COUNTED -> {
                        return new CountResult(
                                event.getElementCounted().getCssSelector(),
                                event.getElementCounted().getCount(),
                                event.getElementCounted().getNote()
                        );
                    }
                    case CLOSED -> {
                        closed = true;
                        throw new AllwrightException("page session " + sessionId + " closed while counting elements");
                    }
                    case ERROR -> throw new AllwrightException(
                            "page session error while counting elements: " + event.getError().getMessage()
                    );
                    default -> {
                    }
                }
            }
        }

        public synchronized HighlightResult highlight(String selector) {
            return highlight(selector, new HighlightOptions());
        }

        public synchronized HighlightResult highlight(String selector, HighlightOptions options) {
            StreamHandle<TabSessionCommand, TabSessionEvent> handle = ensureStream();
            ensureOpen();
            HighlightOptions resolvedOptions = options == null ? new HighlightOptions() : options;
            String transportSelector = normalizeSelectorForTransport(selector);
            HighlightElementsCommand.Builder highlight =
                    HighlightElementsCommand.newBuilder().setCssSelector(transportSelector);
            if (resolvedOptions.durationMs() != null && resolvedOptions.durationMs() > 0) {
                highlight.setDurationMs(resolvedOptions.durationMs());
            }
            if (hasTimeout(resolvedOptions.timeoutMs())) {
                highlight.setRetryOptions(commandRetryOptions(resolvedOptions.timeoutMs()));
            }
            handle.send(
                    TabSessionCommand.newBuilder()
                            .setBrowserSessionId(browserSessionId)
                            .setTabSessionId(sessionId)
                            .setHighlightElements(highlight)
                            .build()
            );

            while (true) {
                TabSessionEvent event = handle.recv("receive tab session event while highlighting elements");
                switch (event.getEventCase()) {
                    case ELEMENTS_HIGHLIGHTED -> {
                        return new HighlightResult(
                                event.getElementsHighlighted().getCssSelector(),
                                event.getElementsHighlighted().getCount(),
                                event.getElementsHighlighted().getNote()
                        );
                    }
                    case CLOSED -> {
                        closed = true;
                        throw new AllwrightException(
                                "page session " + sessionId + " closed while highlighting elements"
                        );
                    }
                    case ERROR -> throw new AllwrightException(
                            "page session error while highlighting elements: " + event.getError().getMessage()
                    );
                    default -> {
                    }
                }
            }
        }

        public synchronized ElementResult focus(String selector) {
            return focus(selector, new CommandOptions());
        }

        public synchronized ElementResult focus(String selector, CommandOptions options) {
            CommandOptions resolvedOptions = options == null ? new CommandOptions() : options;
            String transportSelector = normalizeSelectorForTransport(selector);
            FocusElementCommand.Builder focus = FocusElementCommand.newBuilder().setCssSelector(transportSelector);
            if (hasTimeout(resolvedOptions.timeoutMs())) {
                focus.setRetryOptions(commandRetryOptions(resolvedOptions.timeoutMs()));
            }
            return performElementCommand(
                    "focusing",
                    TabSessionCommand.newBuilder()
                            .setBrowserSessionId(browserSessionId)
                            .setTabSessionId(sessionId)
                            .setFocusElement(focus)
                            .build(),
                    TabSessionEvent.EventCase.ELEMENT_FOCUSED
            );
        }

        public synchronized FillResult fill(String selector, String value) {
            return fill(selector, value, new CommandOptions());
        }

        public synchronized FillResult fill(String selector, String value, CommandOptions options) {
            StreamHandle<TabSessionCommand, TabSessionEvent> handle = ensureStream();
            ensureOpen();
            CommandOptions resolvedOptions = options == null ? new CommandOptions() : options;
            String transportSelector = normalizeSelectorForTransport(selector);
            FillElementCommand.Builder fill = FillElementCommand.newBuilder()
                    .setCssSelector(transportSelector)
                    .setValue(value);
            if (hasTimeout(resolvedOptions.timeoutMs())) {
                fill.setRetryOptions(commandRetryOptions(resolvedOptions.timeoutMs()));
            }
            handle.send(
                    TabSessionCommand.newBuilder()
                            .setBrowserSessionId(browserSessionId)
                            .setTabSessionId(sessionId)
                            .setFillElement(fill)
                            .build()
            );

            while (true) {
                TabSessionEvent event = handle.recv("receive tab session event while filling");
                switch (event.getEventCase()) {
                    case ELEMENT_FILLED -> {
                        return new FillResult(
                                event.getElementFilled().getCssSelector(),
                                event.getElementFilled().getValue(),
                                event.getElementFilled().getNote()
                        );
                    }
                    case CLOSED -> {
                        closed = true;
                        throw new AllwrightException("page session " + sessionId + " closed while filling");
                    }
                    case ERROR -> throw new AllwrightException(
                            "page session error while filling: " + event.getError().getMessage()
                    );
                    default -> {
                    }
                }
            }
        }

        public synchronized ElementResult hover(String selector) {
            return hover(selector, new CommandOptions());
        }

        public synchronized ElementResult hover(String selector, CommandOptions options) {
            CommandOptions resolvedOptions = options == null ? new CommandOptions() : options;
            String transportSelector = normalizeSelectorForTransport(selector);
            HoverElementCommand.Builder hover = HoverElementCommand.newBuilder().setCssSelector(transportSelector);
            if (hasTimeout(resolvedOptions.timeoutMs())) {
                hover.setRetryOptions(commandRetryOptions(resolvedOptions.timeoutMs()));
            }
            return performElementCommand(
                    "hovering",
                    TabSessionCommand.newBuilder()
                            .setBrowserSessionId(browserSessionId)
                            .setTabSessionId(sessionId)
                            .setHoverElement(hover)
                            .build(),
                    TabSessionEvent.EventCase.ELEMENT_HOVERED
            );
        }

        public synchronized PressResult press(String selector, String key) {
            return press(selector, key, new PressOptions());
        }

        public synchronized PressResult press(String selector, String key, PressOptions options) {
            StreamHandle<TabSessionCommand, TabSessionEvent> handle = ensureStream();
            ensureOpen();
            PressOptions resolvedOptions = options == null ? new PressOptions() : options;
            String transportSelector = normalizeSelectorForTransport(selector);
            PressKeyCommand.Builder press = PressKeyCommand.newBuilder()
                    .setCssSelector(transportSelector)
                    .setKey(key);
            if (resolvedOptions.text() != null && !resolvedOptions.text().isBlank()) {
                press.setText(resolvedOptions.text());
            }
            if (hasTimeout(resolvedOptions.timeoutMs())) {
                press.setRetryOptions(commandRetryOptions(resolvedOptions.timeoutMs()));
            }
            handle.send(
                    TabSessionCommand.newBuilder()
                            .setBrowserSessionId(browserSessionId)
                            .setTabSessionId(sessionId)
                            .setPressKey(press)
                            .build()
            );

            while (true) {
                TabSessionEvent event = handle.recv("receive tab session event while pressing key");
                switch (event.getEventCase()) {
                    case KEY_PRESSED -> {
                        return new PressResult(
                                event.getKeyPressed().getCssSelector(),
                                event.getKeyPressed().getKey(),
                                event.getKeyPressed().getNote()
                        );
                    }
                    case CLOSED -> {
                        closed = true;
                        throw new AllwrightException("page session " + sessionId + " closed while pressing key");
                    }
                    case ERROR -> throw new AllwrightException(
                            "page session error while pressing key: " + event.getError().getMessage()
                    );
                    default -> {
                    }
                }
            }
        }

        public synchronized TextResult textContent(String selector) {
            return textContent(selector, new CommandOptions());
        }

        public synchronized TextResult textContent(String selector, CommandOptions options) {
            return readText(selector, options, true);
        }

        public synchronized TextResult innerText(String selector) {
            return innerText(selector, new CommandOptions());
        }

        public synchronized TextResult innerText(String selector, CommandOptions options) {
            return readText(selector, options, false);
        }

        public synchronized WaitForSelectorResult waitForSelector(String selector) {
            return waitForSelector(selector, new WaitForSelectorOptions());
        }

        public synchronized WaitForSelectorResult waitForSelector(String selector, WaitForSelectorOptions options) {
            StreamHandle<TabSessionCommand, TabSessionEvent> handle = ensureStream();
            ensureOpen();
            WaitForSelectorOptions resolvedOptions = options == null ? new WaitForSelectorOptions() : options;
            String transportSelector = normalizeSelectorForTransport(selector);
            WaitForSelectorCommand.Builder waitForSelector =
                    WaitForSelectorCommand.newBuilder().setCssSelector(transportSelector);
            if (resolvedOptions.visible() != null) {
                waitForSelector.setVisible(resolvedOptions.visible());
            }
            if (hasTimeout(resolvedOptions.timeoutMs())) {
                waitForSelector.setRetryOptions(commandRetryOptions(resolvedOptions.timeoutMs()));
            }
            handle.send(
                    TabSessionCommand.newBuilder()
                            .setBrowserSessionId(browserSessionId)
                            .setTabSessionId(sessionId)
                            .setWaitForSelector(waitForSelector)
                            .build()
            );

            while (true) {
                TabSessionEvent event = handle.recv("receive tab session event while waiting for selector");
                switch (event.getEventCase()) {
                    case SELECTOR_WAIT_SATISFIED -> {
                        return new WaitForSelectorResult(
                                event.getSelectorWaitSatisfied().getCssSelector(),
                                event.getSelectorWaitSatisfied().getVisible(),
                                event.getSelectorWaitSatisfied().getNote()
                        );
                    }
                    case CLOSED -> {
                        closed = true;
                        throw new AllwrightException(
                                "page session " + sessionId + " closed while waiting for selector"
                        );
                    }
                    case ERROR -> throw new AllwrightException(
                            "page session error while waiting for selector: " + event.getError().getMessage()
                    );
                    default -> {
                    }
                }
            }
        }

        public synchronized String ping() {
            return ping("ping");
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

        private ElementResult performElementCommand(
                String action,
                TabSessionCommand command,
                TabSessionEvent.EventCase successCase
        ) {
            StreamHandle<TabSessionCommand, TabSessionEvent> handle = ensureStream();
            ensureOpen();
            handle.send(command);

            while (true) {
                TabSessionEvent event = handle.recv("receive tab session event while " + action);
                switch (event.getEventCase()) {
                    case ELEMENT_FOCUSED -> {
                        if (successCase == TabSessionEvent.EventCase.ELEMENT_FOCUSED) {
                            return new ElementResult(
                                    event.getElementFocused().getCssSelector(),
                                    event.getElementFocused().getNote()
                            );
                        }
                    }
                    case ELEMENT_HOVERED -> {
                        if (successCase == TabSessionEvent.EventCase.ELEMENT_HOVERED) {
                            return new ElementResult(
                                    event.getElementHovered().getCssSelector(),
                                    event.getElementHovered().getNote()
                            );
                        }
                    }
                    case CLOSED -> {
                        closed = true;
                        throw new AllwrightException("page session " + sessionId + " closed while " + action);
                    }
                    case ERROR -> throw new AllwrightException(
                            "page session error while " + action + ": " + event.getError().getMessage()
                    );
                    default -> {
                    }
                }
            }
        }

        private TextResult readText(String selector, CommandOptions options, boolean textContent) {
            StreamHandle<TabSessionCommand, TabSessionEvent> handle = ensureStream();
            ensureOpen();
            CommandOptions resolvedOptions = options == null ? new CommandOptions() : options;
            String transportSelector = normalizeSelectorForTransport(selector);
            TabSessionCommand.Builder command = TabSessionCommand.newBuilder()
                    .setBrowserSessionId(browserSessionId)
                    .setTabSessionId(sessionId);
            if (textContent) {
                GetTextContentCommand.Builder getTextContent =
                        GetTextContentCommand.newBuilder().setCssSelector(transportSelector);
                if (hasTimeout(resolvedOptions.timeoutMs())) {
                    getTextContent.setRetryOptions(commandRetryOptions(resolvedOptions.timeoutMs()));
                }
                command.setGetTextContent(getTextContent);
            } else {
                GetInnerTextCommand.Builder getInnerText =
                        GetInnerTextCommand.newBuilder().setCssSelector(transportSelector);
                if (hasTimeout(resolvedOptions.timeoutMs())) {
                    getInnerText.setRetryOptions(commandRetryOptions(resolvedOptions.timeoutMs()));
                }
                command.setGetInnerText(getInnerText);
            }
            handle.send(command.build());

            while (true) {
                TabSessionEvent event = handle.recv(
                        "receive tab session event while " + (textContent ? "reading text content" : "reading inner text")
                );
                switch (event.getEventCase()) {
                    case TEXT_CONTENT_RESOLVED -> {
                        if (textContent) {
                            return new TextResult(
                                    event.getTextContentResolved().getCssSelector(),
                                    event.getTextContentResolved().getText(),
                                    event.getTextContentResolved().getNote()
                            );
                        }
                    }
                    case INNER_TEXT_RESOLVED -> {
                        if (!textContent) {
                            return new TextResult(
                                    event.getInnerTextResolved().getCssSelector(),
                                    event.getInnerTextResolved().getText(),
                                    event.getInnerTextResolved().getNote()
                            );
                        }
                    }
                    case CLOSED -> {
                        closed = true;
                        throw new AllwrightException(
                                "page session " + sessionId + " closed while "
                                        + (textContent ? "reading text content" : "reading inner text")
                        );
                    }
                    case ERROR -> throw new AllwrightException(
                            "page session error while "
                                    + (textContent ? "reading text content" : "reading inner text")
                                    + ": "
                                    + event.getError().getMessage()
                    );
                    default -> {
                    }
                }
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
            return new Locator(page, chainSelectorForTransport(selector, childSelector));
        }

        public ClickResult click() {
            return page.click(selector);
        }

        public ClickResult click(CommandOptions options) {
            return page.click(selector, options);
        }

        public CountResult count() {
            return page.count(selector);
        }

        public CountResult count(CommandOptions options) {
            return page.count(selector, options);
        }

        public HighlightResult highlight() {
            return page.highlight(selector);
        }

        public HighlightResult highlight(HighlightOptions options) {
            return page.highlight(selector, options);
        }

        public ElementResult focus() {
            return page.focus(selector);
        }

        public ElementResult focus(CommandOptions options) {
            return page.focus(selector, options);
        }

        public FillResult fill(String value) {
            return page.fill(selector, value);
        }

        public FillResult fill(String value, CommandOptions options) {
            return page.fill(selector, value, options);
        }

        public ElementResult hover() {
            return page.hover(selector);
        }

        public ElementResult hover(CommandOptions options) {
            return page.hover(selector, options);
        }

        public PressResult press(String key) {
            return page.press(selector, key);
        }

        public PressResult press(String key, PressOptions options) {
            return page.press(selector, key, options);
        }

        public TextResult textContent() {
            return page.textContent(selector);
        }

        public TextResult textContent(CommandOptions options) {
            return page.textContent(selector, options);
        }

        public TextResult innerText() {
            return page.innerText(selector);
        }

        public TextResult innerText(CommandOptions options) {
            return page.innerText(selector, options);
        }

        public WaitForSelectorResult waitFor() {
            return page.waitForSelector(selector);
        }

        public WaitForSelectorResult waitFor(WaitForSelectorOptions options) {
            return page.waitForSelector(selector, options);
        }
    }

    private record SelectorTransport(String kind, String body) {}

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
