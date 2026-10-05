package dev.allwright.client;

import dev.allwright.engine.v1.LaunchAppCommand;
import dev.allwright.engine.v1.SurfaceSessionCommand;
import dev.allwright.engine.v1.SurfaceSessionEvent;

public final class IosDevice {
    private final RuntimeSupport.RuntimeClient runtime;
    private final RuntimeSupport.StreamHandle<SurfaceSessionCommand, SurfaceSessionEvent> stream;
    private final String sessionId;
    private final String surfaceSessionId;
    private NativeApp app;
    private boolean closed;

    IosDevice(RuntimeSupport.RuntimeClient runtime,
              RuntimeSupport.StreamHandle<SurfaceSessionCommand, SurfaceSessionEvent> stream,
              String sessionId, String surfaceSessionId, String initialAppSessionId) {
        this.runtime = runtime;
        this.stream = stream;
        this.sessionId = sessionId;
        this.surfaceSessionId = surfaceSessionId;
        this.app = new NativeApp(runtime, surfaceSessionId, initialAppSessionId);
    }

    public String sessionId() { return sessionId; }
    public NativeApp app() { return app; }
    public NativeApp initialApp() { return app; }
    public synchronized NativeApp launch() { return launch(new MobileIosLaunchOptions()); }

    public synchronized NativeApp launch(MobileIosLaunchOptions options) {
        if (closed) throw new AllwrightException("ios device session " + sessionId + " is closed");
        MobileIosLaunchOptions resolved = options == null ? new MobileIosLaunchOptions() : options;
        LaunchAppCommand.Builder launch = LaunchAppCommand.newBuilder()
                .setStopBeforeLaunch(resolved.stopBeforeLaunch());
        if (resolved.appPath() != null && !resolved.appPath().isBlank()) launch.setApkPath(resolved.appPath());
        if (resolved.appId() != null && !resolved.appId().isBlank()) launch.setAppId(resolved.appId());
        if (CommandSupport.hasTimeout(resolved.timeoutMs())) {
            launch.setRetryOptions(CommandSupport.commandRetryOptions(resolved.timeoutMs()));
        }
        stream.send(SurfaceSessionCommand.newBuilder().setLaunchApp(launch).build());
        while (true) {
            SurfaceSessionEvent event = stream.recv("receive event while launching iOS app");
            switch (event.getEventCase()) {
                case APP_LAUNCHED -> {
                    app = new NativeApp(runtime, surfaceSessionId, event.getAppLaunched().getAppSessionId());
                    return app;
                }
                case CLOSED -> {
                    closed = true;
                    throw new AllwrightException("ios device session " + sessionId + " closed while launching app");
                }
                case ERROR -> throw new AllwrightException(
                        "iOS device session error: " + event.getError().getMessage());
                default -> { }
            }
        }
    }
}
