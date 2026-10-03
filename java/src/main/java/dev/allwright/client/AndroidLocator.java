package dev.allwright.client;

public final class AndroidLocator {
    private final AndroidApp page;
    private final String selector;

    AndroidLocator(AndroidApp page, String selector) {
        this.page = page;
        this.selector = selector;
    }

    public AndroidApp app() {
        return page;
    }

    public String selector() {
        return selector;
    }

    public AndroidLocator locator(String childSelector) {
        return new AndroidLocator(page, AndroidSelectorSupport.chainSelectorForTransport(selector, childSelector));
    }

    public AndroidLocator getByRole(String role) { return getByRole(role, new RoleOptions()); }
    public AndroidLocator getByRole(String role, RoleOptions options) {
        return locator(WebSelectorSupport.role(role, options == null ? new RoleOptions() : options));
    }
    public AndroidLocator getByText(Object text) { return getByText(text, new TextOptions()); }
    public AndroidLocator getByText(Object text, TextOptions options) {
        return locator(WebSelectorSupport.text("text", text, options == null ? new TextOptions() : options));
    }
    public AndroidLocator getByLabel(Object text) { return getByLabel(text, new TextOptions()); }
    public AndroidLocator getByLabel(Object text, TextOptions options) {
        return locator(WebSelectorSupport.text("label", text, options == null ? new TextOptions() : options));
    }
    public AndroidLocator getByTestId(Object text) {
        return locator(WebSelectorSupport.selector(java.util.Map.of("kind", "testId", "text", text)));
    }

    public void click() {
        page.click(selector);
    }

    public void click(CommandOptions options) {
        page.click(selector, options);
    }

    public int count() {
        return page.count(selector);
    }

    public int count(CommandOptions options) {
        return page.count(selector, options);
    }

    public void focus() {
        page.focus(selector);
    }

    public void focus(CommandOptions options) {
        page.focus(selector, options);
    }

    public void fill(String value) {
        page.fill(selector, value);
    }

    public void fill(String value, CommandOptions options) {
        page.fill(selector, value, options);
    }

    public void press(String key) {
        page.press(selector, key);
    }

    public void press(String key, PressOptions options) {
        page.press(selector, key, options);
    }

    public String textContent() {
        return page.textContent(selector);
    }

    public String textContent(CommandOptions options) {
        return page.textContent(selector, options);
    }

    public String innerText() {
        return page.innerText(selector);
    }

    public String innerText(CommandOptions options) {
        return page.innerText(selector, options);
    }

    public void waitFor() {
        page.waitForSelector(selector);
    }

    public void waitFor(WaitForSelectorOptions options) {
        page.waitForSelector(selector, options);
    }
}
