package dev.allwright.client;

import dev.allwright.engine.v1.SurfaceSessionClosedEvent;
import dev.allwright.engine.v1.SurfaceSessionCommand;
import dev.allwright.engine.v1.SurfaceSessionEvent;
import dev.allwright.engine.v1.CloseSurfaceSessionCommand;
import dev.allwright.engine.v1.OpenContextCommand;
import dev.allwright.engine.v1.SessionPingCommand;
import dev.allwright.engine.v1.ContextOpenedEvent;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

public final class Browser implements AutoCloseable {
    private final RuntimeSupport.RuntimeClient runtime;
    private final RuntimeSupport.StreamHandle<SurfaceSessionCommand, SurfaceSessionEvent> stream;
    private final Map<String, Page> pages = new LinkedHashMap<>();
    private final Page initialPage;
    private boolean closed;

    private final String sessionId;
    private final String browserName;
    private final String launchNote;
    private final String cdpWebSocketURL;
    private final String userDataDir;

    Browser(
            RuntimeSupport.RuntimeClient runtime,
            RuntimeSupport.StreamHandle<SurfaceSessionCommand, SurfaceSessionEvent> stream,
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
        OpenContextCommand.Builder openContext = OpenContextCommand.newBuilder();
        if (CommandSupport.hasTimeout(resolvedOptions.timeoutMs())) {
            openContext.setRetryOptions(CommandSupport.commandRetryOptions(resolvedOptions.timeoutMs()));
        }
        stream.send(SurfaceSessionCommand.newBuilder().setOpenContext(openContext).build());

        while (true) {
            SurfaceSessionEvent event = stream.recv("receive browser session event while opening page");
            switch (event.getEventCase()) {
                case CONTEXT_OPENED -> {
                    ContextOpenedEvent opened = event.getContextOpened();
                    Page page = new Page(runtime, sessionId, opened.getContextSessionId());
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
                SurfaceSessionCommand.newBuilder()
                        .setPing(SessionPingCommand.newBuilder().setMessage(message).build())
                        .build()
        );

        while (true) {
            SurfaceSessionEvent event = stream.recv("receive browser session event while pinging browser");
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

    @Override
    public synchronized void close() {
        if (closed) {
            return;
        }

        stream.send(
                SurfaceSessionCommand.newBuilder()
                        .setClose(CloseSurfaceSessionCommand.newBuilder().build())
                        .build()
        );

        while (true) {
            SurfaceSessionEvent event = stream.recv("receive browser session event while closing browser");
            switch (event.getEventCase()) {
                case CLOSED -> {
                    SurfaceSessionClosedEvent ignored = event.getClosed();
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
