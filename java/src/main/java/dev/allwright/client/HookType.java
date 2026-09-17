package dev.allwright.client;

import dev.allwright.engine.v1.HookCompletedEvent;
import java.util.function.BiFunction;

public final class HookType<T> {
    private final String name;
    private final BiFunction<HookContext, HookCompletedEvent, T> decoder;

    HookType(String name, BiFunction<HookContext, HookCompletedEvent, T> decoder) {
        this.name = name;
        this.decoder = decoder;
    }

    String name() {
        return name;
    }

    T decode(HookContext context, HookCompletedEvent event) {
        return decoder.apply(context, event);
    }
}
