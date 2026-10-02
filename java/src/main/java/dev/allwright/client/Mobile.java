package dev.allwright.client;

public final class Mobile {
    private final AndroidSurface android = new AndroidSurface();
    private final IosSurface ios = new IosSurface();

    Mobile() {}

    public AndroidSurface android() {
        return android;
    }

    public IosSurface ios() {
        return ios;
    }
}
