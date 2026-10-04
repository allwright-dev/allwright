package dev.allwright.client;

import dev.allwright.engine.v1.ConnectDesktopCommand;
import dev.allwright.engine.v1.DesktopConnectedEvent;
import dev.allwright.engine.v1.DesktopPlatform;
import dev.allwright.engine.v1.SurfaceSessionCommand;
import dev.allwright.engine.v1.SurfaceSessionEvent;

public final class MacSurface {
    MacSurface() {}

    public MacDesktop connect() {
        return connect(new DesktopMacConnectOptions());
    }

    public MacDesktop connect(DesktopMacConnectOptions options) {
        DesktopMacConnectOptions resolved = options == null ? new DesktopMacConnectOptions() : options;
        RuntimeSupport.RuntimeClient runtime = Allwright.getRuntime();
        RuntimeSupport.StreamHandle<SurfaceSessionCommand, SurfaceSessionEvent> stream =
                new RuntimeSupport.StreamHandle<>(runtime.asyncStub()::surfaceSession);
        ConnectDesktopCommand.Builder connect = ConnectDesktopCommand.newBuilder()
                .setPlatform(DesktopPlatform.DESKTOP_PLATFORM_MAC);
        if (resolved.agentEndpoint() != null && !resolved.agentEndpoint().isBlank()) {
            connect.setAgentEndpoint(resolved.agentEndpoint());
        }
        if (CommandSupport.hasTimeout(resolved.timeoutMs())) {
            connect.setRetryOptions(CommandSupport.commandRetryOptions(resolved.timeoutMs()));
        }
        stream.send(SurfaceSessionCommand.newBuilder().setConnectDesktop(connect).build());

        while (true) {
            SurfaceSessionEvent event = stream.recv("receive mac desktop connect event");
            switch (event.getEventCase()) {
                case DESKTOP_CONNECTED -> {
                    DesktopConnectedEvent connected = event.getDesktopConnected();
                    String sessionId = connected.getDesktopSessionId().isBlank()
                            ? event.getSessionId() : connected.getDesktopSessionId();
                    return new MacDesktop(runtime, stream, sessionId, event.getSessionId(),
                            connected.getInitialAppSessionId());
                }
                case ERROR -> throw new AllwrightException(
                        "mac desktop connect error: " + event.getError().getMessage());
                default -> { }
            }
        }
    }
}
