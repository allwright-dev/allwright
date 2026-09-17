package dev.allwright.client;

public final class Hook<T> {
    private final HookContext context;
    private final String id;
    private final HookType<T> type;

    Hook(HookContext context, String id, HookType<T> type) {
        this.context = context;
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
        return context.waitForHook(id, type, options);
    }
}
