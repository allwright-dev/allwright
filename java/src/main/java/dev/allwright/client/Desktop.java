package dev.allwright.client;

public final class Desktop {
    private final MacSurface mac = new MacSurface();
    private final WindowsSurface windows = new WindowsSurface();

    Desktop() {}

    public MacSurface mac() {
        return mac;
    }

    public WindowsSurface windows() { return windows; }
}
