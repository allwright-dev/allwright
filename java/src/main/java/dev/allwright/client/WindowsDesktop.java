package dev.allwright.client;

import dev.allwright.engine.v1.LaunchDesktopAppCommand;
import dev.allwright.engine.v1.SurfaceSessionCommand;
import dev.allwright.engine.v1.SurfaceSessionEvent;

public final class WindowsDesktop {
    private final RuntimeSupport.RuntimeClient runtime;
    private final RuntimeSupport.StreamHandle<SurfaceSessionCommand, SurfaceSessionEvent> stream;
    private final String sessionId;
    private final String surfaceSessionId;
    private WindowsApp app;
    private boolean closed;

    WindowsDesktop(RuntimeSupport.RuntimeClient runtime, RuntimeSupport.StreamHandle<SurfaceSessionCommand, SurfaceSessionEvent> stream,
                   String sessionId, String surfaceSessionId, String initialAppSessionId) {
        this.runtime = runtime; this.stream = stream; this.sessionId = sessionId; this.surfaceSessionId = surfaceSessionId;
        this.app = new WindowsApp(runtime, surfaceSessionId, initialAppSessionId);
    }
    public String sessionId() { return sessionId; }
    public WindowsApp app() { return app; }
    public WindowsApp initialApp() { return app; }
    public synchronized WindowsApp launch() { return launch(new DesktopWindowsLaunchOptions()); }
    public synchronized WindowsApp launch(DesktopWindowsLaunchOptions options) {
        if (closed) throw new AllwrightException("Windows desktop session " + sessionId + " is closed");
        DesktopWindowsLaunchOptions resolved = options == null ? new DesktopWindowsLaunchOptions() : options;
        if (resolved.appId() == null || resolved.appId().isBlank()) throw new AllwrightException("desktop Windows launch requires appId");
        LaunchDesktopAppCommand.Builder launch = LaunchDesktopAppCommand.newBuilder().setAppId(resolved.appId()).setTerminateRunning(resolved.terminateRunning());
        if (CommandSupport.hasTimeout(resolved.timeoutMs())) launch.setRetryOptions(CommandSupport.commandRetryOptions(resolved.timeoutMs()));
        stream.send(SurfaceSessionCommand.newBuilder().setLaunchDesktopApp(launch).build());
        while (true) {
            SurfaceSessionEvent event = stream.recv("receive event while launching Windows app");
            switch (event.getEventCase()) {
                case DESKTOP_APP_LAUNCHED -> { app = new WindowsApp(runtime, surfaceSessionId, event.getDesktopAppLaunched().getAppSessionId()); return app; }
                case CLOSED -> { closed = true; throw new AllwrightException("Windows desktop session " + sessionId + " closed while launching app"); }
                case ERROR -> throw new AllwrightException("Windows desktop launch error: " + event.getError().getMessage());
                default -> { }
            }
        }
    }
}
