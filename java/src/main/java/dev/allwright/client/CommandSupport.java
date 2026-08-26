package dev.allwright.client;

import dev.allwright.engine.v1.CommandRetryOptions;

final class CommandSupport {
    private CommandSupport() {}

    static CommandRetryOptions commandRetryOptions(Integer timeoutMs) {
        return CommandRetryOptions.newBuilder().setTimeoutMs(timeoutMs).build();
    }

    static boolean hasTimeout(Integer timeoutMs) {
        return timeoutMs != null && timeoutMs > 0;
    }
}
