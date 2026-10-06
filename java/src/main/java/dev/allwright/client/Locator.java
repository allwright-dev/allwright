package dev.allwright.client;

public final class Locator implements WebLocators {
    private final Page page;
    private final String selector;

    Locator(Page page, String selector) {
        this.page = page;
        this.selector = selector;
    }

    public String inputValue() { return inputValue(new CommandOptions()); }
    public String inputValue(CommandOptions options) { return page.inputValue(selector, options); }
    public java.util.List<CapturedOption> selectedOptions() { return selectedOptions(new CommandOptions()); }
    public java.util.List<CapturedOption> selectedOptions(CommandOptions options) { return page.selectedOptions(selector, options); }
    public String selectedText() { return selectedText(new CommandOptions()); }
    public String selectedText(CommandOptions options) { return page.selectedText(selector, options); }
    public boolean isChecked() { return isChecked(new CommandOptions()); }
    public boolean isChecked(CommandOptions options) { return page.isChecked(selector, options); }
    public String getAttribute(String name) { return getAttribute(name, new CommandOptions()); }
    public String getAttribute(String name, CommandOptions options) { return page.getAttribute(selector, name, options); }
    public BoundingBox boundingBox() { return boundingBox(new CommandOptions()); }
    public BoundingBox boundingBox(CommandOptions options) { return page.boundingBox(selector, options); }

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

    public void click() {
        page.click(selector);
    }

    public void click(CommandOptions options) {
        page.click(selector, options);
    }

    public void click(ClickOptions options) {
        page.click(selector, options);
    }

    public void dblclick() {
        page.dblclick(selector);
    }

    public void dblclick(ClickOptions options) {
        page.dblclick(selector, options);
    }

    public int count() {
        return page.count(selector);
    }

    public int count(CommandOptions options) {
        return page.count(selector, options);
    }

    public void highlight() {
        page.highlight(selector);
    }

    public void highlight(HighlightOptions options) {
        page.highlight(selector, options);
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

    public void hover() {
        page.hover(selector);
    }

    public void hover(CommandOptions options) {
        page.hover(selector, options);
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
