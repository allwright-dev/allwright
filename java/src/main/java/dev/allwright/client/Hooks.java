package dev.allwright.client;

public final class Hooks {
    public static final HookType<Page> NEW_PAGE = new HookType<>("new_page", (page, event) -> {
        if (!event.hasNewPage() || event.getNewPage().getContextSessionId().isBlank()) {
            throw new AllwrightException("new page hook completed with an invalid result");
        }
        return page.pageFromHook(event.getNewPage().getContextSessionId());
    });
    public static final HookType<FileChooser> FILE_CHOOSER = new HookType<>("file_chooser", (page, event) -> {
        if (!event.hasFileChooser() || event.getFileChooser().getFileChooserId().isBlank()) {
            throw new AllwrightException("file chooser hook completed with an invalid result");
        }
        return new FileChooser(
                page,
                event.getFileChooser().getFileChooserId(),
                event.getFileChooser().getIsMultiple()
        );
    });
    public static final HookType<Download> DOWNLOAD = new HookType<>("download", (page, event) -> {
        if (!event.hasDownload() || event.getDownload().getDownloadId().isBlank()) {
            throw new AllwrightException("download hook completed with an invalid result");
        }
        return new Download(
                page,
                event.getDownload().getDownloadId(),
                event.getDownload().getUrl(),
                event.getDownload().getSuggestedFilename()
        );
    });

    private Hooks() {}
}
