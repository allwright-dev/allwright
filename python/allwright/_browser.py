from __future__ import annotations

import threading

from ._proto import engine_pb2
from ._types import AllwrightError, CommandOptions, LaunchOptions


class BrowserType:
    def __init__(self, browser_kind: int = engine_pb2.BROWSER_KIND_CHROMIUM) -> None:
        self._browser_kind = browser_kind

    def launch(self, options: LaunchOptions | None = None) -> Browser:
        from .client import launch_browser

        return launch_browser(self._browser_kind, options or LaunchOptions())


class Browser:
    def __init__(
        self,
        runtime: RuntimeClient,
        stream: StreamHandle,
        session_id: str,
        browser_name: str,
        launch_note: str,
        cdp_websocket_url: str,
        user_data_dir: str,
        initial_page: Page,
    ) -> None:
        self._runtime = runtime
        self._stream = stream
        self._lock = threading.Lock()
        self._closed = False
        self._pages: dict[str, Page] = {initial_page.session_id: initial_page}
        self.session_id = session_id
        self.browser_name = browser_name
        self.launch_note = launch_note
        self.cdp_websocket_url = cdp_websocket_url
        self.user_data_dir = user_data_dir
        self._initial_page = initial_page

    def page(self) -> Page:
        return self._initial_page

    def initial_page(self) -> Page:
        return self._initial_page

    def initial_tab(self) -> Page:
        return self._initial_page

    def pages(self) -> list[Page]:
        return list(self._pages.values())

    def new_page(self, options: CommandOptions | None = None) -> Page:
        from ._page import Page
        from ._runtime import retry_options

        with self._lock:
            self._ensure_open()
            command_options = options or CommandOptions()
            self._stream.send(
                engine_pb2.BrowserSessionCommand(
                    open_tab=engine_pb2.OpenTabCommand(
                        retry_options=retry_options(command_options.timeout_ms),
                    ),
                )
            )

            while True:
                event = self._stream.recv("receive browser session event while opening page")
                match event.WhichOneof("event"):
                    case "tab_opened":
                        page = Page(
                            runtime=self._runtime,
                            browser_session_id=self.session_id,
                            session_id=event.tab_opened.tab_session_id,
                        )
                        self._pages[page.session_id] = page
                        return page
                    case "error":
                        raise AllwrightError(
                            f"browser session error while opening page: {event.error.message}"
                        )

    def new_tab(self, options: CommandOptions | None = None) -> Page:
        return self.new_page(options)

    def ping(self, message: str = "ping") -> str:
        with self._lock:
            self._ensure_open()
            self._stream.send(
                engine_pb2.BrowserSessionCommand(
                    ping=engine_pb2.SessionPingCommand(message=message),
                )
            )

            while True:
                event = self._stream.recv("receive browser session event while pinging browser")
                match event.WhichOneof("event"):
                    case "pong":
                        return event.pong.message
                    case "error":
                        raise AllwrightError(
                            f"browser session error while pinging: {event.error.message}"
                        )

    def close(self) -> None:
        with self._lock:
            if self._closed:
                return

            self._stream.send(
                engine_pb2.BrowserSessionCommand(
                    close=engine_pb2.CloseBrowserSessionCommand(),
                )
            )

            while True:
                event = self._stream.recv("receive browser session event while closing browser")
                match event.WhichOneof("event"):
                    case "closed":
                        self._closed = True
                        self._stream.close_send()
                        return
                    case "error":
                        raise AllwrightError(
                            f"browser session error while closing: {event.error.message}"
                        )

    def _ensure_open(self) -> None:
        if self._closed:
            raise AllwrightError(f"browser session {self.session_id} is closed")


from ._page import Page
from ._transport import RuntimeClient, StreamHandle
