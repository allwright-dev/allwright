package dev.allwright.client;

public record MobileAndroidLaunchOptions(
        String apkPath,
        String appId,
        String launchActivity,
        boolean stopBeforeLaunch,
        Integer timeoutMs
) {
    public MobileAndroidLaunchOptions() {
        this(null, null, null, false, null);
    }
}
