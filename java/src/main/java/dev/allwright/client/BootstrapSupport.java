package dev.allwright.client;

import dev.allwright.engine.v1.EngineServiceGrpc;
import dev.allwright.engine.v1.PingRequest;
import io.grpc.ManagedChannel;
import io.grpc.ManagedChannelBuilder;
import io.grpc.StatusRuntimeException;
import java.io.IOException;
import java.net.URI;
import java.net.InetAddress;
import java.net.ServerSocket;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.StandardCopyOption;
import java.time.Duration;
import java.util.Objects;

final class BootstrapSupport {
    private static final String ALLWRIGHT_AUTO_INSTALL_ENV_VAR = "ALLWRIGHT_AUTO_INSTALL";
    private static final String ALLWRIGHT_CLI_PATH_ENV_VAR = "ALLWRIGHT_CLI_PATH";
    private static final String ALLWRIGHT_HOME_ENV_VAR = "ALLWRIGHT_HOME";
    private static final String ALLWRIGHT_REPOSITORY_ENV_VAR = "ALLWRIGHT_REPOSITORY";
    private static final String ALLWRIGHT_VERSION_ENV_VAR = "ALLWRIGHT_VERSION";
    private static final String DEFAULT_RELEASE_REPOSITORY = "allwright-dev/allwright";
    private static final String DEFAULT_RELEASE_VERSION = "0.0.43";
    private static final Duration PING_TIMEOUT = Duration.ofSeconds(1);
    private static final Duration STARTUP_TIMEOUT = Duration.ofSeconds(20);
    private static final HttpClient RELEASE_HTTP_CLIENT = HttpClient.newBuilder()
            .followRedirects(HttpClient.Redirect.NORMAL)
            .build();

    private static Process managedServer;
    private static String managedServerAddr;
    private static String managedServerBaseAddr;

    private BootstrapSupport() {}

    static synchronized String ensureRuntimeReady(String serverAddr) {
        String normalized = Objects.requireNonNull(serverAddr, "serverAddr").trim();
        String expectedVersion = expectedRuntimeVersion();
        PingStatus status = pingServer(normalized);
        if (status != null) {
            if (status.version().equals(expectedVersion)) {
                return normalized;
            }
            if (!isLocalServerAddr(normalized)) {
                throw new AllwrightException(
                        "allwright server at " + normalized + " is running version "
                                + displayVersion(status.version()) + " but this client expects " + expectedVersion
                );
            }
        }
        if (!isLocalServerAddr(normalized)) {
            throw new AllwrightException(
                    "allwright could not reach engine server at " + normalized
                            + ". Automatic startup is only supported for local addresses."
            );
        }

        if (managedServer != null && managedServer.isAlive() && normalized.equals(managedServerBaseAddr) && managedServerAddr != null) {
            return waitForServer(managedServerAddr, expectedVersion);
        }
        if (managedServer != null) {
            managedServer.destroyForcibly();
            managedServer = null;
            managedServerAddr = null;
            managedServerBaseAddr = null;
        }

        shutdownManagedServer();

        Path cliPath = ensureCliAvailable(expectedVersion);
        ensureWebPlugin(cliPath, expectedVersion);
        String resolvedServerAddr = normalized;
        if (status != null && !status.version().equals(expectedVersion)) {
            resolvedServerAddr = allocateManagedServerAddr(normalized);
        }

        try {
            managedServer = new ProcessBuilder(
                    cliPath.toString(),
                    "serve",
                    "--listen-addr",
                    cliListenAddr(resolvedServerAddr)
            )
                    .redirectInput(ProcessBuilder.Redirect.DISCARD)
                    .redirectOutput(ProcessBuilder.Redirect.DISCARD)
                    .redirectError(ProcessBuilder.Redirect.DISCARD)
                    .start();
        } catch (IOException exception) {
            throw new AllwrightException("start allwright server with " + cliPath + ": " + exception.getMessage(), exception);
        }
        managedServerBaseAddr = normalized;
        managedServerAddr = resolvedServerAddr;
        return waitForServer(resolvedServerAddr, expectedVersion);
    }

    static synchronized void shutdownManagedServer() {
        if (managedServer != null) {
            managedServer.destroyForcibly();
            managedServer = null;
        }
        managedServerAddr = null;
        managedServerBaseAddr = null;
    }

    private static String waitForServer(String serverAddr, String expectedVersion) {
        long deadline = System.nanoTime() + STARTUP_TIMEOUT.toNanos();
        while (System.nanoTime() < deadline) {
            PingStatus status = pingServer(serverAddr);
            if (status != null && status.version().equals(expectedVersion)) {
                return serverAddr;
            }
            try {
                Thread.sleep(250);
            } catch (InterruptedException exception) {
                Thread.currentThread().interrupt();
                throw new AllwrightException("interrupted while waiting for allwright server startup", exception);
            }
        }
        shutdownManagedServer();
        throw new AllwrightException(
                "timed out waiting for allwright server at " + serverAddr + " to become ready with version " + expectedVersion
        );
    }

    private static PingStatus pingServer(String serverAddr) {
        ManagedChannel channel = ManagedChannelBuilder.forTarget(serverAddr).usePlaintext().build();
        try {
            var response = EngineServiceGrpc.newBlockingStub(channel)
                    .withDeadlineAfter(PING_TIMEOUT.toMillis(), java.util.concurrent.TimeUnit.MILLISECONDS)
                    .ping(PingRequest.newBuilder().build());
            return new PingStatus(normalizeReleaseVersion(response.getVersion()));
        } catch (StatusRuntimeException ignored) {
            return null;
        } finally {
            channel.shutdownNow();
        }
    }

    private static Path ensureCliAvailable(String expectedVersion) {
        String explicit = System.getenv(ALLWRIGHT_CLI_PATH_ENV_VAR);
        if (explicit != null && !explicit.isBlank()) {
            Path path = Path.of(explicit.trim());
            if (Files.isRegularFile(path) && cliVersionMatches(path, expectedVersion)) {
                return path;
            }
        }

        Path bundled = allwrightHome().resolve("bin").resolve(cliFilename());
        if (Files.isRegularFile(bundled) && cliVersionMatches(bundled, expectedVersion)) {
            return bundled;
        }

        Path repoLocal = repoLocalCliPath();
        if (repoLocal != null && Files.isRegularFile(repoLocal) && cliVersionMatches(repoLocal, expectedVersion)) {
            return repoLocal;
        }

        Path fromPath = resolveFromPath(cliFilename());
        if (fromPath != null && cliVersionMatches(fromPath, expectedVersion)) {
            return fromPath;
        }

        if (!autoInstallEnabled()) {
            throw new AllwrightException("allwright CLI was not found. Install it first or set ALLWRIGHT_CLI_PATH.");
        }

        return installCli();
    }

    private static Path installCli() {
        Path installDir = allwrightHome().resolve("bin");
        try {
            Files.createDirectories(installDir);
        } catch (IOException exception) {
            throw new AllwrightException("create allwright CLI install dir " + installDir + ": " + exception.getMessage(), exception);
        }

        String versionTag = resolveReleaseTag();
        String assetName = cliAssetName(versionTag);
        Path archivePath = installDir.resolve(assetName);
        Path cliPath = installDir.resolve(cliFilename());

        HttpRequest request = HttpRequest.newBuilder()
                .uri(URI.create("https://github.com/" + releaseRepository() + "/releases/download/" + versionTag + "/" + assetName))
                .header("User-Agent", "allwright-java/" + DEFAULT_RELEASE_VERSION)
                .timeout(Duration.ofSeconds(120))
                .build();

        try {
            HttpResponse<Path> response = RELEASE_HTTP_CLIENT.send(request, HttpResponse.BodyHandlers.ofFile(archivePath));
            if (response.statusCode() < 200 || response.statusCode() >= 300) {
                Files.deleteIfExists(archivePath);
                throw new AllwrightException(
                        "failed to download allwright CLI asset " + assetName + ": HTTP " + response.statusCode()
                );
            }
            extractCliArchive(archivePath, installDir, cliPath);
            Files.deleteIfExists(archivePath);
            cliPath.toFile().setExecutable(true);
            return cliPath;
        } catch (InterruptedException exception) {
            Thread.currentThread().interrupt();
            throw new AllwrightException("install allwright CLI: " + exception.getMessage(), exception);
        } catch (IOException exception) {
            throw new AllwrightException("install allwright CLI: " + exception.getMessage(), exception);
        }
    }

    private static void ensureWebPlugin(Path cliPath, String expectedVersion) {
        Path pluginPath = allwrightHome().resolve("plugins").resolve("web").resolve("lib").resolve(webPluginFilename());
        if (Files.isRegularFile(pluginPath) && expectedVersion.equals(installedPluginVersion("web"))) {
            return;
        }
        try {
            Process process = new ProcessBuilder(
                    cliPath.toString(),
                    "plugin",
                    "install",
                    "web",
                    "--version",
                    expectedVersion
            )
                    .redirectInput(ProcessBuilder.Redirect.DISCARD)
                    .redirectOutput(ProcessBuilder.Redirect.DISCARD)
                    .redirectError(ProcessBuilder.Redirect.DISCARD)
                    .start();
            int exitCode = process.waitFor();
            if (exitCode != 0 || !Files.isRegularFile(pluginPath)) {
                throw new AllwrightException(
                        "allwright attempted to install the `web` plugin automatically, but the install did not complete successfully"
                );
            }
            if (!expectedVersion.equals(installedPluginVersion("web"))) {
                throw new AllwrightException(
                        "allwright attempted to install the `web` plugin automatically, but version "
                                + expectedVersion + " is still not active"
                );
            }
        } catch (InterruptedException exception) {
            Thread.currentThread().interrupt();
            throw new AllwrightException("install allwright web plugin: " + exception.getMessage(), exception);
        } catch (IOException exception) {
            throw new AllwrightException("install allwright web plugin: " + exception.getMessage(), exception);
        }
    }

    private static String resolveReleaseTag() {
        String version = System.getenv(ALLWRIGHT_VERSION_ENV_VAR);
        if (version == null || version.isBlank()) {
            version = DEFAULT_RELEASE_VERSION;
        }
        if (!version.trim().equals("latest")) {
            return normalizeReleaseTag(version);
        }
        try {
            HttpRequest request = HttpRequest.newBuilder()
                    .uri(URI.create("https://api.github.com/repos/" + releaseRepository() + "/releases/latest"))
                    .header("User-Agent", "allwright-java/" + DEFAULT_RELEASE_VERSION)
                    .timeout(Duration.ofSeconds(30))
                    .build();
            HttpResponse<String> response = RELEASE_HTTP_CLIENT.send(request, HttpResponse.BodyHandlers.ofString());
            String marker = "\"tag_name\":\"";
            int start = response.body().indexOf(marker);
            if (start < 0) {
                throw new AllwrightException("latest allwright release metadata did not include tag_name");
            }
            int end = response.body().indexOf('"', start + marker.length());
            return response.body().substring(start + marker.length(), end);
        } catch (InterruptedException exception) {
            Thread.currentThread().interrupt();
            throw new AllwrightException("resolve latest allwright release: " + exception.getMessage(), exception);
        } catch (IOException exception) {
            throw new AllwrightException("resolve latest allwright release: " + exception.getMessage(), exception);
        }
    }

    private static void extractCliArchive(Path archivePath, Path installDir, Path cliPath) throws IOException, InterruptedException {
        Path extractRoot = Files.createTempDirectory(installDir, "extract-");
        if (archivePath.toString().endsWith(".zip")) {
            Process process = new ProcessBuilder(
                    "powershell",
                    "-NoProfile",
                    "-Command",
                    "Expand-Archive -Path '" + archivePath + "' -DestinationPath '" + extractRoot + "' -Force"
            )
                    .redirectInput(ProcessBuilder.Redirect.DISCARD)
                    .redirectOutput(ProcessBuilder.Redirect.DISCARD)
                    .redirectError(ProcessBuilder.Redirect.PIPE)
                    .start();
            if (process.waitFor() != 0) {
                String errorOutput = new String(process.getErrorStream().readAllBytes(), StandardCharsets.UTF_8).trim();
                throw new IOException("failed to extract allwright CLI zip archive" + (errorOutput.isEmpty() ? "" : ": " + errorOutput));
            }
        } else {
            Process process = new ProcessBuilder(
                    "tar",
                    "-xzf",
                    archivePath.toString(),
                    "-C",
                    extractRoot.toString()
            )
                    .redirectInput(ProcessBuilder.Redirect.DISCARD)
                    .redirectOutput(ProcessBuilder.Redirect.DISCARD)
                    .redirectError(ProcessBuilder.Redirect.PIPE)
                    .start();
            if (process.waitFor() != 0) {
                String errorOutput = new String(process.getErrorStream().readAllBytes(), StandardCharsets.UTF_8).trim();
                throw new IOException("failed to extract allwright CLI tar archive" + (errorOutput.isEmpty() ? "" : ": " + errorOutput));
            }
        }

        try {
            Path extracted = findExtractedCli(extractRoot);
            if (extracted == null) {
                throw new IOException("allwright CLI archive did not contain bin/" + cliFilename());
            }
            Files.copy(extracted, cliPath, StandardCopyOption.REPLACE_EXISTING);
        } finally {
            deleteRecursively(extractRoot);
        }
    }

    private static String cliAssetName(String versionTag) {
        String os = System.getProperty("os.name").toLowerCase();
        String arch = System.getProperty("os.arch").toLowerCase();
        String target;
        if (os.contains("mac") && arch.contains("aarch64")) {
            target = "aarch64-apple-darwin";
        } else if (os.contains("mac") && (arch.contains("x86_64") || arch.contains("amd64"))) {
            target = "x86_64-apple-darwin";
        } else if (os.contains("linux") && arch.contains("aarch64")) {
            target = "aarch64-unknown-linux-gnu";
        } else if (os.contains("linux") && (arch.contains("x86_64") || arch.contains("amd64"))) {
            target = "x86_64-unknown-linux-gnu";
        } else if (os.contains("windows") && arch.contains("aarch64")) {
            target = "aarch64-pc-windows-msvc";
        } else if (os.contains("windows") && (arch.contains("x86_64") || arch.contains("amd64"))) {
            target = "x86_64-pc-windows-msvc";
        } else {
            throw new AllwrightException("automatic allwright CLI install is not supported on " + os + "/" + arch);
        }
        String extension = os.contains("windows") ? "zip" : "tar.gz";
        return "allwright-" + versionTag + "-" + target + "." + extension;
    }

    private static boolean autoInstallEnabled() {
        String raw = System.getenv(ALLWRIGHT_AUTO_INSTALL_ENV_VAR);
        if (raw == null) {
            return true;
        }
        return switch (raw.trim().toLowerCase()) {
            case "0", "false", "no" -> false;
            default -> true;
        };
    }

    private static Path allwrightHome() {
        String configured = System.getenv(ALLWRIGHT_HOME_ENV_VAR);
        if (configured != null && !configured.isBlank()) {
            return Path.of(configured.trim());
        }
        return Path.of(System.getProperty("user.home"), ".allwright");
    }

    private static Path resolveFromPath(String filename) {
        String path = System.getenv("PATH");
        if (path == null || path.isBlank()) {
            return null;
        }
        for (String entry : path.split(java.io.File.pathSeparator)) {
            if (entry.isBlank()) {
                continue;
            }
            Path candidate = Path.of(entry).resolve(filename);
            if (Files.isRegularFile(candidate)) {
                return candidate;
            }
        }
        return null;
    }

    private static String releaseRepository() {
        String configured = System.getenv(ALLWRIGHT_REPOSITORY_ENV_VAR);
        return configured == null || configured.isBlank() ? DEFAULT_RELEASE_REPOSITORY : configured.trim();
    }

    private static String normalizeReleaseTag(String version) {
        String trimmed = version.trim();
        return trimmed.startsWith("v") ? trimmed : "v" + trimmed;
    }

    private static String normalizeReleaseVersion(String version) {
        return version.trim().replaceFirst("^v", "");
    }

    private static String expectedRuntimeVersion() {
        String configured = System.getenv(ALLWRIGHT_VERSION_ENV_VAR);
        if (configured == null || configured.isBlank()) {
            configured = DEFAULT_RELEASE_VERSION;
        }
        return normalizeReleaseVersion(configured);
    }

    private static String cliListenAddr(String serverAddr) {
        return serverAddr.replaceFirst("^https?://", "");
    }

    private static boolean isLocalServerAddr(String serverAddr) {
        String listenAddr = cliListenAddr(serverAddr);
        int separator = listenAddr.lastIndexOf(':');
        String host = separator > 0 ? listenAddr.substring(0, separator) : listenAddr;
        host = host.replace("[", "").replace("]", "");
        return host.equals("127.0.0.1") || host.equals("localhost") || host.equals("::1");
    }

    private static String installedPluginVersion(String pluginId) {
        Path manifestPath = allwrightHome().resolve("plugins.txt");
        if (!Files.isRegularFile(manifestPath)) {
            return null;
        }
        try {
            for (String line : Files.readAllLines(manifestPath)) {
                String trimmed = line.trim();
                if (trimmed.isEmpty() || trimmed.startsWith("#")) {
                    continue;
                }
                String[] parts = trimmed.split("\t", 3);
                if (parts.length < 3 || !parts[0].equals(pluginId)) {
                    continue;
                }
                return normalizeReleaseVersion(parts[2]);
            }
        } catch (IOException exception) {
            throw new AllwrightException("read allwright plugin manifest " + manifestPath + ": " + exception.getMessage(), exception);
        }
        return null;
    }

    private static String allocateManagedServerAddr(String serverAddr) {
        String host = localBindingHost(serverAddr);
        try (ServerSocket socket = new ServerSocket(0, 0, InetAddress.getByName(host))) {
            int port = socket.getLocalPort();
            if (host.contains(":")) {
                return "[" + host + "]:" + port;
            }
            return host + ":" + port;
        } catch (IOException exception) {
            throw new AllwrightException("reserve local port for managed allwright server on " + host + ": " + exception.getMessage(), exception);
        }
    }

    private static String localBindingHost(String serverAddr) {
        String listenAddr = cliListenAddr(serverAddr);
        int separator = listenAddr.lastIndexOf(':');
        String host = separator > 0 ? listenAddr.substring(0, separator) : listenAddr;
        host = host.replace("[", "").replace("]", "");
        return host.equals("::1") ? "::1" : "127.0.0.1";
    }

    private static String displayVersion(String version) {
        return version == null || version.isBlank() ? "unknown" : version;
    }

    private static Path repoLocalCliPath() {
        Path repoRoot = Path.of("").toAbsolutePath().normalize();
        for (Path candidate : new Path[] {
                repoRoot.resolve("target").resolve("debug").resolve(cliFilename()),
                repoRoot.resolve("target").resolve("release").resolve(cliFilename())
        }) {
            if (Files.isRegularFile(candidate)) {
                return candidate;
            }
        }
        return null;
    }

    private static boolean cliVersionMatches(Path cliPath, String expectedVersion) {
        try {
            Process process = new ProcessBuilder(cliPath.toString(), "--version")
                    .redirectInput(ProcessBuilder.Redirect.DISCARD)
                    .redirectError(ProcessBuilder.Redirect.DISCARD)
                    .start();
            String output = new String(process.getInputStream().readAllBytes());
            if (process.waitFor() != 0) {
                return false;
            }
            for (String token : output.split("\\s+")) {
                if (!token.isEmpty() && Character.isDigit(token.charAt(0))) {
                    return normalizeReleaseVersion(token).equals(expectedVersion);
                }
            }
            return false;
        } catch (InterruptedException exception) {
            Thread.currentThread().interrupt();
            throw new AllwrightException("inspect allwright CLI version via " + cliPath + ": " + exception.getMessage(), exception);
        } catch (IOException exception) {
            throw new AllwrightException("inspect allwright CLI version via " + cliPath + ": " + exception.getMessage(), exception);
        }
    }

    private static String cliFilename() {
        return System.getProperty("os.name").toLowerCase().contains("windows") ? "allwright.exe" : "allwright";
    }

    private static String webPluginFilename() {
        String os = System.getProperty("os.name").toLowerCase();
        if (os.contains("windows")) {
            return "allwright_surface_web.dll";
        }
        if (os.contains("mac")) {
            return "liballwright_surface_web.dylib";
        }
        return "liballwright_surface_web.so";
    }

    private static Path findExtractedCli(Path extractRoot) throws IOException {
        try (var paths = Files.walk(extractRoot)) {
            return paths
                    .filter(Files::isRegularFile)
                    .filter(path -> path.getFileName().toString().equals(cliFilename()))
                    .filter(path -> {
                        Path relative = extractRoot.relativize(path);
                        return relative.getNameCount() >= 2 && relative.getName(relative.getNameCount() - 2).toString().equals("bin");
                    })
                    .findFirst()
                    .orElse(null);
        }
    }

    private static void deleteRecursively(Path root) throws IOException {
        if (!Files.exists(root)) {
            return;
        }
        try (var paths = Files.walk(root)) {
            paths.sorted((left, right) -> right.getNameCount() - left.getNameCount())
                    .forEach(path -> {
                        try {
                            Files.deleteIfExists(path);
                        } catch (IOException ignored) {
                        }
                    });
        }
    }

    private record PingStatus(String version) {}
}
