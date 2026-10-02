package dev.allwright.client;

public record MobileIosConnectOptions(
        String device,
        String agentEndpoint,
        boolean preserveAppState,
        Integer timeoutMs
) {
    public MobileIosConnectOptions() {
        this(null, null, false, null);
    }
}
