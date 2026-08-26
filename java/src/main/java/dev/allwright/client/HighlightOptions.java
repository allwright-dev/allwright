package dev.allwright.client;

public record HighlightOptions(Integer timeoutMs, Integer durationMs) {
    public HighlightOptions() {
        this(null, null);
    }
}
