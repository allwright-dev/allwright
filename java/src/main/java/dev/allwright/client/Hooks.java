package dev.allwright.client;

public final class Hooks {
    public static final HookType<Dialog> DIALOG = new HookType<>("dialog", (page, event) -> {
        if (!event.hasDialog() || event.getDialog().getDialogId().isBlank()) {
            throw new AllwrightException("dialog hook completed with an invalid result");
        }
        var d = event.getDialog();
        return new Dialog((Page) page, d.getDialogId(), d.getType(), d.getMessage(), d.getDefaultValue());
    });
    public static final HookType<Page> NEW_PAGE = new HookType<>("new_page", (page, event) -> {
        if (!event.hasNewPage() || event.getNewPage().getContextSessionId().isBlank()) {
            throw new AllwrightException("new page hook completed with an invalid result");
        }
        return ((Page) page).pageFromHook(event.getNewPage().getContextSessionId());
    });
    public static final HookType<FileChooser> FILE_CHOOSER = new HookType<>("file_chooser", (page, event) -> {
        if (event.hasFileChooser() && !event.getFileChooser().getFileChooserId().isBlank()) {
            return new FileChooser((Page) page, event.getFileChooser().getFileChooserId(),
                    event.getFileChooser().getIsMultiple());
        }
        if (event.hasMobileFileChooser() && !event.getMobileFileChooser().getFileChooserId().isBlank()) {
            return new FileChooser((AndroidApp) page, event.getMobileFileChooser().getFileChooserId(),
                    event.getMobileFileChooser().getIsMultiple());
        }
        throw new AllwrightException("file chooser hook completed with an invalid result");
    });
    public static final HookType<Download> DOWNLOAD = new HookType<>("download", (page, event) -> {
        if (event.hasDownload() && !event.getDownload().getDownloadId().isBlank()) {
            return new Download((Page) page, event.getDownload().getDownloadId(),
                    event.getDownload().getUrl(), event.getDownload().getSuggestedFilename());
        }
        if (event.hasMobileDownload() && !event.getMobileDownload().getDownloadId().isBlank()) {
            return new Download((AndroidApp) page, event.getMobileDownload().getDownloadId(), "",
                    event.getMobileDownload().getSuggestedFilename());
        }
        throw new AllwrightException("download hook completed with an invalid result");
    });

    private Hooks() {}
}
