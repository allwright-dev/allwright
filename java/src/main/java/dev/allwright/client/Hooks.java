package dev.allwright.client;

public final class Hooks {
    public static final HookType<Page> NEW_PAGE = new HookType<>("new_page", (browser, event) -> {
        if (!event.hasNewPage() || event.getNewPage().getContextSessionId().isBlank()) {
            throw new AllwrightException("new page hook completed with an invalid result");
        }
        return browser.pageFromHook(event.getNewPage().getContextSessionId());
    });

    private Hooks() {}
}
