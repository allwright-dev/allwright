package dev.allwright.client;

public record DesktopWindowsLaunchOptions(String appId, boolean terminateRunning, Integer timeoutMs) {
    public DesktopWindowsLaunchOptions() { this(null, false, null); }
}
