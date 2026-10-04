package dev.allwright.client;

public record DesktopMacLaunchOptions(String appId, boolean terminateRunning, Integer timeoutMs) {
    public DesktopMacLaunchOptions() {
        this(null, false, null);
    }
}
