package dev.allwright.client;

public final class Locator implements WebLocators {
    private final Page page;
    private final String selector;

    Locator(Page page, String selector) {
        this.page = page;
        this.selector = selector;
    }

    public Page frame() { return frame(new CommandOptions()); }
    public Page frame(CommandOptions options) { return page.frame(selector, options); }

    public Page page() {
        return page;
    }

    public String selector() {
        return selector;
    }

    public Locator locator(String childSelector) {
        return new Locator(page, SelectorSupport.chainSelectorForTransport(selector, childSelector));
    }

    public Locator not(Locator other) {
        if (other == null || other.page != page) throw new IllegalArgumentException("Excluded locators must belong to the same page");
        return locator(WebSelectorSupport.selector(java.util.Map.of("kind", "exclude", "selector", other.selector)));
    }

    public Locator filter(LocatorFilterOptions options) {
        java.util.Map<String, Object> spec = new java.util.LinkedHashMap<>();
        spec.put("kind", "filter");
        for (Locator inner : new Locator[]{options.has, options.hasNot}) {
            if (inner != null && inner.page != page) throw new IllegalArgumentException("Filter locators must belong to the same page");
        }
        if (options.has != null) spec.put("has", options.has.selector);
        if (options.hasNot != null) spec.put("hasNot", options.hasNot.selector);
        spec.put("hasText", options.hasText); spec.put("hasNotText", options.hasNotText); spec.put("visible", options.visible);
        return locator(WebSelectorSupport.selector(spec));
    }
    public Locator nth(int index) { return locator(WebSelectorSupport.selector(java.util.Map.of("kind", "nth", "index", index))); }
    public Locator first() { return nth(0); }
    public Locator last() { return nth(-1); }

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

    public HighlightResult highlight() {
        return page.highlight(selector);
    }

    public HighlightResult highlight(HighlightOptions options) {
        return page.highlight(selector, options);
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

    public ElementResult hover() {
        return page.hover(selector);
    }

    public ElementResult hover(CommandOptions options) {
        return page.hover(selector, options);
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
