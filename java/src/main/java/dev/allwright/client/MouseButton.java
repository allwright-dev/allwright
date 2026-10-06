package dev.allwright.client;

public enum MouseButton {
    LEFT("left"),
    MIDDLE("middle"),
    RIGHT("right");

    private final String wireName;

    MouseButton(String wireName) {
        this.wireName = wireName;
    }

    String wireName() {
        return wireName;
    }
}
