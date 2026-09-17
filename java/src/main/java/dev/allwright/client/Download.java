package dev.allwright.client;

import java.nio.file.Path;

public final class Download {
    private final Page page;
    private final String id;
    private final String url;
    private final String suggestedFilename;

    Download(Page page, String id, String url, String suggestedFilename) {
        this.page = page;
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
        page.saveDownload(id, path.toString(), options);
    }
}
