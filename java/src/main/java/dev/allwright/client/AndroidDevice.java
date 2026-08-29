package dev.allwright.client;

import java.util.LinkedHashMap;
import java.util.Map;

public final class AndroidDevice {
    private final Map<String, Object> connectInfo;
    private AndroidPage page;

    AndroidDevice(Map<String, Object> connectInfo) {
        this.connectInfo = connectInfo;
        this.page = new AndroidPage(
                ConfigSupport.mapValue(connectInfo.get("browser_session")),
                ConfigSupport.mapValue(ConfigSupport.mapValue(connectInfo.get("initial_page")).get("page_session"))
        );
    }

    public String sessionId() {
        Map<String, Object> browserSession = ConfigSupport.mapValue(connectInfo.get("browser_session"));
        Map<String, Object> automation = ConfigSupport.mapValue(browserSession == null ? null : browserSession.get("automation"));
        return automation == null ? "" : String.valueOf(automation.getOrDefault("session_id", ""));
    }

    public AndroidPage page() {
        return page;
    }

    public AndroidPage initialPage() {
        return page();
    }

    public AndroidPage launch() {
        return launch(new MobileAndroidLaunchOptions());
    }

    public AndroidPage launch(MobileAndroidLaunchOptions options) {
        MobileAndroidLaunchOptions resolved = options == null ? new MobileAndroidLaunchOptions() : options;
        Map<String, Object> payload = new LinkedHashMap<>();
        payload.put("command", "launch_app");
        payload.put("browser_session", connectInfo.get("browser_session"));

        Map<String, Object> launchOptions = new LinkedHashMap<>();
        launchOptions.put("apk_path", resolved.apkPath());
        launchOptions.put("app_id", resolved.appId());
        launchOptions.put("launch_activity", resolved.launchActivity());
        launchOptions.put("stop_before_launch", resolved.stopBeforeLaunch());
        launchOptions.put("timeout_ms", resolved.timeoutMs());
        payload.put("options", launchOptions);

        String response = BootstrapSupport.invokePlugin("mobile-android", MobileJsonSupport.toJson(payload));
        Map<String, Object> result = AndroidSurface.mobileResult("launch", response);
        page = new AndroidPage(
                ConfigSupport.mapValue(connectInfo.get("browser_session")),
                ConfigSupport.mapValue(result.get("page_session"))
        );
        return page;
    }
}
