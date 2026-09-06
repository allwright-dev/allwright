package dev.allwright.client;

/** Selects JSON (default) or standard YAML encoding of the same snapshot tree. */
public record AccessibilitySnapshotOptions(String format, Integer timeoutMs, String mode) {
    public AccessibilitySnapshotOptions() { this("json", null); }
    public AccessibilitySnapshotOptions(String format) { this(format, null); }
    public AccessibilitySnapshotOptions(String format, Integer timeoutMs) { this(format, timeoutMs, "default"); }
    public AccessibilitySnapshotOptions {
        if (mode == null) mode = "default";
        if (!java.util.Set.of("default", "ai", "autoexpect", "codegen").contains(mode)) {
            throw new IllegalArgumentException("invalid accessibility snapshot mode");
        }
        if (format == null) format = "json";
        if (!format.equals("json") && !format.equals("yaml")) {
            throw new IllegalArgumentException("accessibility snapshot format must be 'json' or 'yaml'");
        }
    }
}
