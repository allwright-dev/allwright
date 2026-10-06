package dev.allwright.client;

import dev.allwright.engine.v1.ConnectDesktopCommand;
import dev.allwright.engine.v1.DesktopConnectedEvent;
import dev.allwright.engine.v1.DesktopPlatform;
import dev.allwright.engine.v1.SurfaceSessionCommand;
import dev.allwright.engine.v1.SurfaceSessionEvent;

public final class WindowsSurface {
    WindowsSurface() {}
    public WindowsDesktop connect() { return connect(new DesktopWindowsConnectOptions()); }
    public WindowsDesktop connect(DesktopWindowsConnectOptions options) {
        DesktopWindowsConnectOptions resolved = options == null ? new DesktopWindowsConnectOptions() : options;
        RuntimeSupport.RuntimeClient runtime = Allwright.getRuntime();
        RuntimeSupport.StreamHandle<SurfaceSessionCommand, SurfaceSessionEvent> stream =
                new RuntimeSupport.StreamHandle<>(runtime.asyncStub()::surfaceSession);
        ConnectDesktopCommand.Builder connect = ConnectDesktopCommand.newBuilder()
                .setPlatform(DesktopPlatform.DESKTOP_PLATFORM_WINDOWS);
        if (resolved.agentEndpoint() != null && !resolved.agentEndpoint().isBlank()) connect.setAgentEndpoint(resolved.agentEndpoint());
        if (CommandSupport.hasTimeout(resolved.timeoutMs())) connect.setRetryOptions(CommandSupport.commandRetryOptions(resolved.timeoutMs()));
        stream.send(SurfaceSessionCommand.newBuilder().setConnectDesktop(connect).build());
        while (true) {
            SurfaceSessionEvent event = stream.recv("receive Windows desktop connect event");
            switch (event.getEventCase()) {
                case DESKTOP_CONNECTED -> {
                    DesktopConnectedEvent connected = event.getDesktopConnected();
                    String sessionId = connected.getDesktopSessionId().isBlank() ? event.getSessionId() : connected.getDesktopSessionId();
                    return new WindowsDesktop(runtime, stream, sessionId, event.getSessionId(), connected.getInitialAppSessionId());
                }
                case ERROR -> throw new AllwrightException("Windows desktop connect error: " + event.getError().getMessage());
                default -> { }
            }
        }
    }
}
