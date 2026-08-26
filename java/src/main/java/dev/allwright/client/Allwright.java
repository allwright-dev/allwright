package dev.allwright.client;

import allwright.engine.v1.EngineServiceGrpc;
import allwright.engine.v1.Engine.*;
import io.grpc.ManagedChannel;
import io.grpc.ManagedChannelBuilder;
import io.grpc.StatusRuntimeException;
import io.grpc.stub.StreamObserver;
import java.util.Map;
import java.util.Objects;
import java.util.concurrent.BlockingQueue;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.LinkedBlockingQueue;

public final class Allwright {
    public static final String DEFAULT_SERVER_ADDR = "127.0.0.1:50051";
    public static final String SERVER_ADDR_ENV_VAR = "ALLWRIGHT_SERVER_ADDR";

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
