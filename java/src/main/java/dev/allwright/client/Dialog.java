package dev.allwright.client;

/** A pending JavaScript alert, confirm, or prompt, returned by a dialog hook. */
public record Dialog(Page page, String id, String type, String message, String defaultValue) {
    public void accept() { accept(null, new CommandOptions()); }
    public void accept(String promptText) { accept(promptText, new CommandOptions()); }
    public void accept(CommandOptions options) { accept(null, options); }
    public void accept(String promptText, CommandOptions options) { page.handleDialog(id, true, promptText, options); }
    public void dismiss() { dismiss(new CommandOptions()); }
    public void dismiss(CommandOptions options) { page.handleDialog(id, false, null, options); }
}
