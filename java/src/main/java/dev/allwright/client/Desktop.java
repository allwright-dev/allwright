package dev.allwright.client;

public final class Desktop {
    private final MacSurface mac = new MacSurface();

    Desktop() {}

    public MacSurface mac() {
        return mac;
    }
}
