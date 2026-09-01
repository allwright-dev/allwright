package dev.allwright.client;

import java.io.IOException;
import java.nio.file.Files;

import dev.allwright.engine.v1.ClickElementCommand;
import dev.allwright.engine.v1.CloseContextSessionCommand;
import dev.allwright.engine.v1.CountElementsCommand;
import dev.allwright.engine.v1.FillElementCommand;
import dev.allwright.engine.v1.FocusElementCommand;
import dev.allwright.engine.v1.GetInnerTextCommand;
import dev.allwright.engine.v1.GetTextContentCommand;
import dev.allwright.engine.v1.HighlightElementsCommand;
import dev.allwright.engine.v1.HoverElementCommand;
import dev.allwright.engine.v1.NavigatePageCommand;
import dev.allwright.engine.v1.PressKeyCommand;
import dev.allwright.engine.v1.ContextSessionCommand;
import dev.allwright.engine.v1.ContextSessionEvent;
import dev.allwright.engine.v1.ContextSessionPingCommand;
import dev.allwright.engine.v1.ScreenshotCommand;
import dev.allwright.engine.v1.WaitForSelectorCommand;

public final class Page implements AutoCloseable {
    private final RuntimeSupport.RuntimeClient runtime;
    private final String browserSessionId;
    private final String sessionId;
    private RuntimeSupport.StreamHandle<ContextSessionCommand, ContextSessionEvent> stream;
    private boolean closed;

    Page(RuntimeSupport.RuntimeClient runtime, String browserSessionId, String sessionId) {
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
        return new Locator(this, SelectorSupport.normalizeSelectorForTransport(selector));
    }

    public synchronized NavigateResult goTo(String url) {
        return goTo(url, new CommandOptions());
    }

    public synchronized NavigateResult goTo(String url, CommandOptions options) {
        RuntimeSupport.StreamHandle<ContextSessionCommand, ContextSessionEvent> handle = ensureStream();
        ensureOpen();
        CommandOptions resolvedOptions = options == null ? new CommandOptions() : options;
        NavigatePageCommand.Builder navigate = NavigatePageCommand.newBuilder().setUrl(url);
        if (CommandSupport.hasTimeout(resolvedOptions.timeoutMs())) {
            navigate.setRetryOptions(CommandSupport.commandRetryOptions(resolvedOptions.timeoutMs()));
        }
        handle.send(
                ContextSessionCommand.newBuilder()
                        .setSurfaceSessionId(browserSessionId)
                        .setContextSessionId(sessionId)
                        .setNavigate(navigate)
                        .build()
        );

        String navigatedUrl = null;
        String navigatedNote = null;

        while (true) {
            ContextSessionEvent event = handle.recv("receive tab session event while navigating");
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
        RuntimeSupport.StreamHandle<ContextSessionCommand, ContextSessionEvent> handle = ensureStream();
        ensureOpen();
        CommandOptions resolvedOptions = options == null ? new CommandOptions() : options;
        String transportSelector = SelectorSupport.normalizeSelectorForTransport(selector);
        ClickElementCommand.Builder click = ClickElementCommand.newBuilder().setCssSelector(transportSelector);
        if (CommandSupport.hasTimeout(resolvedOptions.timeoutMs())) {
            click.setRetryOptions(CommandSupport.commandRetryOptions(resolvedOptions.timeoutMs()));
        }
        handle.send(
                ContextSessionCommand.newBuilder()
                        .setSurfaceSessionId(browserSessionId)
                        .setContextSessionId(sessionId)
                        .setClickElement(click)
                        .build()
        );

        while (true) {
            ContextSessionEvent event = handle.recv("receive tab session event while clicking");
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
        RuntimeSupport.StreamHandle<ContextSessionCommand, ContextSessionEvent> handle = ensureStream();
        ensureOpen();
        CommandOptions resolvedOptions = options == null ? new CommandOptions() : options;
        String transportSelector = SelectorSupport.normalizeSelectorForTransport(selector);
        CountElementsCommand.Builder count = CountElementsCommand.newBuilder().setCssSelector(transportSelector);
        if (CommandSupport.hasTimeout(resolvedOptions.timeoutMs())) {
            count.setRetryOptions(CommandSupport.commandRetryOptions(resolvedOptions.timeoutMs()));
        }
        handle.send(
                ContextSessionCommand.newBuilder()
                        .setSurfaceSessionId(browserSessionId)
                        .setContextSessionId(sessionId)
                        .setCountElements(count)
                        .build()
        );

        while (true) {
            ContextSessionEvent event = handle.recv("receive tab session event while counting elements");
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
        RuntimeSupport.StreamHandle<ContextSessionCommand, ContextSessionEvent> handle = ensureStream();
        ensureOpen();
        HighlightOptions resolvedOptions = options == null ? new HighlightOptions() : options;
        String transportSelector = SelectorSupport.normalizeSelectorForTransport(selector);
        HighlightElementsCommand.Builder highlight =
                HighlightElementsCommand.newBuilder().setCssSelector(transportSelector);
        if (resolvedOptions.durationMs() != null && resolvedOptions.durationMs() > 0) {
            highlight.setDurationMs(resolvedOptions.durationMs());
        }
        if (CommandSupport.hasTimeout(resolvedOptions.timeoutMs())) {
            highlight.setRetryOptions(CommandSupport.commandRetryOptions(resolvedOptions.timeoutMs()));
        }
        handle.send(
                ContextSessionCommand.newBuilder()
                        .setSurfaceSessionId(browserSessionId)
                        .setContextSessionId(sessionId)
                        .setHighlightElements(highlight)
                        .build()
        );

        while (true) {
            ContextSessionEvent event = handle.recv("receive tab session event while highlighting elements");
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
                    throw new AllwrightException("page session " + sessionId + " closed while highlighting elements");
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
        FocusElementCommand.Builder focus = FocusElementCommand.newBuilder()
                .setCssSelector(SelectorSupport.normalizeSelectorForTransport(selector));
        if (CommandSupport.hasTimeout(resolvedOptions.timeoutMs())) {
            focus.setRetryOptions(CommandSupport.commandRetryOptions(resolvedOptions.timeoutMs()));
        }
        return performElementCommand(
                "focusing",
                ContextSessionCommand.newBuilder()
                        .setSurfaceSessionId(browserSessionId)
                        .setContextSessionId(sessionId)
                        .setFocusElement(focus)
                        .build(),
                ContextSessionEvent.EventCase.ELEMENT_FOCUSED
        );
    }

    public synchronized FillResult fill(String selector, String value) {
        return fill(selector, value, new CommandOptions());
    }

    public synchronized FillResult fill(String selector, String value, CommandOptions options) {
        RuntimeSupport.StreamHandle<ContextSessionCommand, ContextSessionEvent> handle = ensureStream();
        ensureOpen();
        CommandOptions resolvedOptions = options == null ? new CommandOptions() : options;
        FillElementCommand.Builder fill = FillElementCommand.newBuilder()
                .setCssSelector(SelectorSupport.normalizeSelectorForTransport(selector))
                .setValue(value);
        if (CommandSupport.hasTimeout(resolvedOptions.timeoutMs())) {
            fill.setRetryOptions(CommandSupport.commandRetryOptions(resolvedOptions.timeoutMs()));
        }
        handle.send(
                ContextSessionCommand.newBuilder()
                        .setSurfaceSessionId(browserSessionId)
                        .setContextSessionId(sessionId)
                        .setFillElement(fill)
                        .build()
        );
        while (true) {
            ContextSessionEvent event = handle.recv("receive tab session event while filling");
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
        HoverElementCommand.Builder hover = HoverElementCommand.newBuilder()
                .setCssSelector(SelectorSupport.normalizeSelectorForTransport(selector));
        if (CommandSupport.hasTimeout(resolvedOptions.timeoutMs())) {
            hover.setRetryOptions(CommandSupport.commandRetryOptions(resolvedOptions.timeoutMs()));
        }
        return performElementCommand(
                "hovering",
                ContextSessionCommand.newBuilder()
                        .setSurfaceSessionId(browserSessionId)
                        .setContextSessionId(sessionId)
                        .setHoverElement(hover)
                        .build(),
                ContextSessionEvent.EventCase.ELEMENT_HOVERED
        );
    }

    public synchronized PressResult press(String selector, String key) {
        return press(selector, key, new PressOptions());
    }

    public synchronized PressResult press(String selector, String key, PressOptions options) {
        RuntimeSupport.StreamHandle<ContextSessionCommand, ContextSessionEvent> handle = ensureStream();
        ensureOpen();
        PressOptions resolvedOptions = options == null ? new PressOptions() : options;
        PressKeyCommand.Builder press = PressKeyCommand.newBuilder()
                .setCssSelector(SelectorSupport.normalizeSelectorForTransport(selector))
                .setKey(key);
        if (resolvedOptions.text() != null) {
            press.setText(resolvedOptions.text());
        }
        if (CommandSupport.hasTimeout(resolvedOptions.timeoutMs())) {
            press.setRetryOptions(CommandSupport.commandRetryOptions(resolvedOptions.timeoutMs()));
        }
        handle.send(
                ContextSessionCommand.newBuilder()
                        .setSurfaceSessionId(browserSessionId)
                        .setContextSessionId(sessionId)
                        .setPressKey(press)
                        .build()
        );
        while (true) {
            ContextSessionEvent event = handle.recv("receive tab session event while pressing key");
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
        RuntimeSupport.StreamHandle<ContextSessionCommand, ContextSessionEvent> handle = ensureStream();
        ensureOpen();
        WaitForSelectorOptions resolvedOptions = options == null ? new WaitForSelectorOptions() : options;
        WaitForSelectorCommand.Builder waitFor = WaitForSelectorCommand.newBuilder()
                .setCssSelector(SelectorSupport.normalizeSelectorForTransport(selector));
        if (resolvedOptions.visible() != null) {
            waitFor.setVisible(resolvedOptions.visible());
        }
        if (CommandSupport.hasTimeout(resolvedOptions.timeoutMs())) {
            waitFor.setRetryOptions(CommandSupport.commandRetryOptions(resolvedOptions.timeoutMs()));
        }
        handle.send(
                ContextSessionCommand.newBuilder()
                        .setSurfaceSessionId(browserSessionId)
                        .setContextSessionId(sessionId)
                        .setWaitForSelector(waitFor)
                        .build()
        );
        while (true) {
            ContextSessionEvent event = handle.recv("receive tab session event while waiting for selector");
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
                    throw new AllwrightException("page session " + sessionId + " closed while waiting for selector");
                }
                case ERROR -> throw new AllwrightException(
                        "page session error while waiting for selector: " + event.getError().getMessage()
                );
                default -> {
                }
            }
        }
    }

    public synchronized ScreenshotResult screenshot() {
        return screenshot(new ScreenshotOptions());
    }

    public synchronized ScreenshotResult screenshot(ScreenshotOptions options) {
        RuntimeSupport.StreamHandle<ContextSessionCommand, ContextSessionEvent> handle = ensureStream();
        ensureOpen();
        ScreenshotOptions resolvedOptions = options == null ? new ScreenshotOptions() : options;
        ScreenshotCommand.Builder screenshot = ScreenshotCommand.newBuilder();
        if (CommandSupport.hasTimeout(resolvedOptions.timeoutMs())) {
            screenshot.setRetryOptions(CommandSupport.commandRetryOptions(resolvedOptions.timeoutMs()));
        }
        if (resolvedOptions.fullPage()) {
            screenshot.setFullPage(true);
        }
        handle.send(
                ContextSessionCommand.newBuilder()
                        .setSurfaceSessionId(browserSessionId)
                        .setContextSessionId(sessionId)
                        .setScreenshot(screenshot)
                        .build()
        );
        while (true) {
            ContextSessionEvent event = handle.recv("receive tab session event while capturing screenshot");
            switch (event.getEventCase()) {
                case SCREENSHOT_CAPTURED -> {
                    ScreenshotResult result = new ScreenshotResult(
                            event.getScreenshotCaptured().getPngData().toByteArray(),
                            event.getScreenshotCaptured().getNote()
                    );
                    if (resolvedOptions.path() != null) {
                        try {
                            Files.write(resolvedOptions.path(), result.pngData());
                        } catch (IOException error) {
                            throw new AllwrightException(
                                    "write screenshot to " + resolvedOptions.path(),
                                    error
                            );
                        }
                    }
                    return result;
                }
                case CLOSED -> {
                    closed = true;
                    throw new AllwrightException("page session " + sessionId + " closed while capturing screenshot");
                }
                case ERROR -> throw new AllwrightException(
                        "page session error while capturing screenshot: " + event.getError().getMessage()
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
        RuntimeSupport.StreamHandle<ContextSessionCommand, ContextSessionEvent> handle = ensureStream();
        ensureOpen();
        handle.send(
                ContextSessionCommand.newBuilder()
                        .setSurfaceSessionId(browserSessionId)
                        .setContextSessionId(sessionId)
                        .setPing(ContextSessionPingCommand.newBuilder().setMessage(message).build())
                        .build()
        );

        while (true) {
            ContextSessionEvent event = handle.recv("receive tab session event while pinging page");
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

    @Override
    public synchronized void close() {
        RuntimeSupport.StreamHandle<ContextSessionCommand, ContextSessionEvent> handle = ensureStream();
        if (closed) {
            return;
        }

        handle.send(
                ContextSessionCommand.newBuilder()
                        .setSurfaceSessionId(browserSessionId)
                        .setContextSessionId(sessionId)
                        .setClose(CloseContextSessionCommand.newBuilder().build())
                        .build()
        );

        while (true) {
            ContextSessionEvent event = handle.recv("receive tab session event while closing page");
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
            ContextSessionCommand command,
            ContextSessionEvent.EventCase successCase
    ) {
        RuntimeSupport.StreamHandle<ContextSessionCommand, ContextSessionEvent> handle = ensureStream();
        ensureOpen();
        handle.send(command);

        while (true) {
            ContextSessionEvent event = handle.recv("receive tab session event while " + action);
            switch (event.getEventCase()) {
                case ELEMENT_FOCUSED -> {
                    if (successCase == ContextSessionEvent.EventCase.ELEMENT_FOCUSED) {
                        return new ElementResult(
                                event.getElementFocused().getCssSelector(),
                                event.getElementFocused().getNote()
                        );
                    }
                }
                case ELEMENT_HOVERED -> {
                    if (successCase == ContextSessionEvent.EventCase.ELEMENT_HOVERED) {
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
        RuntimeSupport.StreamHandle<ContextSessionCommand, ContextSessionEvent> handle = ensureStream();
        ensureOpen();
        CommandOptions resolvedOptions = options == null ? new CommandOptions() : options;
        String transportSelector = SelectorSupport.normalizeSelectorForTransport(selector);
        ContextSessionCommand.Builder command = ContextSessionCommand.newBuilder()
                .setSurfaceSessionId(browserSessionId)
                .setContextSessionId(sessionId);
        if (textContent) {
            GetTextContentCommand.Builder getTextContent =
                    GetTextContentCommand.newBuilder().setCssSelector(transportSelector);
            if (CommandSupport.hasTimeout(resolvedOptions.timeoutMs())) {
                getTextContent.setRetryOptions(CommandSupport.commandRetryOptions(resolvedOptions.timeoutMs()));
            }
            command.setGetTextContent(getTextContent);
        } else {
            GetInnerTextCommand.Builder getInnerText =
                    GetInnerTextCommand.newBuilder().setCssSelector(transportSelector);
            if (CommandSupport.hasTimeout(resolvedOptions.timeoutMs())) {
                getInnerText.setRetryOptions(CommandSupport.commandRetryOptions(resolvedOptions.timeoutMs()));
            }
            command.setGetInnerText(getInnerText);
        }
        handle.send(command.build());

        while (true) {
            ContextSessionEvent event = handle.recv(
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

    private RuntimeSupport.StreamHandle<ContextSessionCommand, ContextSessionEvent> ensureStream() {
        if (stream == null) {
            stream = new RuntimeSupport.StreamHandle<>(runtime.asyncStub()::contextSession);
        }
        return stream;
    }
}
