package dev.allwright.client;

public record MobileIosLaunchOptions(
        String appPath,
        String appId,
        boolean stopBeforeLaunch,
        Integer timeoutMs
) {
    public MobileIosLaunchOptions() {
        this(null, null, false, null);
    }

    public MobileIosLaunchOptions(String appId, boolean stopBeforeLaunch, Integer timeoutMs) {
        this(null, appId, stopBeforeLaunch, timeoutMs);
    }
}
