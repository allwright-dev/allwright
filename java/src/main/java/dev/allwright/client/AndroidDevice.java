package dev.allwright.client;

import dev.allwright.engine.v1.AppLaunchedEvent;
import dev.allwright.engine.v1.SurfaceSessionCommand;
import dev.allwright.engine.v1.SurfaceSessionEvent;
import dev.allwright.engine.v1.LaunchAppCommand;

public final class AndroidDevice {
    private final RuntimeSupport.RuntimeClient runtime;
    private final RuntimeSupport.StreamHandle<SurfaceSessionCommand, SurfaceSessionEvent> stream;
    private final String sessionId;
    private final String surfaceSessionId;
    private AndroidApp app;
    private boolean closed;

    AndroidDevice(
            RuntimeSupport.RuntimeClient runtime,
            RuntimeSupport.StreamHandle<SurfaceSessionCommand, SurfaceSessionEvent> stream,
            String sessionId,
            String surfaceSessionId,
            String initialAppSessionId
    ) {
        this.runtime = runtime;
        this.stream = stream;
        this.sessionId = sessionId;
        this.surfaceSessionId = surfaceSessionId;
        this.app = new AndroidApp(runtime, surfaceSessionId, initialAppSessionId);
    }

    public String sessionId() {
        return sessionId;
    }

    public AndroidApp app() {
        return app;
    }

    public AndroidApp initialApp() {
        return app();
    }

    public synchronized AndroidApp launch() {
        return launch(new MobileAndroidLaunchOptions());
    }

    public synchronized AndroidApp launch(MobileAndroidLaunchOptions options) {
        ensureOpen();
        MobileAndroidLaunchOptions resolved = options == null ? new MobileAndroidLaunchOptions() : options;
        LaunchAppCommand.Builder launch = LaunchAppCommand.newBuilder()
                .setStopBeforeLaunch(resolved.stopBeforeLaunch());
        if (resolved.apkPath() != null && !resolved.apkPath().isBlank()) {
            launch.setApkPath(resolved.apkPath());
        }
        if (resolved.appId() != null && !resolved.appId().isBlank()) {
            launch.setAppId(resolved.appId());
        }
        if (resolved.launchActivity() != null && !resolved.launchActivity().isBlank()) {
            launch.setLaunchActivity(resolved.launchActivity());
        }
        if (CommandSupport.hasTimeout(resolved.timeoutMs())) {
            launch.setRetryOptions(CommandSupport.commandRetryOptions(resolved.timeoutMs()));
        }

        stream.send(SurfaceSessionCommand.newBuilder().setLaunchApp(launch).build());

        while (true) {
            SurfaceSessionEvent event = stream.recv("receive browser session event while launching Android app");
            switch (event.getEventCase()) {
                case APP_LAUNCHED -> {
                    AppLaunchedEvent launched = event.getAppLaunched();
                    app = new AndroidApp(runtime, surfaceSessionId, launched.getAppSessionId());
                    return app;
                }
                case CLOSED -> {
                    closed = true;
                    throw new AllwrightException("android device session " + sessionId + " closed while launching app");
                }
                case ERROR -> throw new AllwrightException(
                        "android device session error while launching app: " + event.getError().getMessage()
                );
                default -> {
                }
            }
        }
    }

    private void ensureOpen() {
        if (closed) {
            throw new AllwrightException("android device session " + sessionId + " is closed");
        }
    }
}
