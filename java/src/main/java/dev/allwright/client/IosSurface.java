package dev.allwright.client;

import dev.allwright.engine.v1.ConnectMobileCommand;
import dev.allwright.engine.v1.MobileConnectedEvent;
import dev.allwright.engine.v1.MobilePlatform;
import dev.allwright.engine.v1.SurfaceSessionCommand;
import dev.allwright.engine.v1.SurfaceSessionEvent;

public final class IosSurface {
    IosSurface() {}

    public IosDevice connect() {
        return connect(new MobileIosConnectOptions());
    }

    public IosDevice connect(MobileIosConnectOptions options) {
        MobileIosConnectOptions resolved = options == null ? new MobileIosConnectOptions() : options;
        RuntimeSupport.RuntimeClient runtime = Allwright.getRuntime();
        RuntimeSupport.StreamHandle<SurfaceSessionCommand, SurfaceSessionEvent> stream =
                new RuntimeSupport.StreamHandle<>(runtime.asyncStub()::surfaceSession);
        ConnectMobileCommand.Builder connect = ConnectMobileCommand.newBuilder()
                .setPlatform(MobilePlatform.MOBILE_PLATFORM_IOS)
                .setPreserveAppState(resolved.preserveAppState());
        if (resolved.device() != null && !resolved.device().isBlank()) connect.setDevice(resolved.device());
        // The protobuf field is transport-internal; the public option is the iOS agent endpoint.
        if (resolved.agentEndpoint() != null && !resolved.agentEndpoint().isBlank()) {
            connect.setAdbEndpoint(resolved.agentEndpoint());
        }
        if (CommandSupport.hasTimeout(resolved.timeoutMs())) {
            connect.setRetryOptions(CommandSupport.commandRetryOptions(resolved.timeoutMs()));
        }
        stream.send(SurfaceSessionCommand.newBuilder().setConnectMobile(connect).build());
        while (true) {
            SurfaceSessionEvent event = stream.recv("receive iOS connect event");
            switch (event.getEventCase()) {
                case MOBILE_CONNECTED -> {
                    MobileConnectedEvent connected = event.getMobileConnected();
                    String sessionId = connected.getDeviceSessionId().isBlank()
                            ? event.getSessionId() : connected.getDeviceSessionId();
                    return new IosDevice(runtime, stream, sessionId, event.getSessionId(),
                            connected.getInitialAppSessionId());
                }
                case ERROR -> throw new AllwrightException(
                        "iOS device session error: " + event.getError().getMessage());
                default -> { }
            }
        }
    }
}
