package dev.allwright.client;

import dev.allwright.engine.v1.SurfaceSessionCommand;
import dev.allwright.engine.v1.SurfaceSessionEvent;
import dev.allwright.engine.v1.ConnectMobileCommand;
import dev.allwright.engine.v1.MobileConnectedEvent;
import dev.allwright.engine.v1.MobilePlatform;

public final class AndroidSurface {
    AndroidSurface() {}

    public AndroidDevice connect() {
        return connect(new MobileAndroidConnectOptions());
    }

    public AndroidDevice connect(MobileAndroidConnectOptions options) {
        MobileAndroidConnectOptions resolved = options == null ? new MobileAndroidConnectOptions() : options;
        RuntimeSupport.RuntimeClient runtime = Allwright.getRuntime();
        RuntimeSupport.StreamHandle<SurfaceSessionCommand, SurfaceSessionEvent> stream =
                new RuntimeSupport.StreamHandle<>(runtime.asyncStub()::surfaceSession);

        ConnectMobileCommand.Builder connect = ConnectMobileCommand.newBuilder()
                .setPlatform(MobilePlatform.MOBILE_PLATFORM_ANDROID)
                .setPreserveAppState(resolved.preserveAppState());
        if (resolved.device() != null && !resolved.device().isBlank()) {
            connect.setDevice(resolved.device());
        }
        if (resolved.adbEndpoint() != null && !resolved.adbEndpoint().isBlank()) {
            connect.setAdbEndpoint(resolved.adbEndpoint());
        }
        if (CommandSupport.hasTimeout(resolved.timeoutMs())) {
            connect.setRetryOptions(CommandSupport.commandRetryOptions(resolved.timeoutMs()));
        }

        stream.send(SurfaceSessionCommand.newBuilder().setConnectMobile(connect).build());

        while (true) {
            SurfaceSessionEvent event = stream.recv("receive browser session event while connecting Android device");
            switch (event.getEventCase()) {
                case MOBILE_CONNECTED -> {
                    MobileConnectedEvent connected = event.getMobileConnected();
                    String sessionId = connected.getDeviceSessionId().isBlank()
                            ? event.getSessionId()
                            : connected.getDeviceSessionId();
                    return new AndroidDevice(
                            runtime,
                            stream,
                            sessionId,
                            event.getSessionId(),
                            connected.getInitialAppSessionId()
                    );
                }
                case ERROR -> throw new AllwrightException(
                        "device session error during Android connect: " + event.getError().getMessage()
                );
                default -> {
                }
            }
        }
    }
}
