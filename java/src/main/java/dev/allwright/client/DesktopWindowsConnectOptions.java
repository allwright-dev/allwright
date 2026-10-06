package dev.allwright.client;

public record DesktopWindowsConnectOptions(String agentEndpoint, Integer timeoutMs) {
    public DesktopWindowsConnectOptions() { this(null, null); }
}
