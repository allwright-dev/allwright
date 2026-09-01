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
