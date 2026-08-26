package dev.allwright.client;

import java.nio.file.Path;
import java.util.LinkedHashMap;
import java.util.Map;

final class ConfigSupport {
    private ConfigSupport() {}

    static void validateConfigShape(Map<?, ?> root, Path source) {
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
    static Map<String, Object> mapValue(Object value) {
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
    static Map<String, Map<String, Object>> suiteMapValue(Object value) {
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

    static RetryConfig retryConfigValue(Object value) {
        Map<String, Object> map = mapValue(value);
        if (map == null) {
            return null;
        }
        return new RetryConfig(integerValue(map.get("timeoutMs")), integerValue(map.get("intervalMs")));
    }

    static LaunchOptions launchOptionsValue(Map<String, Object> browser) {
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

    static LaunchOptions mergeLaunchOptions(LaunchOptions base, LaunchOptions override) {
        if (override == null) {
            return base == null ? new LaunchOptions() : base;
        }
        LaunchOptions resolvedBase = base == null ? new LaunchOptions() : base;
        return new LaunchOptions(
                firstNonBlank(override.browserBinary(), resolvedBase.browserBinary()),
                override.timeoutMs() != null ? override.timeoutMs() : resolvedBase.timeoutMs()
        );
    }

    static RetryConfig mergeRetryConfig(RetryConfig base, RetryConfig override) {
        RetryConfig resolvedBase = base == null ? new RetryConfig() : base;
        if (override == null) {
            return resolvedBase;
        }
        return new RetryConfig(
                override.timeoutMs() != null ? override.timeoutMs() : resolvedBase.timeoutMs(),
                override.intervalMs() != null ? override.intervalMs() : resolvedBase.intervalMs()
        );
    }

    static String browserNameValue(Map<String, Object> browser) {
        return trimToNull(stringValue(browser == null ? null : browser.get("name")));
    }

    static String browserBinaryValue(Map<String, Object> browser) {
        return trimToNull(stringValue(browser == null ? null : browser.get("binary")));
    }

    static String serverAddrValue(Map<String, Object> server) {
        return trimToNull(stringValue(server == null ? null : server.get("addr")));
    }

    static Integer integerValue(Object value) {
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

    static String stringValue(Object value) {
        return value == null ? null : String.valueOf(value);
    }

    static String trimToNull(String value) {
        if (value == null) {
            return null;
        }
        String trimmed = value.trim();
        return trimmed.isEmpty() ? null : trimmed;
    }

    static String firstNonBlank(String... values) {
        for (String value : values) {
            if (value != null && !value.isBlank()) {
                return value.trim();
            }
        }
        return null;
    }
}
