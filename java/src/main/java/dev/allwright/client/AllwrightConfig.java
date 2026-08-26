package dev.allwright.client;

import java.util.Map;

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
