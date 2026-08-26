package dev.allwright.client;

import java.nio.file.Path;

public record ResolvedConfig(
        Path configFilePath,
        String suiteName,
        String serverAddr,
        String browserName,
        String browserBinary,
        LaunchOptions launchOptions,
        RetryConfig expect
) {}
