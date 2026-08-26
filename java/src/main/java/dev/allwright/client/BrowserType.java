package dev.allwright.client;

import dev.allwright.engine.v1.BrowserKind;

public final class BrowserType {
    private final BrowserKind browserKind;

    BrowserType(BrowserKind browserKind) {
        this.browserKind = browserKind;
    }

    public Browser launch() {
        return launch(new LaunchOptions());
    }

    public Browser launch(LaunchOptions options) {
        return Allwright.launchBrowser(browserKind, options);
    }
}
