package dev.allwright.client;

import java.nio.file.Path;

public record ScreenshotOptions(Integer timeoutMs, boolean fullPage, Path path) {
    public ScreenshotOptions() {
        this(null, false, null);
    }

    public ScreenshotOptions(Integer timeoutMs) {
        this(timeoutMs, false, null);
    }
}
