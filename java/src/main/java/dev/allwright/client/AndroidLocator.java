package dev.allwright.client;

public final class AndroidLocator extends NativeLocator {
    private final AndroidApp androidApp;

    AndroidLocator(AndroidApp app, String selector) {
        super(app, selector);
        this.androidApp = app;
    }

    @Override
    public AndroidApp app() {
        return androidApp;
    }

    @Override
    public AndroidLocator locator(String childSelector) {
        return new AndroidLocator(
                androidApp,
                NativeSelectorSupport.chainSelectorForTransport(selector(), childSelector)
        );
    }

    @Override
    public AndroidLocator getByRole(String role) { return getByRole(role, new RoleOptions()); }

    @Override
    public AndroidLocator getByRole(String role, RoleOptions options) {
        return locator(WebSelectorSupport.role(role, options == null ? new RoleOptions() : options));
    }

    @Override
    public AndroidLocator getByText(Object text) { return getByText(text, new TextOptions()); }

    @Override
    public AndroidLocator getByText(Object text, TextOptions options) {
        return locator(WebSelectorSupport.text("text", text, options == null ? new TextOptions() : options));
    }

    @Override
    public AndroidLocator getByLabel(Object text) { return getByLabel(text, new TextOptions()); }

    @Override
    public AndroidLocator getByLabel(Object text, TextOptions options) {
        return locator(WebSelectorSupport.text("label", text, options == null ? new TextOptions() : options));
    }

    @Override
    public AndroidLocator getByTestId(Object text) {
        return locator(WebSelectorSupport.selector(java.util.Map.of("kind", "testId", "text", text)));
    }
}
