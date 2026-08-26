package dev.allwright.client;

public record LaunchOptions(String browserBinary, Integer timeoutMs) {
    public LaunchOptions() {
        this(null, null);
    }
}
