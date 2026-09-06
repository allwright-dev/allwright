package dev.allwright.client;
public final class LocatorFilterOptions {
    Locator has;
    public LocatorFilterOptions setHas(Locator value) { this.has = value; return this; }
    Locator hasNot;
    public LocatorFilterOptions setHasNot(Locator value) { this.hasNot = value; return this; }
    Object hasText;
    public LocatorFilterOptions setHasText(Object value) { this.hasText = value; return this; }
    Object hasNotText;
    public LocatorFilterOptions setHasNotText(Object value) { this.hasNotText = value; return this; }
    Boolean visible;
    public LocatorFilterOptions setVisible(Boolean value) { this.visible = value; return this; }
}
