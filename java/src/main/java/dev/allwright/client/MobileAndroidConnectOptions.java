package dev.allwright.client;

public record MobileAndroidConnectOptions(
        String device,
        String adbEndpoint,
        boolean preserveAppState,
        Integer timeoutMs
) {
    public MobileAndroidConnectOptions() {
        this(null, null, false, null);
    }
}
