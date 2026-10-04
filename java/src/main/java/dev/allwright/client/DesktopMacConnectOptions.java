package dev.allwright.client;

public record DesktopMacConnectOptions(String agentEndpoint, Integer timeoutMs) {
    public DesktopMacConnectOptions() {
        this(null, null);
    }
}
