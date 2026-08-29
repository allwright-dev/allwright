package dev.allwright.client;

import dev.allwright.engine.v1.BrowserLaunchedEvent;
import dev.allwright.engine.v1.BrowserKind;
import dev.allwright.engine.v1.BrowserSessionCommand;
import dev.allwright.engine.v1.BrowserSessionEvent;
import dev.allwright.engine.v1.EngineServiceGrpc;
import dev.allwright.engine.v1.LaunchBrowserCommand;
import dev.allwright.engine.v1.PingRequest;
import io.grpc.ManagedChannel;
import io.grpc.ManagedChannelBuilder;
import io.grpc.StatusRuntimeException;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;
import java.util.Map;
import java.util.Objects;
import org.yaml.snakeyaml.Yaml;

public final class Allwright {
    public static final String DEFAULT_SERVER_ADDR = "127.0.0.1:50051";
    public static final String SERVER_ADDR_ENV_VAR = "ALLWRIGHT_SERVER_ADDR";
    private static final List<String> CONFIG_FILENAMES = List.of(
            "allwright.config.yaml",
            "allwright.config.yml",
            "allwright.config.json",
            ".allwright/config.yaml",
            ".allwright/config.yml",
            ".allwright/config.json"
    );

    private static final Object RUNTIME_LOCK = new Object();
    private static RuntimeSupport.RuntimeClient runtimeClient;
    private static String serverAddrOverride;
    private static final BrowserType CHROMIUM = new BrowserType(BrowserKind.BROWSER_KIND_CHROMIUM);
    private static final BrowserType FIREFOX = new BrowserType(BrowserKind.BROWSER_KIND_FIREFOX);
    private static final Mobile MOBILE = new Mobile();

    private Allwright() {}

    public static BrowserType chromium() {
        return CHROMIUM;
    }

    public static BrowserType firefox() {
        return FIREFOX;
    }

    public static Mobile mobile() {
        return MOBILE;
    }

    public static Browser launchChrome() {
        return launchChrome(new LaunchOptions());
    }

    public static Browser launchChrome(LaunchOptions options) {
        return launchBrowser(BrowserKind.BROWSER_KIND_CHROMIUM, options);
    }

    public static Browser launchFirefox() {
        return launchFirefox(new LaunchOptions());
    }

    public static Browser launchFirefox(LaunchOptions options) {
        return launchBrowser(BrowserKind.BROWSER_KIND_FIREFOX, options);
    }

    public static Browser launchBrowser() {
        return launchBrowser(new ResolveConfigOptions());
    }

    public static Browser launchBrowser(ResolveConfigOptions options) {
        return launchConfiguredBrowser(resolveConfig(options));
    }

    public static Browser launchBrowser(BrowserKind browserKind, LaunchOptions options) {
        RuntimeSupport.RuntimeClient runtime = getRuntime();
        RuntimeSupport.StreamHandle<BrowserSessionCommand, BrowserSessionEvent> stream =
                new RuntimeSupport.StreamHandle<>(runtime.asyncStub()::browserSession);

        LaunchBrowserCommand.Builder launch = LaunchBrowserCommand.newBuilder().setBrowserKind(browserKind);
        if (options.browserBinary() != null && !options.browserBinary().isBlank()) {
            launch.setBrowserBinary(options.browserBinary());
        }
        if (CommandSupport.hasTimeout(options.timeoutMs())) {
            launch.setRetryOptions(CommandSupport.commandRetryOptions(options.timeoutMs()));
        }

        stream.send(BrowserSessionCommand.newBuilder().setLaunchBrowser(launch).build());

        while (true) {
            BrowserSessionEvent event = stream.recv("receive browser session event during launch");
            switch (event.getEventCase()) {
                case BROWSER_LAUNCHED -> {
                    BrowserLaunchedEvent launched = event.getBrowserLaunched();
                    Page initialPage = new Page(runtime, event.getSessionId(), launched.getInitialTabSessionId());
                    return new Browser(
                            runtime,
                            stream,
                            event.getSessionId(),
                            launched.getBrowser(),
                            launched.getNote(),
                            "",
                            launched.getUserDataDir(),
                            initialPage
                    );
                }
                case ERROR -> throw new AllwrightException(
                        "browser session error during launch: " + event.getError().getMessage()
                );
                default -> {
                }
            }
        }
    }

    public static String ping() {
        try {
            return getRuntime().blockingStub().ping(PingRequest.newBuilder().build()).getMessage();
        } catch (StatusRuntimeException exception) {
            throw new AllwrightException("ping engine server: " + exception.getStatus(), exception);
        }
    }

    public static void setServerAddr(String serverAddr) {
        synchronized (RUNTIME_LOCK) {
            serverAddrOverride = Objects.requireNonNull(serverAddr, "serverAddr").trim();
            shutdownLocked();
        }
    }

    public static void shutdown() {
        synchronized (RUNTIME_LOCK) {
            shutdownLocked();
        }
    }

    public static Path findConfigFile() {
        return findConfigFile(Path.of("").toAbsolutePath());
    }

    public static Path findConfigFile(Path startDir) {
        Path currentDir = Objects.requireNonNull(startDir, "startDir").toAbsolutePath().normalize();
        while (true) {
            for (String filename : CONFIG_FILENAMES) {
                Path candidate = currentDir.resolve(filename);
                if (Files.isRegularFile(candidate)) {
                    return candidate;
                }
            }
            Path parent = currentDir.getParent();
            if (parent == null) {
                return null;
            }
            currentDir = parent;
        }
    }

    public static AllwrightConfig loadConfigFile(Path configFile) {
        Path resolved = Objects.requireNonNull(configFile, "configFile").toAbsolutePath().normalize();
        try {
            Object loaded = new Yaml().load(Files.readString(resolved));
            if (!(loaded instanceof Map<?, ?> root)) {
                throw new AllwrightException("allwright config " + resolved + " must contain a top-level object");
            }
            ConfigSupport.validateConfigShape(root, resolved);
            return new AllwrightConfig(
                    ConfigSupport.integerValue(root.get("schemaVersion")),
                    ConfigSupport.mapValue(root.get("server")),
                    ConfigSupport.mapValue(root.get("web")),
                    ConfigSupport.mapValue(root.get("mobile")),
                    ConfigSupport.mapValue(root.get("desktop")),
                    ConfigSupport.retryConfigValue(root.get("expect")),
                    ConfigSupport.suiteMapValue(root.get("suites"))
            );
        } catch (IOException exception) {
            throw new AllwrightException("read allwright config " + resolved + ": " + exception.getMessage(), exception);
        }
    }

    public static ResolvedConfig resolveConfig() {
        return resolveConfig(new ResolveConfigOptions());
    }

    public static ResolvedConfig resolveConfig(ResolveConfigOptions options) {
        ResolveConfigOptions resolvedOptions = options == null ? new ResolveConfigOptions() : options;
        Path configFilePath = resolvedOptions.configFile() != null
                ? resolvedOptions.configFile().toAbsolutePath().normalize()
                : findConfigFile(resolvedOptions.cwd() == null ? Path.of("").toAbsolutePath() : resolvedOptions.cwd());
        AllwrightConfig fileConfig = configFilePath != null ? loadConfigFile(configFilePath) : new AllwrightConfig();
        String suiteName = ConfigSupport.trimToNull(resolvedOptions.suite());
        Map<String, Object> suiteConfig = null;

        if (suiteName != null) {
            suiteConfig = fileConfig.suites() == null ? null : fileConfig.suites().get(suiteName);
            if (suiteConfig == null) {
                throw new AllwrightException(
                        "allwright config suite \"" + suiteName + "\" was not found in "
                                + (configFilePath == null ? "the resolved config file" : configFilePath)
                );
            }
        }

        Map<String, Object> resolvedWeb = ConfigSupport.mergeSurfaceMap(
                fileConfig.web(),
                ConfigSupport.mapValue(suiteConfig == null ? null : suiteConfig.get("web"))
        );
        Map<String, Object> resolvedMobile = ConfigSupport.mergeSurfaceMap(
                fileConfig.mobile(),
                ConfigSupport.mapValue(suiteConfig == null ? null : suiteConfig.get("mobile"))
        );
        Map<String, Object> resolvedDesktop = ConfigSupport.mergeSurfaceMap(
                fileConfig.desktop(),
                ConfigSupport.mapValue(suiteConfig == null ? null : suiteConfig.get("desktop"))
        );

        String browserName = ConfigSupport.firstNonBlank(
                ConfigSupport.browserNameValue(ConfigSupport.browserMapValue(ConfigSupport.mapValue(suiteConfig == null ? null : suiteConfig.get("web")))),
                ConfigSupport.browserNameValue(ConfigSupport.browserMapValue(fileConfig.web()))
        );
        String browserBinary = ConfigSupport.firstNonBlank(
                ConfigSupport.browserBinaryValue(ConfigSupport.browserMapValue(ConfigSupport.mapValue(suiteConfig == null ? null : suiteConfig.get("web")))),
                ConfigSupport.browserBinaryValue(ConfigSupport.browserMapValue(fileConfig.web()))
        );
        String serverAddr = ConfigSupport.firstNonBlank(
                ConfigSupport.serverAddrValue(ConfigSupport.mapValue(suiteConfig == null ? null : suiteConfig.get("server"))),
                ConfigSupport.serverAddrValue(fileConfig.server())
        );
        LaunchOptions launchOptions = ConfigSupport.mergeLaunchOptions(
                ConfigSupport.launchOptionsValue(ConfigSupport.browserMapValue(fileConfig.web())),
                ConfigSupport.launchOptionsValue(ConfigSupport.browserMapValue(ConfigSupport.mapValue(suiteConfig == null ? null : suiteConfig.get("web"))))
        );
        if (browserBinary != null) {
            launchOptions = new LaunchOptions(browserBinary, launchOptions.timeoutMs());
        }
        if (browserName == null && resolvedMobile == null && resolvedDesktop == null) {
            browserName = "chromium";
        }
        RetryConfig expect = ConfigSupport.mergeRetryConfig(
                fileConfig.expect(),
                ConfigSupport.retryConfigValue(suiteConfig == null ? null : suiteConfig.get("expect"))
        );

        return new ResolvedConfig(
                configFilePath,
                suiteName,
                serverAddr,
                browserName,
                browserBinary,
                launchOptions,
                expect,
                resolvedWeb,
                resolvedMobile,
                resolvedDesktop
        );
    }

    public static Browser launchConfiguredBrowser(ResolvedConfig config) {
        Objects.requireNonNull(config, "config");
        if (config.serverAddr() != null && !config.serverAddr().isBlank()) {
            setServerAddr(config.serverAddr());
        }
        if ((config.browserName() == null || config.browserName().isBlank())
                && (config.mobile() != null || config.desktop() != null)) {
            throw new AllwrightException(
                    "resolved config does not define web.browser.name and includes only non-web surfaces"
            );
        }
        return switch (config.browserName()) {
            case "firefox" -> launchFirefox(config.launchOptions());
            case "chromium", "" -> launchChrome(config.launchOptions());
            default -> throw new AllwrightException(
                    "unsupported browser.name \"" + config.browserName() + "\"; use \"chromium\" or \"firefox\""
            );
        };
    }

    private static RuntimeSupport.RuntimeClient getRuntime() {
        synchronized (RUNTIME_LOCK) {
            if (runtimeClient == null) {
                String serverAddr = resolveServerAddr();
                String resolvedServerAddr = BootstrapSupport.ensureRuntimeReady(serverAddr);
                ManagedChannel channel = ManagedChannelBuilder.forTarget(resolvedServerAddr)
                        .usePlaintext()
                        .build();
                runtimeClient = new RuntimeSupport.RuntimeClient(
                        channel,
                        EngineServiceGrpc.newBlockingStub(channel),
                        EngineServiceGrpc.newStub(channel)
                );
            }
            return runtimeClient;
        }
    }

    private static String resolveServerAddr() {
        if (serverAddrOverride != null && !serverAddrOverride.isBlank()) {
            return serverAddrOverride;
        }
        String fromEnv = System.getenv(SERVER_ADDR_ENV_VAR);
        return (fromEnv == null || fromEnv.isBlank()) ? DEFAULT_SERVER_ADDR : fromEnv.trim();
    }

    private static void shutdownLocked() {
        if (runtimeClient != null) {
            runtimeClient.channel().shutdownNow();
            runtimeClient = null;
        }
        BootstrapSupport.shutdownManagedServer();
    }
}
