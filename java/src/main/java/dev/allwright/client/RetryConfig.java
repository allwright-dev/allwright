package dev.allwright.client;

public record RetryConfig(Integer timeoutMs, Integer intervalMs) {
    public RetryConfig() {
        this(null, null);
    }
}
