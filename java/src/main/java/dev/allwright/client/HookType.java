package dev.allwright.client;

import dev.allwright.engine.v1.HookCompletedEvent;
import java.util.function.BiFunction;

public final class HookType<T> {
    private final String name;
    private final BiFunction<Page, HookCompletedEvent, T> decoder;

    HookType(String name, BiFunction<Page, HookCompletedEvent, T> decoder) {
        this.name = name;
        this.decoder = decoder;
    }

    String name() {
        return name;
    }

    T decode(Page page, HookCompletedEvent event) {
        return decoder.apply(page, event);
    }
}
