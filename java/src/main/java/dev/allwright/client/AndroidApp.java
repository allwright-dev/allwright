package dev.allwright.client;

import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.StandardCopyOption;
import java.util.UUID;
import com.google.protobuf.ByteString;

import dev.allwright.engine.v1.ClickElementCommand;
import dev.allwright.engine.v1.CountElementsCommand;
import dev.allwright.engine.v1.ContextSessionCommand;
import dev.allwright.engine.v1.ContextSessionEvent;
import dev.allwright.engine.v1.FillElementCommand;
import dev.allwright.engine.v1.FocusElementCommand;
import dev.allwright.engine.v1.GetInnerTextCommand;
import dev.allwright.engine.v1.GetTextContentCommand;
import dev.allwright.engine.v1.PressKeyCommand;
import dev.allwright.engine.v1.ScreenshotCommand;
import dev.allwright.engine.v1.WaitForSelectorCommand;
import dev.allwright.engine.v1.HookCompletedEvent;
import dev.allwright.engine.v1.RegisterHookCommand;
import dev.allwright.engine.v1.RegisterMobileFileChooserHook;
import dev.allwright.engine.v1.RegisterMobileDownloadHook;
import dev.allwright.engine.v1.WaitForHookCommand;
import dev.allwright.engine.v1.SetMobileFileChooserFilesCommand;
import dev.allwright.engine.v1.SaveMobileDownloadCommand;
import dev.allwright.engine.v1.UploadFileChunkCommand;
import dev.allwright.engine.v1.ReadFileChunkCommand;

public final class AndroidApp implements HookContext {
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

    public synchronized CountResult count(String selector) {
        return count(selector, new CommandOptions());
    }

    public synchronized CountResult count(String selector, CommandOptions options) {
        RuntimeSupport.StreamHandle<ContextSessionCommand, ContextSessionEvent> handle = ensureStream();
        ensureOpen();
        CommandOptions resolvedOptions = options == null ? new CommandOptions() : options;
        CountElementsCommand.Builder count = CountElementsCommand.newBuilder()
                .setCssSelector(AndroidSelectorSupport.normalizeSelectorForTransport(selector));
        if (CommandSupport.hasTimeout(resolvedOptions.timeoutMs())) {
            count.setRetryOptions(CommandSupport.commandRetryOptions(resolvedOptions.timeoutMs()));
        }
        handle.send(
                ContextSessionCommand.newBuilder()
                        .setSurfaceSessionId(surfaceSessionId)
                        .setContextSessionId(sessionId)
                        .setCountElements(count)
                        .build()
        );

        while (true) {
            ContextSessionEvent event = handle.recv("receive tab session event while counting Android elements");
            switch (event.getEventCase()) {
                case ATTACHED -> {
                }
                case ELEMENT_COUNTED -> {
                    return new CountResult(
                            event.getElementCounted().getCssSelector(),
                            event.getElementCounted().getCount(),
                            event.getElementCounted().getNote()
                    );
                }
                case CLOSED -> {
                    closed = true;
                    throw new AllwrightException("android app session " + sessionId + " closed while counting elements");
                }
                case ERROR -> throw new AllwrightException(
                        "android app session error while counting elements: " + event.getError().getMessage()
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
        RuntimeSupport.StreamHandle<ContextSessionCommand, ContextSessionEvent> handle = ensureStream();
        ensureOpen();
        CommandOptions resolvedOptions = options == null ? new CommandOptions() : options;
        FocusElementCommand.Builder focus = FocusElementCommand.newBuilder()
                .setCssSelector(AndroidSelectorSupport.normalizeSelectorForTransport(selector));
        if (CommandSupport.hasTimeout(resolvedOptions.timeoutMs())) {
            focus.setRetryOptions(CommandSupport.commandRetryOptions(resolvedOptions.timeoutMs()));
        }
        handle.send(
                ContextSessionCommand.newBuilder()
                        .setSurfaceSessionId(surfaceSessionId)
                        .setContextSessionId(sessionId)
                        .setFocusElement(focus)
                        .build()
        );

        while (true) {
            ContextSessionEvent event = handle.recv("receive tab session event while focusing Android element");
            switch (event.getEventCase()) {
                case ATTACHED -> {
                }
                case ELEMENT_FOCUSED -> {
                    return new ElementResult(
                            event.getElementFocused().getCssSelector(),
                            event.getElementFocused().getNote()
                    );
                }
                case CLOSED -> {
                    closed = true;
                    throw new AllwrightException("android app session " + sessionId + " closed while focusing");
                }
                case ERROR -> throw new AllwrightException(
                        "android app session error while focusing: " + event.getError().getMessage()
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

    public synchronized PressResult press(String selector, String key) {
        return press(selector, key, new PressOptions());
    }

    public synchronized PressResult press(String selector, String key, PressOptions options) {
        RuntimeSupport.StreamHandle<ContextSessionCommand, ContextSessionEvent> handle = ensureStream();
        ensureOpen();
        PressOptions resolvedOptions = options == null ? new PressOptions() : options;
        PressKeyCommand.Builder press = PressKeyCommand.newBuilder()
                .setCssSelector(AndroidSelectorSupport.normalizeSelectorForTransport(selector))
                .setKey(key);
        if (resolvedOptions.text() != null) {
            press.setText(resolvedOptions.text());
        }
        if (CommandSupport.hasTimeout(resolvedOptions.timeoutMs())) {
            press.setRetryOptions(CommandSupport.commandRetryOptions(resolvedOptions.timeoutMs()));
        }
        handle.send(
                ContextSessionCommand.newBuilder()
                        .setSurfaceSessionId(surfaceSessionId)
                        .setContextSessionId(sessionId)
                        .setPressKey(press)
                        .build()
        );

        while (true) {
            ContextSessionEvent event = handle.recv("receive tab session event while pressing Android key");
            switch (event.getEventCase()) {
                case ATTACHED -> {
                }
                case KEY_PRESSED -> {
                    return new PressResult(
                            event.getKeyPressed().getCssSelector(),
                            event.getKeyPressed().getKey(),
                            event.getKeyPressed().getNote()
                    );
                }
                case CLOSED -> {
                    closed = true;
                    throw new AllwrightException("android app session " + sessionId + " closed while pressing key");
                }
                case ERROR -> throw new AllwrightException(
                        "android app session error while pressing key: " + event.getError().getMessage()
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
                .setCssSelector(AndroidSelectorSupport.normalizeSelectorForTransport(selector));
        if (resolvedOptions.visible() != null) {
            waitFor.setVisible(resolvedOptions.visible());
        }
        if (CommandSupport.hasTimeout(resolvedOptions.timeoutMs())) {
            waitFor.setRetryOptions(CommandSupport.commandRetryOptions(resolvedOptions.timeoutMs()));
        }
        handle.send(
                ContextSessionCommand.newBuilder()
                        .setSurfaceSessionId(surfaceSessionId)
                        .setContextSessionId(sessionId)
                        .setWaitForSelector(waitFor)
                        .build()
        );

        while (true) {
            ContextSessionEvent event = handle.recv("receive tab session event while waiting for Android selector");
            switch (event.getEventCase()) {
                case ATTACHED -> {
                }
                case SELECTOR_WAIT_SATISFIED -> {
                    return new WaitForSelectorResult(
                            event.getSelectorWaitSatisfied().getCssSelector(),
                            event.getSelectorWaitSatisfied().getVisible(),
                            event.getSelectorWaitSatisfied().getNote()
                    );
                }
                case CLOSED -> {
                    closed = true;
                    throw new AllwrightException("android app session " + sessionId + " closed while waiting for selector");
                }
                case ERROR -> throw new AllwrightException(
                        "android app session error while waiting for selector: " + event.getError().getMessage()
                );
                default -> {
                }
            }
        }
    }

    public synchronized String accessibilitySnapshot() {
        return accessibilitySnapshot(new AccessibilitySnapshotOptions());
    }

    public synchronized String accessibilitySnapshot(AccessibilitySnapshotOptions options) {
        RuntimeSupport.StreamHandle<ContextSessionCommand, ContextSessionEvent> handle = ensureStream();
        ensureOpen();
        AccessibilitySnapshotOptions resolved = options == null ? new AccessibilitySnapshotOptions() : options;
        var command = dev.allwright.engine.v1.AccessibilitySnapshotCommand.newBuilder().setFormat(resolved.format()).setMode(resolved.mode());
        if (CommandSupport.hasTimeout(resolved.timeoutMs())) {
            command.setRetryOptions(CommandSupport.commandRetryOptions(resolved.timeoutMs()));
        }
        handle.send(ContextSessionCommand.newBuilder()
                .setSurfaceSessionId(surfaceSessionId)
                .setContextSessionId(sessionId)
                .setAccessibilitySnapshot(command)
                .build());
        while (true) {
            ContextSessionEvent event = handle.recv("receive accessibility snapshot");
            switch (event.getEventCase()) {
                case ACCESSIBILITY_SNAPSHOT_CAPTURED -> { return event.getAccessibilitySnapshotCaptured().getSnapshot(); }
                case CLOSED -> {
                    closed = true;
                    throw new AllwrightException("android app session " + sessionId + " closed while capturing accessibility snapshot");
                }
                case ERROR -> throw new AllwrightException("accessibility snapshot failed: " + event.getError().getMessage());
                default -> { }
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

    public synchronized <T> Hook<T> registerHook(HookType<T> type) {
        if (!"file_chooser".equals(type.name()) && !"download".equals(type.name())) {
            throw new AllwrightException("hook type " + type.name() + " is not supported by Android apps");
        }
        var handle = ensureStream();
        RegisterHookCommand register = "file_chooser".equals(type.name())
                ? RegisterHookCommand.newBuilder().setMobileFileChooser(RegisterMobileFileChooserHook.newBuilder()).build()
                : RegisterHookCommand.newBuilder().setMobileDownload(RegisterMobileDownloadHook.newBuilder()).build();
        handle.send(ContextSessionCommand.newBuilder().setSurfaceSessionId(surfaceSessionId)
                .setContextSessionId(sessionId).setRegisterHook(register).build());
        while (true) {
            ContextSessionEvent event = handle.recv("register Android hook");
            if (event.getEventCase() == ContextSessionEvent.EventCase.HOOK_REGISTERED) {
                return new Hook<>(this, event.getHookRegistered().getHookId(), type);
            }
            if (event.getEventCase() == ContextSessionEvent.EventCase.ERROR) {
                throw new AllwrightException(event.getError().getMessage());
            }
        }
    }

    @Override
    public synchronized <T> T waitForHook(String hookId, HookType<T> type, CommandOptions options) {
        var handle = ensureStream();
        WaitForHookCommand.Builder wait = WaitForHookCommand.newBuilder().setHookId(hookId);
        if (options != null && CommandSupport.hasTimeout(options.timeoutMs())) {
            wait.setRetryOptions(CommandSupport.commandRetryOptions(options.timeoutMs()));
        }
        handle.send(ContextSessionCommand.newBuilder().setSurfaceSessionId(surfaceSessionId)
                .setContextSessionId(sessionId).setWaitForHook(wait).build());
        while (true) {
            ContextSessionEvent event = handle.recv("wait for Android hook");
            if (event.getEventCase() == ContextSessionEvent.EventCase.HOOK_COMPLETED
                    && hookId.equals(event.getHookCompleted().getHookId())) {
                return type.decode(this, event.getHookCompleted());
            }
            if (event.getEventCase() == ContextSessionEvent.EventCase.ERROR) {
                throw new AllwrightException(event.getError().getMessage());
            }
        }
    }

    @Override
    public synchronized void setHookFileChooserFiles(
            String chooserId, java.util.List<String> files, CommandOptions options
    ) {
        var handle = ensureStream();
        java.util.List<String> fileIds = new java.util.ArrayList<>();
        for (String file : files) fileIds.add(uploadLocalFile(handle, Path.of(file)));
        SetMobileFileChooserFilesCommand.Builder command = SetMobileFileChooserFilesCommand.newBuilder()
                .setFileChooserId(chooserId).addAllFileIds(fileIds);
        if (options != null && CommandSupport.hasTimeout(options.timeoutMs())) {
            command.setRetryOptions(CommandSupport.commandRetryOptions(options.timeoutMs()));
        }
        handle.send(ContextSessionCommand.newBuilder().setSurfaceSessionId(surfaceSessionId)
                .setContextSessionId(sessionId).setSetMobileFileChooserFiles(command).build());
        while (true) {
            ContextSessionEvent event = handle.recv("set Android file chooser files");
            if (event.getEventCase() == ContextSessionEvent.EventCase.MOBILE_FILE_CHOOSER_FILES_SET
                    && chooserId.equals(event.getMobileFileChooserFilesSet().getFileChooserId())) return;
            if (event.getEventCase() == ContextSessionEvent.EventCase.ERROR) {
                throw new AllwrightException(event.getError().getMessage());
            }
        }
    }

    @Override
    public synchronized void saveHookDownload(String downloadId, String path, CommandOptions options) {
        var handle = ensureStream();
        SaveMobileDownloadCommand.Builder command = SaveMobileDownloadCommand.newBuilder()
                .setDownloadId(downloadId);
        if (options != null && CommandSupport.hasTimeout(options.timeoutMs())) {
            command.setRetryOptions(CommandSupport.commandRetryOptions(options.timeoutMs()));
        }
        handle.send(ContextSessionCommand.newBuilder().setSurfaceSessionId(surfaceSessionId)
                .setContextSessionId(sessionId).setSaveMobileDownload(command).build());
        while (true) {
            ContextSessionEvent event = handle.recv("save Android download");
            if (event.getEventCase() == ContextSessionEvent.EventCase.MOBILE_DOWNLOAD_SAVED
                    && downloadId.equals(event.getMobileDownloadSaved().getDownloadId())) {
                downloadToLocalFile(handle, event.getMobileDownloadSaved().getFileId(), Path.of(path));
                return;
            }
            if (event.getEventCase() == ContextSessionEvent.EventCase.ERROR) {
                throw new AllwrightException(event.getError().getMessage());
            }
        }
    }

    private String uploadLocalFile(
            RuntimeSupport.StreamHandle<ContextSessionCommand, ContextSessionEvent> handle, Path path
    ) {
        String transferId = UUID.randomUUID().toString();
        try (InputStream source = Files.newInputStream(path)) {
            long size = Files.size(path);
            long offset = 0;
            while (true) {
                byte[] data = source.readNBytes(256 * 1024);
                boolean last = offset + data.length >= size;
                handle.send(ContextSessionCommand.newBuilder().setSurfaceSessionId(surfaceSessionId)
                        .setContextSessionId(sessionId)
                        .setUploadFileChunk(UploadFileChunkCommand.newBuilder()
                                .setTransferId(transferId).setName(path.getFileName().toString())
                                .setOffset(offset).setData(ByteString.copyFrom(data)).setLast(last)).build());
                offset += data.length;
                if (last) break;
            }
        } catch (IOException error) {
            throw new AllwrightException("failed to read upload: " + error.getMessage());
        }
        while (true) {
            ContextSessionEvent event = handle.recv("upload Android chooser file");
            if (event.getEventCase() == ContextSessionEvent.EventCase.FILE_UPLOADED
                    && transferId.equals(event.getFileUploaded().getTransferId())) {
                return event.getFileUploaded().getFileId();
            }
            if (event.getEventCase() == ContextSessionEvent.EventCase.ERROR) {
                throw new AllwrightException(event.getError().getMessage());
            }
        }
    }

    private void downloadToLocalFile(
            RuntimeSupport.StreamHandle<ContextSessionCommand, ContextSessionEvent> handle,
            String fileId, Path path
    ) {
        Path temporary = path.resolveSibling(path.getFileName() + ".allwright-" + UUID.randomUUID() + ".tmp");
        try (OutputStream destination = Files.newOutputStream(temporary)) {
            long offset = 0;
            while (true) {
                handle.send(ContextSessionCommand.newBuilder().setSurfaceSessionId(surfaceSessionId)
                        .setContextSessionId(sessionId)
                        .setReadFileChunk(ReadFileChunkCommand.newBuilder().setFileId(fileId)
                                .setOffset(offset).setMaxBytes(256 * 1024)).build());
                ContextSessionEvent event = handle.recv("download Android file");
                if (event.getEventCase() == ContextSessionEvent.EventCase.ERROR) {
                    throw new AllwrightException(event.getError().getMessage());
                }
                if (event.getEventCase() != ContextSessionEvent.EventCase.FILE_CHUNK
                        || !fileId.equals(event.getFileChunk().getFileId())
                        || event.getFileChunk().getOffset() != offset) continue;
                byte[] data = event.getFileChunk().getData().toByteArray();
                destination.write(data);
                offset += data.length;
                if (event.getFileChunk().getLast()) break;
            }
        } catch (IOException error) {
            try { Files.deleteIfExists(temporary); } catch (IOException ignored) { }
            throw new AllwrightException("failed to save Android download: " + error.getMessage());
        }
        try {
            Files.move(temporary, path, StandardCopyOption.REPLACE_EXISTING);
        } catch (IOException error) {
            throw new AllwrightException("failed to finish Android download: " + error.getMessage());
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

    private TextResult readText(String selector, CommandOptions options, boolean textContent) {
        RuntimeSupport.StreamHandle<ContextSessionCommand, ContextSessionEvent> handle = ensureStream();
        ensureOpen();
        CommandOptions resolvedOptions = options == null ? new CommandOptions() : options;
        String transportSelector = AndroidSelectorSupport.normalizeSelectorForTransport(selector);
        ContextSessionCommand.Builder command = ContextSessionCommand.newBuilder()
                .setSurfaceSessionId(surfaceSessionId)
                .setContextSessionId(sessionId);
        if (textContent) {
            GetTextContentCommand.Builder getTextContent = GetTextContentCommand.newBuilder()
                    .setCssSelector(transportSelector);
            if (CommandSupport.hasTimeout(resolvedOptions.timeoutMs())) {
                getTextContent.setRetryOptions(CommandSupport.commandRetryOptions(resolvedOptions.timeoutMs()));
            }
            command.setGetTextContent(getTextContent);
        } else {
            GetInnerTextCommand.Builder getInnerText = GetInnerTextCommand.newBuilder()
                    .setCssSelector(transportSelector);
            if (CommandSupport.hasTimeout(resolvedOptions.timeoutMs())) {
                getInnerText.setRetryOptions(CommandSupport.commandRetryOptions(resolvedOptions.timeoutMs()));
            }
            command.setGetInnerText(getInnerText);
        }
        handle.send(command.build());

        while (true) {
            ContextSessionEvent event = handle.recv("receive tab session event while reading Android text");
            switch (event.getEventCase()) {
                case ATTACHED -> {
                }
                case TEXT_CONTENT_RESOLVED -> {
                    return new TextResult(
                            event.getTextContentResolved().getCssSelector(),
                            event.getTextContentResolved().getText(),
                            event.getTextContentResolved().getNote()
                    );
                }
                case INNER_TEXT_RESOLVED -> {
                    return new TextResult(
                            event.getInnerTextResolved().getCssSelector(),
                            event.getInnerTextResolved().getText(),
                            event.getInnerTextResolved().getNote()
                    );
                }
                case CLOSED -> {
                    closed = true;
                    throw new AllwrightException("android app session " + sessionId + " closed while reading text");
                }
                case ERROR -> throw new AllwrightException(
                        "android app session error while reading text: " + event.getError().getMessage()
                );
                default -> {
                }
            }
        }
    }
}
