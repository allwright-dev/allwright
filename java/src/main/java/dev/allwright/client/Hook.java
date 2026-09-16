package dev.allwright.client;

public final class Hook<T> {
    private final Page page;
    private final String id;
    private final HookType<T> type;

    Hook(Page page, String id, HookType<T> type) {
        this.page = page;
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
        return page.waitForHook(id, type, options);
    }
}
