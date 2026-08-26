package dev.allwright.client;

public record CommandOptions(Integer timeoutMs) {
    public CommandOptions() {
        this(null);
    }
}
