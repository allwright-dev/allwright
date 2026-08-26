package dev.allwright.client;

public record WaitForSelectorOptions(Integer timeoutMs, Boolean visible) {
    public WaitForSelectorOptions() {
        this(null, null);
    }
}
