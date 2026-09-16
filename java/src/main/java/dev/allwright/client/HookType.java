package dev.allwright.client;

import dev.allwright.engine.v1.HookCompletedEvent;
import java.util.function.BiFunction;

public final class HookType<T> {
    private final String name;
    private final BiFunction<Browser, HookCompletedEvent, T> decoder;

    HookType(String name, BiFunction<Browser, HookCompletedEvent, T> decoder) {
        this.name = name;
        this.decoder = decoder;
    }

    String name() {
        return name;
    }

    T decode(Browser browser, HookCompletedEvent event) {
        return decoder.apply(browser, event);
    }
}
