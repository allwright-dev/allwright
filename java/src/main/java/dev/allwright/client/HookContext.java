package dev.allwright.client;

interface HookContext {
    <T> T waitForHook(String hookId, HookType<T> type, CommandOptions options);

    void setHookFileChooserFiles(String chooserId, java.util.List<String> files, CommandOptions options);

    void saveHookDownload(String downloadId, String path, CommandOptions options);
}
