package dev.allwright.client;

import java.nio.file.Path;

public record ResolveConfigOptions(Path cwd, Path configFile, String suite) {
    public ResolveConfigOptions() {
        this(null, null, null);
    }
}
