package dev.allwright.client;
/** Shared web locator builders for pages and nested locators. Text accepts String or Pattern. */
public interface WebLocators {
    Locator locator(String selector);
    default Locator getByRole(String role) { return getByRole(role, new RoleOptions()); }
    default Locator getByRole(String role, RoleOptions options) { return locator(WebSelectorSupport.role(role, options)); }
    default Locator getByText(Object text) { return getByText(text, new TextOptions()); }
    default Locator getByText(Object text, TextOptions options) { return locator(WebSelectorSupport.text("text", text, options)); }
    default Locator getByLabel(Object text) { return getByLabel(text, new TextOptions()); }
    default Locator getByLabel(Object text, TextOptions options) { return locator(WebSelectorSupport.text("label", text, options)); }
    default Locator getByPlaceholder(Object text) { return getByPlaceholder(text, new TextOptions()); }
    default Locator getByPlaceholder(Object text, TextOptions options) { return locator(WebSelectorSupport.text("placeholder", text, options)); }
    default Locator getByAltText(Object text) { return getByAltText(text, new TextOptions()); }
    default Locator getByAltText(Object text, TextOptions options) { return locator(WebSelectorSupport.text("altText", text, options)); }
    default Locator getByTitle(Object text) { return getByTitle(text, new TextOptions()); }
    default Locator getByTitle(Object text, TextOptions options) { return locator(WebSelectorSupport.text("title", text, options)); }
    default Locator getByTestId(Object text) { return getByTestId(text, new TextOptions()); }
    default Locator getByTestId(Object text, TextOptions options) { return locator(WebSelectorSupport.text("testId", text, options)); }
}
