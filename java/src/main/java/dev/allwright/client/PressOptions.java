package dev.allwright.client;

public record PressOptions(Integer timeoutMs, String text) {
    public PressOptions() {
        this(null, null);
    }
}
