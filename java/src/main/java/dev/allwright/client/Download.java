package dev.allwright.client;

import java.nio.file.Path;

public final class Download {
    private final Page page;
    private final AndroidApp app;
    private final HookContext context;
    private final String id;
    private final String url;
    private final String suggestedFilename;

    Download(Page page, String id, String url, String suggestedFilename) {
        this.page = page;
        this.app = null;
        this.context = page;
        this.id = id;
        this.url = url;
        this.suggestedFilename = suggestedFilename;
    }

    Download(AndroidApp app, String id, String url, String suggestedFilename) {
        this.page = null;
        this.app = app;
        this.context = app;
        this.id = id;
        this.url = url;
        this.suggestedFilename = suggestedFilename;
    }

    public String id() {
        return id;
    }

    public Page page() {
        return page;
    }

    public AndroidApp app() {
        return app;
    }

    public String url() {
        return url;
    }

    public String suggestedFilename() {
        return suggestedFilename;
    }

    public void saveAs(Path path) {
        saveAs(path, new CommandOptions());
    }

    public void saveAs(Path path, CommandOptions options) {
        context.saveHookDownload(id, path.toString(), options);
    }
}
