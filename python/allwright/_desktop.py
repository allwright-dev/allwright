from __future__ import annotations

import threading

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


MacApp = NativeApp
MacLocator = NativeLocator


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
        self._app = NativeApp(runtime, surface_session_id, initial_app_session_id)

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
                        self._app = NativeApp(
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


class DesktopNamespace:
    def __init__(self) -> None:
        self.mac = MacSurface()


desktop = DesktopNamespace()
