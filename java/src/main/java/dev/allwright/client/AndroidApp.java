package dev.allwright.client;

import dev.allwright.engine.v1.ClickElementCommand;
import dev.allwright.engine.v1.FillElementCommand;
import dev.allwright.engine.v1.ContextSessionCommand;
import dev.allwright.engine.v1.ContextSessionEvent;
import dev.allwright.engine.v1.ScreenshotCommand;

public final class AndroidApp {
    private final RuntimeSupport.RuntimeClient runtime;
    private final String surfaceSessionId;
    private final String sessionId;
    private RuntimeSupport.StreamHandle<ContextSessionCommand, ContextSessionEvent> stream;
    private boolean closed;

    AndroidApp(RuntimeSupport.RuntimeClient runtime, String surfaceSessionId, String sessionId) {
        this.runtime = runtime;
        this.surfaceSessionId = surfaceSessionId;
        this.sessionId = sessionId;
    }

    public String sessionId() {
        return sessionId;
    }

    public AndroidLocator locator(String selector) {
        return new AndroidLocator(this, AndroidSelectorSupport.normalizeSelectorForTransport(selector));
    }

    public synchronized ClickResult click(String selector) {
        return click(selector, new CommandOptions());
    }

    public synchronized ClickResult click(String selector, CommandOptions options) {
        RuntimeSupport.StreamHandle<ContextSessionCommand, ContextSessionEvent> handle = ensureStream();
        ensureOpen();
        CommandOptions resolvedOptions = options == null ? new CommandOptions() : options;
        ClickElementCommand.Builder click = ClickElementCommand.newBuilder()
                .setCssSelector(AndroidSelectorSupport.normalizeSelectorForTransport(selector));
        if (CommandSupport.hasTimeout(resolvedOptions.timeoutMs())) {
            click.setRetryOptions(CommandSupport.commandRetryOptions(resolvedOptions.timeoutMs()));
        }
        handle.send(
                ContextSessionCommand.newBuilder()
                        .setSurfaceSessionId(surfaceSessionId)
                        .setContextSessionId(sessionId)
                        .setClickElement(click)
                        .build()
        );

        while (true) {
            ContextSessionEvent event = handle.recv("receive tab session event while clicking Android element");
            switch (event.getEventCase()) {
                case ATTACHED -> {
                }
                case ELEMENT_CLICKED -> {
                    return new ClickResult(
                            event.getElementClicked().getCssSelector(),
                            event.getElementClicked().getNote(),
                            event.getElementClicked().getBidiSessionId()
                    );
                }
                case CLOSED -> {
                    closed = true;
                    throw new AllwrightException("android app session " + sessionId + " closed while clicking");
                }
                case ERROR -> throw new AllwrightException(
                        "android app session error while clicking: " + event.getError().getMessage()
                );
                default -> {
                }
            }
        }
    }

    public synchronized FillResult fill(String selector, String value) {
        return fill(selector, value, new CommandOptions());
    }

    public synchronized FillResult fill(String selector, String value, CommandOptions options) {
        RuntimeSupport.StreamHandle<ContextSessionCommand, ContextSessionEvent> handle = ensureStream();
        ensureOpen();
        CommandOptions resolvedOptions = options == null ? new CommandOptions() : options;
        FillElementCommand.Builder fill = FillElementCommand.newBuilder()
                .setCssSelector(AndroidSelectorSupport.normalizeSelectorForTransport(selector))
                .setValue(value);
        if (CommandSupport.hasTimeout(resolvedOptions.timeoutMs())) {
            fill.setRetryOptions(CommandSupport.commandRetryOptions(resolvedOptions.timeoutMs()));
        }
        handle.send(
                ContextSessionCommand.newBuilder()
                        .setSurfaceSessionId(surfaceSessionId)
                        .setContextSessionId(sessionId)
                        .setFillElement(fill)
                        .build()
        );

        while (true) {
            ContextSessionEvent event = handle.recv("receive tab session event while filling Android element");
            switch (event.getEventCase()) {
                case ATTACHED -> {
                }
                case ELEMENT_FILLED -> {
                    return new FillResult(
                            event.getElementFilled().getCssSelector(),
                            event.getElementFilled().getValue(),
                            event.getElementFilled().getNote()
                    );
                }
                case CLOSED -> {
                    closed = true;
                    throw new AllwrightException("android app session " + sessionId + " closed while filling");
                }
                case ERROR -> throw new AllwrightException(
                        "android app session error while filling: " + event.getError().getMessage()
                );
                default -> {
                }
            }
        }
    }

    public synchronized ScreenshotResult screenshot() {
        return screenshot(new CommandOptions());
    }

    public synchronized ScreenshotResult screenshot(CommandOptions options) {
        RuntimeSupport.StreamHandle<ContextSessionCommand, ContextSessionEvent> handle = ensureStream();
        ensureOpen();
        CommandOptions resolvedOptions = options == null ? new CommandOptions() : options;
        ScreenshotCommand.Builder screenshot = ScreenshotCommand.newBuilder();
        if (CommandSupport.hasTimeout(resolvedOptions.timeoutMs())) {
            screenshot.setRetryOptions(CommandSupport.commandRetryOptions(resolvedOptions.timeoutMs()));
        }
        handle.send(
                ContextSessionCommand.newBuilder()
                        .setSurfaceSessionId(surfaceSessionId)
                        .setContextSessionId(sessionId)
                        .setScreenshot(screenshot)
                        .build()
        );

        while (true) {
            ContextSessionEvent event = handle.recv("receive tab session event while capturing Android screenshot");
            switch (event.getEventCase()) {
                case ATTACHED -> {
                }
                case SCREENSHOT_CAPTURED -> {
                    return new ScreenshotResult(
                            event.getScreenshotCaptured().getPngData().toByteArray(),
                            event.getScreenshotCaptured().getNote()
                    );
                }
                case CLOSED -> {
                    closed = true;
                    throw new AllwrightException("android app session " + sessionId + " closed while capturing screenshot");
                }
                case ERROR -> throw new AllwrightException(
                        "android app session error while capturing screenshot: " + event.getError().getMessage()
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

    private void ensureOpen() {
        if (closed) {
            throw new AllwrightException("android app session " + sessionId + " is closed");
        }
    }
}
