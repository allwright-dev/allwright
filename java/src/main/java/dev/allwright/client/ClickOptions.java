package dev.allwright.client;

public record ClickOptions(Integer timeoutMs, MouseButton button, Integer clickCount) {
    public ClickOptions() {
        this(null, MouseButton.LEFT, null);
    }

    public ClickOptions(Integer timeoutMs) {
        this(timeoutMs, MouseButton.LEFT, null);
    }
}
