package dev.allwright.client;

import java.nio.file.Path;
import java.util.List;

public final class FileChooser {
    private final Page page;
    private final String id;
    private final boolean multiple;

    FileChooser(Page page, String id, boolean multiple) {
        this.page = page;
        this.id = id;
        this.multiple = multiple;
    }

    public String id() {
        return id;
    }

    public Page page() {
        return page;
    }

    public boolean isMultiple() {
        return multiple;
    }

    public void setFiles(Path file) {
        setFiles(List.of(file), new CommandOptions());
    }

    public void setFiles(List<Path> files) {
        setFiles(files, new CommandOptions());
    }

    public void setFiles(List<Path> files, CommandOptions options) {
        if (!multiple && files.size() > 1) {
            throw new AllwrightException("file chooser does not accept multiple files");
        }
        page.setFileChooserFiles(id, files.stream().map(Path::toString).toList(), options);
    }
}
