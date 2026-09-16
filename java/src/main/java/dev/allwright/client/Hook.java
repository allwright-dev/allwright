package dev.allwright.client;

public final class Hook<T> {
    private final Browser browser;
    private final String id;
    private final HookType<T> type;

    Hook(Browser browser, String id, HookType<T> type) {
        this.browser = browser;
        this.id = id;
        this.type = type;
    }

    public String id() {
        return id;
    }

    public T waitFor() {
        return waitFor(new CommandOptions());
    }

    public T waitFor(CommandOptions options) {
        return browser.waitForHook(id, type, options);
    }
}
