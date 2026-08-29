package dev.allwright.client;

import java.util.LinkedHashMap;
import java.util.Map;
import org.yaml.snakeyaml.Yaml;

public final class AndroidSurface {
    AndroidSurface() {}

    public AndroidDevice connect() {
        return connect(new MobileAndroidConnectOptions());
    }

    public AndroidDevice connect(MobileAndroidConnectOptions options) {
        BootstrapSupport.ensurePluginsInstalled("mobile-android");
        MobileAndroidConnectOptions resolved = options == null ? new MobileAndroidConnectOptions() : options;
        Map<String, Object> request = new LinkedHashMap<>();
        request.put("command", "connect");
        request.put("platform", "android");
        request.put("device", resolved.device());
        request.put("adb_endpoint", resolved.adbEndpoint());
        request.put("preserve_app_state", resolved.preserveAppState());
        request.put("timeout_ms", resolved.timeoutMs());
        String response = BootstrapSupport.invokePlugin("mobile-android", MobileJsonSupport.toJson(request));
        return new AndroidDevice(mobileResult("connect", response));
    }

    static Map<String, Object> mobileResult(String commandName, String response) {
        Object parsed = new Yaml().load(response);
        Map<String, Object> envelope = ConfigSupport.mapValue(parsed);
        if (envelope == null) {
            throw new AllwrightException("mobile-android plugin " + commandName + " returned no envelope");
        }
        Object ok = envelope.get("ok");
        if (!(ok instanceof Boolean success) || !success) {
            throw new AllwrightException(
                    String.valueOf(envelope.getOrDefault("error", "mobile-android plugin " + commandName + " failed"))
            );
        }
        Map<String, Object> result = ConfigSupport.mapValue(envelope.get("result"));
        if (result == null) {
            throw new AllwrightException("mobile-android plugin " + commandName + " returned no result");
        }
        return result;
    }
}
