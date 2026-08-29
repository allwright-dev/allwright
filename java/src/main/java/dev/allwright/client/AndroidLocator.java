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

    public FillResult fill(String value) {
        return page.fill(selector, value);
    }

    public FillResult fill(String value, CommandOptions options) {
        return page.fill(selector, value, options);
    }
}
