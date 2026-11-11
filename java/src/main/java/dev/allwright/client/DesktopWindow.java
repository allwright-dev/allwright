package dev.allwright.client;

import dev.allwright.engine.v1.CloseDesktopWindowCommand;
import dev.allwright.engine.v1.ContextSessionCommand;
import dev.allwright.engine.v1.ContextSessionEvent;

public final class DesktopWindow extends NativeApp {
    private final String windowId;
    private final String title;
    private final boolean main;

    DesktopWindow(RuntimeSupport.RuntimeClient runtime, String surfaceSessionId, String sessionId,
                  String windowId, String title, boolean main) {
        super(runtime, surfaceSessionId, sessionId);
        this.windowId = windowId;
        this.title = title;
        this.main = main;
    }

    public String windowId() { return windowId; }
    public String title() { return title; }
    public boolean isMain() { return main; }

    public synchronized void close() {
        var handle = ensureStream();
        ensureOpen();
        handle.send(ContextSessionCommand.newBuilder().setSurfaceSessionId(surfaceSessionId())
                .setContextSessionId(sessionId())
                .setCloseDesktopWindow(CloseDesktopWindowCommand.newBuilder()).build());
        while (true) {
            ContextSessionEvent event = handle.recv("close desktop window");
            if (event.getEventCase() == ContextSessionEvent.EventCase.DESKTOP_WINDOW_CLOSED
                    && windowId.equals(event.getDesktopWindowClosed().getWindowId())) return;
            if (event.getEventCase() == ContextSessionEvent.EventCase.ERROR) {
                throw new AllwrightException(event.getError().getMessage());
            }
        }
    }
}
