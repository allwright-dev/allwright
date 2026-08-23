from __future__ import annotations

import os
import queue
import threading
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Callable, Iterator

import grpc

DEFAULT_SERVER_ADDR = "127.0.0.1:50051"
SERVER_ADDR_ENV_VAR = "ALLWRIGHT_SERVER_ADDR"
PROTO_PATH = Path(__file__).resolve().parents[2] / "proto" / "engine" / "v1" / "engine.proto"
_STREAM_SENTINEL = object()

engine_pb2, engine_pb2_grpc = grpc.protos_and_services(str(PROTO_PATH))

_runtime_lock = threading.Lock()
_runtime_client: _RuntimeClient | None = None
_server_addr_override: str | None = None


class AllwrightError(RuntimeError):
    pass


@dataclass(slots=True)
class LaunchOptions:
    chrome_binary: str | None = None


@dataclass(slots=True)
class NavigateResult:
    url: str
    note: str
    bidi_session_id: str
    mapper_target_id: str
    mapper_session_id: str
    package_version: str


@dataclass(slots=True)
class ClickResult:
    selector: str
    note: str
    bidi_session_id: str


class BrowserType:
    def launch(self, options: LaunchOptions | None = None) -> Browser:
        return launch_chrome(options or LaunchOptions())


class Browser:
    def __init__(
        self,
        runtime: _RuntimeClient,
        stream: _StreamHandle,
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

    def new_page(self) -> Page:
        with self._lock:
            self._ensure_open()
            self._stream.send(
                engine_pb2.BrowserSessionCommand(
                    open_tab=engine_pb2.OpenTabCommand(),
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

    def new_tab(self) -> Page:
        return self.new_page()

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


class Page:
    def __init__(self, runtime: _RuntimeClient, browser_session_id: str, session_id: str) -> None:
        self._runtime = runtime
        self._browser_session_id = browser_session_id
        self._session_id = session_id
        self._lock = threading.Lock()
        self._handle: _StreamHandle | None = None
        self._closed = False

    @property
    def session_id(self) -> str:
        return self._session_id

    @property
    def browser_session_id(self) -> str:
        return self._browser_session_id

    def goto(self, url: str) -> NavigateResult:
        with self._lock:
            handle = self._ensure_handle()
            self._ensure_open()
            handle.send(
                engine_pb2.TabSessionCommand(
                    browser_session_id=self.browser_session_id,
                    tab_session_id=self.session_id,
                    navigate=engine_pb2.NavigateTabCommand(url=url),
                )
            )

            navigated: Any | None = None
            injection: Any | None = None

            while True:
                event = handle.recv("receive tab session event while navigating")
                match event.WhichOneof("event"):
                    case "navigated":
                        navigated = event.navigated
                    case "chromium_bidi_injection":
                        injection = event.chromium_bidi_injection
                    case "closed":
                        self._closed = True
                        raise AllwrightError(
                            f"page session {self.session_id} closed while navigating"
                        )
                    case "error":
                        raise AllwrightError(
                            f"page session error while navigating: {event.error.message}"
                        )

                if navigated is not None and injection is not None:
                    return NavigateResult(
                        url=navigated.url,
                        note=navigated.note,
                        bidi_session_id=injection.bidi_session_id,
                        mapper_target_id=injection.mapper_target_id,
                        mapper_session_id=injection.mapper_session_id,
                        package_version=injection.package_version,
                    )

    def navigate(self, url: str) -> NavigateResult:
        return self.goto(url)

    def click(self, selector: str) -> ClickResult:
        with self._lock:
            handle = self._ensure_handle()
            self._ensure_open()
            handle.send(
                engine_pb2.TabSessionCommand(
                    browser_session_id=self.browser_session_id,
                    tab_session_id=self.session_id,
                    click_element=engine_pb2.ClickElementCommand(css_selector=selector),
                )
            )

            while True:
                event = handle.recv("receive tab session event while clicking")
                match event.WhichOneof("event"):
                    case "element_clicked":
                        clicked = event.element_clicked
                        return ClickResult(
                            selector=clicked.css_selector,
                            note=clicked.note,
                            bidi_session_id=clicked.bidi_session_id,
                        )
                    case "closed":
                        self._closed = True
                        raise AllwrightError(
                            f"page session {self.session_id} closed while clicking"
                        )
                    case "error":
                        raise AllwrightError(
                            f"page session error while clicking: {event.error.message}"
                        )

    def ping(self, message: str = "ping") -> str:
        with self._lock:
            handle = self._ensure_handle()
            self._ensure_open()
            handle.send(
                engine_pb2.TabSessionCommand(
                    browser_session_id=self.browser_session_id,
                    tab_session_id=self.session_id,
                    ping=engine_pb2.TabSessionPingCommand(message=message),
                )
            )

            while True:
                event = handle.recv("receive tab session event while pinging page")
                match event.WhichOneof("event"):
                    case "pong":
                        return event.pong.message
                    case "closed":
                        self._closed = True
                        raise AllwrightError(
                            f"page session {self.session_id} closed while pinging"
                        )
                    case "error":
                        raise AllwrightError(
                            f"page session error while pinging: {event.error.message}"
                        )

    def close(self) -> None:
        with self._lock:
            handle = self._ensure_handle()
            if self._closed:
                return

            handle.send(
                engine_pb2.TabSessionCommand(
                    browser_session_id=self.browser_session_id,
                    tab_session_id=self.session_id,
                    close=engine_pb2.CloseTabSessionCommand(),
                )
            )

            while True:
                event = handle.recv("receive tab session event while closing page")
                match event.WhichOneof("event"):
                    case "closed":
                        self._closed = True
                        handle.close_send()
                        return
                    case "error":
                        raise AllwrightError(
                            f"page session error while closing: {event.error.message}"
                        )

    def _ensure_handle(self) -> _StreamHandle:
        if self._handle is None:
            self._handle = _StreamHandle(self._runtime.stub.TabSession)
        return self._handle

    def _ensure_open(self) -> None:
        if self._closed:
            raise AllwrightError(f"page session {self.session_id} is closed")


class _StreamHandle:
    def __init__(self, stream_factory: Callable[[Iterator[Any]], Iterator[Any]]) -> None:
        self._queue: queue.Queue[Any] = queue.Queue()
        self._responses = stream_factory(self._request_iterator())
        self._send_closed = False

    def send(self, message: Any) -> None:
        if self._send_closed:
            raise AllwrightError("cannot send on a closed stream")
        self._queue.put(message)

    def recv(self, action: str) -> Any:
        try:
            return next(self._responses)
        except StopIteration as exc:
            raise AllwrightError(f"{action}: stream ended unexpectedly") from exc
        except grpc.RpcError as exc:
            raise AllwrightError(f"{action}: {exc}") from exc

    def close_send(self) -> None:
        if self._send_closed:
            return
        self._send_closed = True
        self._queue.put(_STREAM_SENTINEL)

    def _request_iterator(self) -> Iterator[Any]:
        while True:
            item = self._queue.get()
            if item is _STREAM_SENTINEL:
                return
            yield item


class _RuntimeClient:
    def __init__(self, server_addr: str) -> None:
        self.channel = grpc.insecure_channel(server_addr)
        self.stub = engine_pb2_grpc.EngineServiceStub(self.channel)

    def close(self) -> None:
        self.channel.close()


def ping() -> str:
    runtime = _get_runtime()
    response = runtime.stub.Ping(engine_pb2.PingRequest())
    return response.message


def launch_chrome(options: LaunchOptions | None = None) -> Browser:
    runtime = _get_runtime()
    stream = _StreamHandle(runtime.stub.BrowserSession)
    launch_options = options or LaunchOptions()

    command_kwargs: dict[str, Any] = {}
    if launch_options.chrome_binary:
        command_kwargs["chrome_binary"] = launch_options.chrome_binary

    stream.send(
        engine_pb2.BrowserSessionCommand(
            launch_chrome=engine_pb2.LaunchChromeCommand(**command_kwargs),
        )
    )

    while True:
        event = stream.recv("receive browser session event during launch")
        match event.WhichOneof("event"):
            case "chrome_launched":
                launched = event.chrome_launched
                initial_page = Page(
                    runtime=runtime,
                    browser_session_id=event.session_id,
                    session_id=launched.initial_tab_session_id,
                )
                return Browser(
                    runtime=runtime,
                    stream=stream,
                    session_id=event.session_id,
                    browser_name=launched.browser,
                    launch_note=launched.note,
                    cdp_websocket_url=launched.cdp_websocket_url,
                    user_data_dir=launched.user_data_dir,
                    initial_page=initial_page,
                )
            case "error":
                raise AllwrightError(
                    f"browser session error during launch: {event.error.message}"
                )


def set_server_addr(server_addr: str) -> None:
    global _runtime_client, _server_addr_override
    normalized = server_addr.strip()
    with _runtime_lock:
        _server_addr_override = normalized
        if _runtime_client is not None:
            _runtime_client.close()
            _runtime_client = None


def shutdown() -> None:
    global _runtime_client
    with _runtime_lock:
        if _runtime_client is not None:
            _runtime_client.close()
            _runtime_client = None


def _get_runtime() -> _RuntimeClient:
    global _runtime_client
    with _runtime_lock:
        if _runtime_client is None:
            _runtime_client = _RuntimeClient(_resolve_server_addr())
        return _runtime_client


def _resolve_server_addr() -> str:
    if _server_addr_override:
        return _server_addr_override

    value = os.getenv(SERVER_ADDR_ENV_VAR, "").strip()
    if value:
        return value
    return DEFAULT_SERVER_ADDR


chromium = BrowserType()
