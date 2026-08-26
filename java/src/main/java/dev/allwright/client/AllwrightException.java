package dev.allwright.client;

public final class AllwrightException extends RuntimeException {
    public AllwrightException(String message) {
        super(message);
    }

    public AllwrightException(String message, Throwable cause) {
        super(message, cause);
    }
}
