package dev.allwright.client;

/** Selects JSON (default) or standard YAML encoding of the same snapshot tree. */
public record AccessibilitySnapshotOptions(String format, Integer timeoutMs) {
    public AccessibilitySnapshotOptions() { this("json", null); }
    public AccessibilitySnapshotOptions(String format) { this(format, null); }
    public AccessibilitySnapshotOptions {
        if (format == null) format = "json";
        if (!format.equals("json") && !format.equals("yaml")) {
            throw new IllegalArgumentException("accessibility snapshot format must be 'json' or 'yaml'");
        }
    }
}
