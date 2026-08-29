package dev.allwright.client;

import java.util.LinkedHashMap;
import java.util.Map;

public final class AndroidPage {
    private final Map<String, Object> browserSession;
    private final Map<String, Object> pageSession;

    AndroidPage(Map<String, Object> browserSession, Map<String, Object> pageSession) {
        this.browserSession = browserSession == null ? Map.of() : browserSession;
        this.pageSession = pageSession == null ? Map.of() : pageSession;
    }

    public String sessionId() {
        return String.valueOf(pageSession.getOrDefault("page_id", ""));
    }

    public AndroidLocator locator(String selector) {
        return new AndroidLocator(this, AndroidSelectorSupport.normalizeSelectorForTransport(selector));
    }

    public ClickResult click(String selector) {
        return click(selector, new CommandOptions());
    }

    public ClickResult click(String selector, CommandOptions options) {
        Map<String, Object> payload = new LinkedHashMap<>();
        payload.put("command", "click_element");
        payload.put("browser_session", browserSession);
        payload.put("page_session", pageSession);
        payload.put("selector", AndroidSelectorSupport.normalizeSelectorForTransport(selector));
        payload.put("timeout_ms", options == null ? null : options.timeoutMs());
        String response = BootstrapSupport.invokePlugin("mobile-android", MobileJsonSupport.toJson(payload));
        Map<String, Object> result = AndroidSurface.mobileResult("click", response);
        return new ClickResult(
                String.valueOf(result.getOrDefault("selector", "")),
                String.valueOf(result.getOrDefault("note", "")),
                String.valueOf(result.getOrDefault("session_id", ""))
        );
    }

    public FillResult fill(String selector, String value) {
        return fill(selector, value, new CommandOptions());
    }

    public FillResult fill(String selector, String value, CommandOptions options) {
        Map<String, Object> payload = new LinkedHashMap<>();
        payload.put("command", "fill_element");
        payload.put("browser_session", browserSession);
        payload.put("page_session", pageSession);
        payload.put("selector", AndroidSelectorSupport.normalizeSelectorForTransport(selector));
        payload.put("value", value);
        payload.put("timeout_ms", options == null ? null : options.timeoutMs());
        String response = BootstrapSupport.invokePlugin("mobile-android", MobileJsonSupport.toJson(payload));
        Map<String, Object> result = AndroidSurface.mobileResult("fill", response);
        return new FillResult(
                String.valueOf(result.getOrDefault("selector", "")),
                String.valueOf(result.getOrDefault("value", "")),
                String.valueOf(result.getOrDefault("note", ""))
        );
    }
}
