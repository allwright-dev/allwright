package dev.allwright.client;

import java.nio.file.Path;
import java.util.Map;

public record ResolvedConfig(
        Path configFilePath,
        String suiteName,
        String serverAddr,
        String browserName,
        String browserBinary,
        LaunchOptions launchOptions,
        RetryConfig expect,
        Map<String, Object> web,
        Map<String, Object> mobile,
        Map<String, Object> desktop
) {}
