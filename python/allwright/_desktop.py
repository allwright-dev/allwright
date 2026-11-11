from __future__ import annotations

import threading

from ._hooks import Hook, HookType
from ._mobile import NativeApp, NativeLocator
from ._proto import engine_pb2
from ._transport import RuntimeClient, StreamHandle
from ._types import AllwrightError


class DesktopMacConnectOptions:
    def __init__(self, agent_endpoint: str | None = None, timeout_ms: int | None = None) -> None:
        self.agent_endpoint = agent_endpoint
        self.timeout_ms = timeout_ms


class DesktopMacLaunchOptions:
    def __init__(
        self,
        app_id: str,
        terminate_running: bool = False,
        timeout_ms: int | None = None,
    ) -> None:
        self.app_id = app_id
        self.terminate_running = terminate_running
        self.timeout_ms = timeout_ms


MacLocator = NativeLocator
WindowsLocator = NativeLocator


class DesktopWindow(NativeApp):
    def __init__(self, runtime: RuntimeClient, surface_session_id: str, session_id: str,
                 window_id: str, title: str, is_main: bool) -> None:
        super().__init__(runtime, surface_session_id, session_id)
        self.window_id, self.title, self.is_main = window_id, title, is_main

    def close(self) -> None:
        with self._lock:
            handle = self._ensure_handle()
            self._ensure_open()
            handle.send(engine_pb2.ContextSessionCommand(
                surface_session_id=self._surface_session_id,
                context_session_id=self._session_id,
                close_desktop_window=engine_pb2.CloseDesktopWindowCommand(),
            ))
            while True:
                event = handle.recv("close desktop window")
                kind = event.WhichOneof("event")
                if kind == "desktop_window_closed" and event.desktop_window_closed.window_id == self.window_id:
                    self._closed = True
                    return
                if kind == "error":
                    raise AllwrightError(event.error.message)


class DesktopApp(NativeApp):
    def windows(self) -> list[DesktopWindow]:
        with self._lock:
            handle = self._ensure_handle()
            self._ensure_open()
            handle.send(engine_pb2.ContextSessionCommand(
                surface_session_id=self._surface_session_id,
                context_session_id=self._session_id,
                list_desktop_windows=engine_pb2.ListDesktopWindowsCommand(),
            ))
            while True:
                event = handle.recv("list desktop windows")
                kind = event.WhichOneof("event")
                if kind == "desktop_windows_resolved":
                    return [self._window(result) for result in event.desktop_windows_resolved.windows]
                if kind == "error":
                    raise AllwrightError(event.error.message)

    def main_window(self) -> DesktopWindow:
        for window in self.windows():
            if window.is_main:
                return window
        raise AllwrightError("desktop application does not currently expose a main window")

    def register_hook(self, hook_type: HookType):
        if hook_type.name != "window":
            return super().register_hook(hook_type)
        with self._lock:
            handle = self._ensure_handle()
            self._ensure_open()
            handle.send(engine_pb2.ContextSessionCommand(
                surface_session_id=self._surface_session_id,
                context_session_id=self._session_id,
                register_hook=engine_pb2.RegisterHookCommand(
                    desktop_window=engine_pb2.RegisterDesktopWindowHook()
                ),
            ))
            while True:
                event = handle.recv("register desktop window hook")
                kind = event.WhichOneof("event")
                if kind == "hook_registered":
                    return Hook(self, event.hook_registered.hook_id, hook_type)
                if kind == "error":
                    raise AllwrightError(event.error.message)

    def _wait_for_hook(self, hook_id, hook_type, options):
        if hook_type.name != "window":
            return super()._wait_for_hook(hook_id, hook_type, options)
        from ._runtime import retry_options

        with self._lock:
            handle = self._ensure_handle()
            self._ensure_open()
            handle.send(engine_pb2.ContextSessionCommand(
                surface_session_id=self._surface_session_id,
                context_session_id=self._session_id,
                wait_for_hook=engine_pb2.WaitForHookCommand(
                    hook_id=hook_id,
                    retry_options=retry_options(getattr(options, "timeout_ms", None)),
                ),
            ))
            while True:
                event = handle.recv("wait for desktop window hook")
                kind = event.WhichOneof("event")
                if kind == "hook_completed" and event.hook_completed.hook_id == hook_id:
                    if event.hook_completed.WhichOneof("result") != "desktop_window":
                        raise AllwrightError("desktop window hook returned an invalid result")
                    return self._window(event.hook_completed.desktop_window.window)
                if kind == "error":
                    raise AllwrightError(event.error.message)

    def _window(self, result) -> DesktopWindow:
        if not result.context_session_id or not result.window_id:
            raise AllwrightError("desktop window result is missing its context or window id")
        return DesktopWindow(self._runtime, self._surface_session_id, result.context_session_id,
                             result.window_id, result.title, result.is_main)


MacApp = DesktopApp
WindowsApp = DesktopApp


class MacDesktop:
    def __init__(
        self,
        runtime: RuntimeClient,
        stream: StreamHandle,
        session_id: str,
        surface_session_id: str,
        initial_app_session_id: str,
    ) -> None:
        self._runtime = runtime
        self._stream = stream
        self._lock = threading.Lock()
        self._closed = False
        self._session_id = session_id
        self._surface_session_id = surface_session_id
        self._app = DesktopApp(runtime, surface_session_id, initial_app_session_id)

    @property
    def session_id(self) -> str:
        return self._session_id

    def app(self) -> MacApp:
        return self._app

    def launch(self, options: DesktopMacLaunchOptions) -> MacApp:
        from ._runtime import retry_options

        with self._lock:
            if self._closed:
                raise AllwrightError(f"macOS desktop session {self._session_id} is closed")
            self._stream.send(engine_pb2.SurfaceSessionCommand(
                launch_desktop_app=engine_pb2.LaunchDesktopAppCommand(
                    app_id=options.app_id,
                    terminate_running=options.terminate_running,
                    retry_options=retry_options(options.timeout_ms),
                )
            ))
            while True:
                event = self._stream.recv("receive macOS app launch event")
                match event.WhichOneof("event"):
                    case "desktop_app_launched":
                        self._app = DesktopApp(
                            self._runtime,
                            self._surface_session_id,
                            event.desktop_app_launched.app_session_id,
                        )
                        return self._app
                    case "closed":
                        self._closed = True
                        raise AllwrightError(
                            f"macOS desktop session {self._session_id} closed while launching app"
                        )
                    case "error":
                        raise AllwrightError(event.error.message)


class MacSurface:
    def connect(self, options: DesktopMacConnectOptions | None = None) -> MacDesktop:
        from ._runtime import get_runtime, retry_options

        runtime = get_runtime()
        stream = StreamHandle(runtime.stub.SurfaceSession)
        resolved = options or DesktopMacConnectOptions()
        stream.send(engine_pb2.SurfaceSessionCommand(
            connect_desktop=engine_pb2.ConnectDesktopCommand(
                platform=engine_pb2.DESKTOP_PLATFORM_MAC,
                agent_endpoint=resolved.agent_endpoint,
                retry_options=retry_options(resolved.timeout_ms),
            )
        ))
        while True:
            event = stream.recv("receive macOS desktop connect event")
            match event.WhichOneof("event"):
                case "desktop_connected":
                    connected = event.desktop_connected
                    return MacDesktop(
                        runtime,
                        stream,
                        connected.desktop_session_id or event.session_id,
                        event.session_id,
                        connected.initial_app_session_id,
                    )
                case "error":
                    raise AllwrightError(event.error.message)


DesktopWindowsConnectOptions = DesktopMacConnectOptions
DesktopWindowsLaunchOptions = DesktopMacLaunchOptions
WindowsDesktop = MacDesktop


class WindowsSurface:
    def connect(self, options: DesktopWindowsConnectOptions | None = None) -> WindowsDesktop:
        from ._runtime import get_runtime, retry_options

        runtime = get_runtime()
        stream = StreamHandle(runtime.stub.SurfaceSession)
        resolved = options or DesktopWindowsConnectOptions()
        stream.send(engine_pb2.SurfaceSessionCommand(
            connect_desktop=engine_pb2.ConnectDesktopCommand(
                platform=engine_pb2.DESKTOP_PLATFORM_WINDOWS,
                agent_endpoint=resolved.agent_endpoint,
                retry_options=retry_options(resolved.timeout_ms),
            )
        ))
        while True:
            event = stream.recv("receive Windows desktop connect event")
            match event.WhichOneof("event"):
                case "desktop_connected":
                    connected = event.desktop_connected
                    return WindowsDesktop(
                        runtime,
                        stream,
                        connected.desktop_session_id or event.session_id,
                        event.session_id,
                        connected.initial_app_session_id,
                    )
                case "error":
                    raise AllwrightError(event.error.message)


class DesktopNamespace:
    def __init__(self) -> None:
        self.mac = MacSurface()
        self.windows = WindowsSurface()


desktop = DesktopNamespace()
