package dev.allwright.client;

import dev.allwright.engine.v1.ContextSessionCommand;
import dev.allwright.engine.v1.ContextSessionEvent;
import dev.allwright.engine.v1.DesktopWindowInfo;
import dev.allwright.engine.v1.ListDesktopWindowsCommand;
import dev.allwright.engine.v1.RegisterDesktopWindowHook;
import dev.allwright.engine.v1.RegisterHookCommand;
import java.util.ArrayList;
import java.util.List;

public class DesktopApp extends NativeApp {
    DesktopApp(RuntimeSupport.RuntimeClient runtime, String surfaceSessionId, String sessionId) {
        super(runtime, surfaceSessionId, sessionId);
    }

    public synchronized List<DesktopWindow> windows() {
        var handle = ensureStream();
        ensureOpen();
        handle.send(ContextSessionCommand.newBuilder().setSurfaceSessionId(surfaceSessionId())
                .setContextSessionId(sessionId())
                .setListDesktopWindows(ListDesktopWindowsCommand.newBuilder()).build());
        while (true) {
            ContextSessionEvent event = handle.recv("list desktop windows");
            if (event.getEventCase() == ContextSessionEvent.EventCase.DESKTOP_WINDOWS_RESOLVED) {
                List<DesktopWindow> result = new ArrayList<>();
                for (DesktopWindowInfo window : event.getDesktopWindowsResolved().getWindowsList()) {
                    result.add(windowFromInfo(window));
                }
                return List.copyOf(result);
            }
            if (event.getEventCase() == ContextSessionEvent.EventCase.ERROR) {
                throw new AllwrightException(event.getError().getMessage());
            }
        }
    }

    public DesktopWindow mainWindow() {
        return windows().stream().filter(DesktopWindow::isMain).findFirst()
                .orElseThrow(() -> new AllwrightException(
                        "desktop application does not currently expose a main window"));
    }

    @Override
    public synchronized <T> Hook<T> registerHook(HookType<T> type) {
        if (!"window".equals(type.name())) return super.registerHook(type);
        var handle = ensureStream();
        ensureOpen();
        var register = RegisterHookCommand.newBuilder()
                .setDesktopWindow(RegisterDesktopWindowHook.newBuilder()).build();
        handle.send(ContextSessionCommand.newBuilder().setSurfaceSessionId(surfaceSessionId())
                .setContextSessionId(sessionId()).setRegisterHook(register).build());
        while (true) {
            ContextSessionEvent event = handle.recv("register desktop window hook");
            if (event.getEventCase() == ContextSessionEvent.EventCase.HOOK_REGISTERED) {
                return new Hook<>(this, event.getHookRegistered().getHookId(), type);
            }
            if (event.getEventCase() == ContextSessionEvent.EventCase.ERROR) {
                throw new AllwrightException(event.getError().getMessage());
            }
        }
    }

    DesktopWindow windowFromInfo(DesktopWindowInfo info) {
        if (info.getContextSessionId().isBlank() || info.getWindowId().isBlank()) {
            throw new AllwrightException("desktop window result is missing its context or window id");
        }
        return new DesktopWindow(runtime(), surfaceSessionId(), info.getContextSessionId(),
                info.getWindowId(), info.getTitle(), info.getIsMain());
    }
}
