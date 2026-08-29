package dev.allwright.client;

public final class Mobile {
    private final AndroidSurface android = new AndroidSurface();

    Mobile() {}

    public AndroidSurface android() {
        return android;
    }
}
