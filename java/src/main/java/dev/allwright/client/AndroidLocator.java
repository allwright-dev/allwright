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

    public ClickResult click() {
        return page.click(selector);
    }

    public ClickResult click(CommandOptions options) {
        return page.click(selector, options);
    }

    public CountResult count() {
        return page.count(selector);
    }

    public CountResult count(CommandOptions options) {
        return page.count(selector, options);
    }

    public ElementResult focus() {
        return page.focus(selector);
    }

    public ElementResult focus(CommandOptions options) {
        return page.focus(selector, options);
    }

    public FillResult fill(String value) {
        return page.fill(selector, value);
    }

    public FillResult fill(String value, CommandOptions options) {
        return page.fill(selector, value, options);
    }

    public PressResult press(String key) {
        return page.press(selector, key);
    }

    public PressResult press(String key, PressOptions options) {
        return page.press(selector, key, options);
    }

    public TextResult textContent() {
        return page.textContent(selector);
    }

    public TextResult textContent(CommandOptions options) {
        return page.textContent(selector, options);
    }

    public TextResult innerText() {
        return page.innerText(selector);
    }

    public TextResult innerText(CommandOptions options) {
        return page.innerText(selector, options);
    }

    public WaitForSelectorResult waitFor() {
        return page.waitForSelector(selector);
    }

    public WaitForSelectorResult waitFor(WaitForSelectorOptions options) {
        return page.waitForSelector(selector, options);
    }
}
