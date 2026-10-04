package dev.allwright.client;

import dev.allwright.engine.v1.DesktopAppLaunchedEvent;
import dev.allwright.engine.v1.LaunchDesktopAppCommand;
import dev.allwright.engine.v1.SurfaceSessionCommand;
import dev.allwright.engine.v1.SurfaceSessionEvent;

public final class MacDesktop {
    private final RuntimeSupport.RuntimeClient runtime;
    private final RuntimeSupport.StreamHandle<SurfaceSessionCommand, SurfaceSessionEvent> stream;
    private final String sessionId;
    private final String surfaceSessionId;
    private MacApp app;
    private boolean closed;

    MacDesktop(RuntimeSupport.RuntimeClient runtime,
               RuntimeSupport.StreamHandle<SurfaceSessionCommand, SurfaceSessionEvent> stream,
               String sessionId, String surfaceSessionId, String initialAppSessionId) {
        this.runtime = runtime;
        this.stream = stream;
        this.sessionId = sessionId;
        this.surfaceSessionId = surfaceSessionId;
        this.app = new MacApp(runtime, surfaceSessionId, initialAppSessionId);
    }

    public String sessionId() { return sessionId; }
    public MacApp app() { return app; }
    public MacApp initialApp() { return app; }
    public synchronized MacApp launch() { return launch(new DesktopMacLaunchOptions()); }

    public synchronized MacApp launch(DesktopMacLaunchOptions options) {
        if (closed) throw new AllwrightException("mac desktop session " + sessionId + " is closed");
        DesktopMacLaunchOptions resolved = options == null ? new DesktopMacLaunchOptions() : options;
        if (resolved.appId() == null || resolved.appId().isBlank()) {
            throw new AllwrightException("desktop mac launch requires appId");
        }
        LaunchDesktopAppCommand.Builder launch = LaunchDesktopAppCommand.newBuilder()
                .setAppId(resolved.appId())
                .setTerminateRunning(resolved.terminateRunning());
        if (CommandSupport.hasTimeout(resolved.timeoutMs())) {
            launch.setRetryOptions(CommandSupport.commandRetryOptions(resolved.timeoutMs()));
        }
        stream.send(SurfaceSessionCommand.newBuilder().setLaunchDesktopApp(launch).build());

        while (true) {
            SurfaceSessionEvent event = stream.recv("receive event while launching mac app");
            switch (event.getEventCase()) {
                case DESKTOP_APP_LAUNCHED -> {
                    DesktopAppLaunchedEvent launched = event.getDesktopAppLaunched();
                    app = new MacApp(runtime, surfaceSessionId, launched.getAppSessionId());
                    return app;
                }
                case CLOSED -> {
                    closed = true;
                    throw new AllwrightException("mac desktop session " + sessionId + " closed while launching app");
                }
                case ERROR -> throw new AllwrightException(
                        "mac desktop launch error: " + event.getError().getMessage());
                default -> { }
            }
        }
    }
}
