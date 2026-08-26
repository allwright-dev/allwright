from __future__ import annotations

import json
import os
import queue
import threading
from contextlib import contextmanager
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Callable, Iterator

import grpc
import yaml

DEFAULT_SERVER_ADDR = "127.0.0.1:50051"
SERVER_ADDR_ENV_VAR = "ALLWRIGHT_SERVER_ADDR"
PROTO_ROOT = Path(__file__).resolve().parent / "proto"
PROTO_RELATIVE_PATH = Path("engine") / "v1" / "engine.proto"
_STREAM_SENTINEL = object()
CONFIG_FILENAMES = (
    "allwright.config.yaml",
    "allwright.config.yml",
    "allwright.config.json",
    ".allwright/config.yaml",
    ".allwright/config.yml",
    ".allwright/config.json",
)

_runtime_lock = threading.Lock()
_runtime_client: _RuntimeClient | None = None
_server_addr_override: str | None = None


@contextmanager
def _proto_cwd(path: Path) -> Iterator[None]:
    previous = Path.cwd()
    os.chdir(path)
    try:
        yield
    finally:
        os.chdir(previous)


with _proto_cwd(PROTO_ROOT):
    engine_pb2, engine_pb2_grpc = grpc.protos_and_services(str(PROTO_RELATIVE_PATH))


class AllwrightError(RuntimeError):
    pass


@dataclass(slots=True)
class LaunchOptions:
    browser_binary: str | None = None
    timeout_ms: int | None = None


@dataclass(slots=True)
class RetryConfig:
    timeout_ms: int | None = None
    interval_ms: int | None = None


@dataclass(slots=True)
class AllwrightConfig:
    schema_version: int | None = None
    server: dict[str, Any] | None = None
    browser: dict[str, Any] | None = None
    expect: RetryConfig | None = None
    suites: dict[str, dict[str, Any]] | None = None


@dataclass(slots=True)
class ResolveConfigOptions:
    cwd: str | Path | None = None
    config_file: str | Path | None = None
    suite: str | None = None


@dataclass(slots=True)
class ResolvedConfig:
    config_file_path: Path | None
    suite_name: str | None
    server_addr: str | None
    browser_name: str
    browser_binary: str | None
    launch_options: LaunchOptions
    expect: RetryConfig


@dataclass(slots=True)
class CommandOptions:
    timeout_ms: int | None = None


@dataclass(slots=True)
class HighlightOptions:
    timeout_ms: int | None = None
    duration_ms: int | None = None


@dataclass(slots=True)
class PressOptions:
    timeout_ms: int | None = None
    text: str | None = None


@dataclass(slots=True)
class WaitForSelectorOptions:
    timeout_ms: int | None = None
    visible: bool | None = None


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


@dataclass(slots=True)
class CountResult:
    selector: str
    count: int
    note: str


@dataclass(slots=True)
class HighlightResult:
    selector: str
    count: int
    note: str


@dataclass(slots=True)
class ElementResult:
    selector: str
    note: str


@dataclass(slots=True)
class FillResult:
    selector: str
    value: str
    note: str


@dataclass(slots=True)
class PressResult:
    selector: str
    key: str
    note: str


@dataclass(slots=True)
class TextResult:
    selector: str
    text: str
    note: str


@dataclass(slots=True)
class WaitForSelectorResult:
    selector: str
    visible: bool
    note: str


class BrowserType:
    def __init__(self, browser_kind: int = engine_pb2.BROWSER_KIND_CHROMIUM) -> None:
        self._browser_kind = browser_kind

    def launch(self, options: LaunchOptions | None = None) -> Browser:
        return launch_browser(self._browser_kind, options or LaunchOptions())


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

    def new_page(self, options: CommandOptions | None = None) -> Page:
        with self._lock:
            self._ensure_open()
            command_options = options or CommandOptions()
            self._stream.send(
                engine_pb2.BrowserSessionCommand(
                    open_tab=engine_pb2.OpenTabCommand(
                        retry_options=_retry_options(command_options.timeout_ms),
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

    def locator(self, selector: str) -> Locator:
        return Locator(page=self, selector=selector)

    def goto(self, url: str, options: CommandOptions | None = None) -> NavigateResult:
        with self._lock:
            handle = self._ensure_handle()
            self._ensure_open()
            command_options = options or CommandOptions()
            handle.send(
                engine_pb2.TabSessionCommand(
                    browser_session_id=self.browser_session_id,
                    tab_session_id=self.session_id,
                    navigate=engine_pb2.NavigateTabCommand(
                        url=url,
                        retry_options=_retry_options(command_options.timeout_ms),
                    ),
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

    def navigate(self, url: str, options: CommandOptions | None = None) -> NavigateResult:
        return self.goto(url, options)

    def click(self, selector: str, options: CommandOptions | None = None) -> ClickResult:
        with self._lock:
            handle = self._ensure_handle()
            self._ensure_open()
            command_options = options or CommandOptions()
            handle.send(
                engine_pb2.TabSessionCommand(
                    browser_session_id=self.browser_session_id,
                    tab_session_id=self.session_id,
                    click_element=engine_pb2.ClickElementCommand(
                        css_selector=selector,
                        retry_options=_retry_options(command_options.timeout_ms),
                    ),
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

    def count(self, selector: str, options: CommandOptions | None = None) -> CountResult:
        with self._lock:
            handle = self._ensure_handle()
            self._ensure_open()
            command_options = options or CommandOptions()
            handle.send(
                engine_pb2.TabSessionCommand(
                    browser_session_id=self.browser_session_id,
                    tab_session_id=self.session_id,
                    count_elements=engine_pb2.CountElementsCommand(
                        css_selector=selector,
                        retry_options=_retry_options(command_options.timeout_ms),
                    ),
                )
            )

            while True:
                event = handle.recv("receive tab session event while counting elements")
                match event.WhichOneof("event"):
                    case "element_counted":
                        counted = event.element_counted
                        return CountResult(
                            selector=counted.css_selector,
                            count=counted.count,
                            note=counted.note,
                        )
                    case "closed":
                        self._closed = True
                        raise AllwrightError(
                            f"page session {self.session_id} closed while counting elements"
                        )
                    case "error":
                        raise AllwrightError(
                            f"page session error while counting elements: {event.error.message}"
                        )

    def highlight(self, selector: str, options: HighlightOptions | None = None) -> HighlightResult:
        with self._lock:
            handle = self._ensure_handle()
            self._ensure_open()
            highlight_options = options or HighlightOptions()
            handle.send(
                engine_pb2.TabSessionCommand(
                    browser_session_id=self.browser_session_id,
                    tab_session_id=self.session_id,
                    highlight_elements=engine_pb2.HighlightElementsCommand(
                        css_selector=selector,
                        duration_ms=highlight_options.duration_ms,
                        retry_options=_retry_options(highlight_options.timeout_ms),
                    ),
                )
            )

            while True:
                event = handle.recv("receive tab session event while highlighting elements")
                match event.WhichOneof("event"):
                    case "elements_highlighted":
                        highlighted = event.elements_highlighted
                        return HighlightResult(
                            selector=highlighted.css_selector,
                            count=highlighted.count,
                            note=highlighted.note,
                        )
                    case "closed":
                        self._closed = True
                        raise AllwrightError(
                            f"page session {self.session_id} closed while highlighting elements"
                        )
                    case "error":
                        raise AllwrightError(
                            f"page session error while highlighting elements: {event.error.message}"
                        )

    def focus(self, selector: str, options: CommandOptions | None = None) -> ElementResult:
        return self._element_command(
            action="focusing",
            event_name="element_focused",
            command=engine_pb2.TabSessionCommand(
                browser_session_id=self.browser_session_id,
                tab_session_id=self.session_id,
                focus_element=engine_pb2.FocusElementCommand(
                    css_selector=selector,
                    retry_options=_retry_options((options or CommandOptions()).timeout_ms),
                ),
            ),
        )

    def hover(self, selector: str, options: CommandOptions | None = None) -> ElementResult:
        return self._element_command(
            action="hovering",
            event_name="element_hovered",
            command=engine_pb2.TabSessionCommand(
                browser_session_id=self.browser_session_id,
                tab_session_id=self.session_id,
                hover_element=engine_pb2.HoverElementCommand(
                    css_selector=selector,
                    retry_options=_retry_options((options or CommandOptions()).timeout_ms),
                ),
            ),
        )

    def fill(
        self,
        selector: str,
        value: str,
        options: CommandOptions | None = None,
    ) -> FillResult:
        with self._lock:
            handle = self._ensure_handle()
            self._ensure_open()
            command_options = options or CommandOptions()
            handle.send(
                engine_pb2.TabSessionCommand(
                    browser_session_id=self.browser_session_id,
                    tab_session_id=self.session_id,
                    fill_element=engine_pb2.FillElementCommand(
                        css_selector=selector,
                        value=value,
                        retry_options=_retry_options(command_options.timeout_ms),
                    ),
                )
            )

            while True:
                event = handle.recv("receive tab session event while filling")
                match event.WhichOneof("event"):
                    case "element_filled":
                        filled = event.element_filled
                        return FillResult(
                            selector=filled.css_selector,
                            value=filled.value,
                            note=filled.note,
                        )
                    case "closed":
                        self._closed = True
                        raise AllwrightError(
                            f"page session {self.session_id} closed while filling"
                        )
                    case "error":
                        raise AllwrightError(
                            f"page session error while filling: {event.error.message}"
                        )

    def press(
        self,
        selector: str,
        key: str,
        options: PressOptions | None = None,
    ) -> PressResult:
        with self._lock:
            handle = self._ensure_handle()
            self._ensure_open()
            press_options = options or PressOptions()
            handle.send(
                engine_pb2.TabSessionCommand(
                    browser_session_id=self.browser_session_id,
                    tab_session_id=self.session_id,
                    press_key=engine_pb2.PressKeyCommand(
                        css_selector=selector,
                        key=key,
                        text=press_options.text,
                        retry_options=_retry_options(press_options.timeout_ms),
                    ),
                )
            )

            while True:
                event = handle.recv("receive tab session event while pressing key")
                match event.WhichOneof("event"):
                    case "key_pressed":
                        pressed = event.key_pressed
                        return PressResult(
                            selector=pressed.css_selector,
                            key=pressed.key,
                            note=pressed.note,
                        )
                    case "closed":
                        self._closed = True
                        raise AllwrightError(
                            f"page session {self.session_id} closed while pressing key"
                        )
                    case "error":
                        raise AllwrightError(
                            f"page session error while pressing key: {event.error.message}"
                        )

    def text_content(self, selector: str, options: CommandOptions | None = None) -> TextResult:
        return self._read_text(selector, options or CommandOptions(), text_content=True)

    def inner_text(self, selector: str, options: CommandOptions | None = None) -> TextResult:
        return self._read_text(selector, options or CommandOptions(), text_content=False)

    def wait_for_selector(
        self,
        selector: str,
        options: WaitForSelectorOptions | None = None,
    ) -> WaitForSelectorResult:
        with self._lock:
            handle = self._ensure_handle()
            self._ensure_open()
            wait_options = options or WaitForSelectorOptions()
            handle.send(
                engine_pb2.TabSessionCommand(
                    browser_session_id=self.browser_session_id,
                    tab_session_id=self.session_id,
                    wait_for_selector=engine_pb2.WaitForSelectorCommand(
                        css_selector=selector,
                        visible=wait_options.visible,
                        retry_options=_retry_options(wait_options.timeout_ms),
                    ),
                )
            )

            while True:
                event = handle.recv("receive tab session event while waiting for selector")
                match event.WhichOneof("event"):
                    case "selector_wait_satisfied":
                        satisfied = event.selector_wait_satisfied
                        return WaitForSelectorResult(
                            selector=satisfied.css_selector,
                            visible=satisfied.visible,
                            note=satisfied.note,
                        )
                    case "closed":
                        self._closed = True
                        raise AllwrightError(
                            f"page session {self.session_id} closed while waiting for selector"
                        )
                    case "error":
                        raise AllwrightError(
                            f"page session error while waiting for selector: {event.error.message}"
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

    def _element_command(self, action: str, event_name: str, command: Any) -> ElementResult:
        with self._lock:
            handle = self._ensure_handle()
            self._ensure_open()
            handle.send(command)

            while True:
                event = handle.recv(f"receive tab session event while {action}")
                match event.WhichOneof("event"):
                    case name if name == event_name:
                        payload = getattr(event, event_name)
                        return ElementResult(selector=payload.css_selector, note=payload.note)
                    case "closed":
                        self._closed = True
                        raise AllwrightError(
                            f"page session {self.session_id} closed while {action}"
                        )
                    case "error":
                        raise AllwrightError(
                            f"page session error while {action}: {event.error.message}"
                        )

    def _read_text(
        self,
        selector: str,
        options: CommandOptions,
        *,
        text_content: bool,
    ) -> TextResult:
        with self._lock:
            handle = self._ensure_handle()
            self._ensure_open()
            if text_content:
                command = engine_pb2.TabSessionCommand(
                    browser_session_id=self.browser_session_id,
                    tab_session_id=self.session_id,
                    get_text_content=engine_pb2.GetTextContentCommand(
                        css_selector=selector,
                        retry_options=_retry_options(options.timeout_ms),
                    ),
                )
            else:
                command = engine_pb2.TabSessionCommand(
                    browser_session_id=self.browser_session_id,
                    tab_session_id=self.session_id,
                    get_inner_text=engine_pb2.GetInnerTextCommand(
                        css_selector=selector,
                        retry_options=_retry_options(options.timeout_ms),
                    ),
                )
            handle.send(command)

            while True:
                event = handle.recv("receive tab session event while reading text")
                match event.WhichOneof("event"):
                    case "text_content_resolved":
                        resolved = event.text_content_resolved
                        return TextResult(
                            selector=resolved.css_selector,
                            text=resolved.text,
                            note=resolved.note,
                        )
                    case "inner_text_resolved":
                        resolved = event.inner_text_resolved
                        return TextResult(
                            selector=resolved.css_selector,
                            text=resolved.text,
                            note=resolved.note,
                        )
                    case "closed":
                        self._closed = True
                        raise AllwrightError(
                            f"page session {self.session_id} closed while reading text"
                        )
                    case "error":
                        raise AllwrightError(
                            f"page session error while reading text: {event.error.message}"
                        )


@dataclass(slots=True)
class Locator:
    page: Page
    selector: str

    def locator(self, selector: str) -> Locator:
        return Locator(page=self.page, selector=f"{self.selector} {selector}".strip())

    def click(self, options: CommandOptions | None = None) -> ClickResult:
        return self.page.click(self.selector, options)

    def count(self, options: CommandOptions | None = None) -> CountResult:
        return self.page.count(self.selector, options)

    def highlight(self, options: HighlightOptions | None = None) -> HighlightResult:
        return self.page.highlight(self.selector, options)

    def focus(self, options: CommandOptions | None = None) -> ElementResult:
        return self.page.focus(self.selector, options)

    def fill(self, value: str, options: CommandOptions | None = None) -> FillResult:
        return self.page.fill(self.selector, value, options)

    def hover(self, options: CommandOptions | None = None) -> ElementResult:
        return self.page.hover(self.selector, options)

    def press(self, key: str, options: PressOptions | None = None) -> PressResult:
        return self.page.press(self.selector, key, options)

    def text_content(self, options: CommandOptions | None = None) -> TextResult:
        return self.page.text_content(self.selector, options)

    def inner_text(self, options: CommandOptions | None = None) -> TextResult:
        return self.page.inner_text(self.selector, options)

    def wait_for(self, options: WaitForSelectorOptions | None = None) -> WaitForSelectorResult:
        return self.page.wait_for_selector(self.selector, options)


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
    return launch_browser(engine_pb2.BROWSER_KIND_CHROMIUM, options)


def launch_firefox(options: LaunchOptions | None = None) -> Browser:
    return launch_browser(engine_pb2.BROWSER_KIND_FIREFOX, options)


def launch_browser(browser_kind: int, options: LaunchOptions | None = None) -> Browser:
    runtime = _get_runtime()
    stream = _StreamHandle(runtime.stub.BrowserSession)
    launch_options = options or LaunchOptions()

    command_kwargs: dict[str, Any] = {}
    if launch_options.browser_binary:
        command_kwargs["browser_binary"] = launch_options.browser_binary
    if launch_options.timeout_ms is not None:
        command_kwargs["retry_options"] = _retry_options(launch_options.timeout_ms)

    stream.send(
        engine_pb2.BrowserSessionCommand(
            launch_browser=engine_pb2.LaunchBrowserCommand(
                browser_kind=browser_kind,
                **command_kwargs,
            ),
        )
    )

    while True:
        event = stream.recv("receive browser session event during launch")
        match event.WhichOneof("event"):
            case "browser_launched":
                launched = event.browser_launched
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
                    cdp_websocket_url="",
                    user_data_dir=launched.user_data_dir,
                    initial_page=initial_page,
                )
            case "error":
                raise AllwrightError(
                    f"browser session error during launch: {event.error.message}"
                )


def find_config_file(start_dir: str | Path | None = None) -> Path | None:
    current_dir = Path(start_dir or Path.cwd()).resolve()

    while True:
        for filename in CONFIG_FILENAMES:
            candidate = current_dir / filename
            if candidate.is_file():
                return candidate

        if current_dir.parent == current_dir:
            return None
        current_dir = current_dir.parent


def load_config_file(config_file: str | Path) -> AllwrightConfig:
    resolved = Path(config_file).resolve()
    raw = resolved.read_text(encoding="utf-8")

    if resolved.suffix.lower() == ".json":
        parsed = json.loads(raw)
    elif resolved.suffix.lower() in {".yaml", ".yml"}:
        parsed = yaml.safe_load(raw) or {}
    else:
        raise AllwrightError(
            f"unsupported allwright config file extension {resolved.suffix or '<none>'} for {resolved}"
        )

    if not isinstance(parsed, dict):
        raise AllwrightError(f"allwright config {resolved} must contain a top-level object")

    _validate_config_shape(parsed, resolved)
    return AllwrightConfig(
        schema_version=parsed.get("schemaVersion"),
        server=parsed.get("server"),
        browser=parsed.get("browser"),
        expect=_retry_config_from_mapping(parsed.get("expect")),
        suites=parsed.get("suites"),
    )


def resolve_config(options: ResolveConfigOptions | None = None) -> ResolvedConfig:
    resolved_options = options or ResolveConfigOptions()
    config_file_path = (
        Path(resolved_options.config_file).resolve()
        if resolved_options.config_file
        else find_config_file(resolved_options.cwd)
    )
    file_config = load_config_file(config_file_path) if config_file_path else AllwrightConfig()
    suite_name = (resolved_options.suite or "").strip() or None
    suite_config = None

    if suite_name:
        suite_config = (file_config.suites or {}).get(suite_name)
        if suite_config is None:
            source = str(config_file_path) if config_file_path else "the resolved config file"
            raise AllwrightError(
                f'allwright config suite "{suite_name}" was not found in {source}'
            )

    server_addr = _first_non_empty(
        _server_addr_from_mapping(suite_config.get("server") if suite_config else None),
        _server_addr_from_mapping(file_config.server),
    )
    browser_name = _first_non_empty(
        _browser_name_from_mapping(suite_config.get("browser") if suite_config else None),
        _browser_name_from_mapping(file_config.browser),
    ) or "chromium"
    browser_binary = _first_non_empty(
        _browser_binary_from_mapping(suite_config.get("browser") if suite_config else None),
        _browser_binary_from_mapping(file_config.browser),
    )
    launch_options = _merge_launch_options(
        _launch_options_from_mapping(file_config.browser),
        _launch_options_from_mapping(suite_config.get("browser") if suite_config else None),
    )
    if browser_binary:
        launch_options.browser_binary = browser_binary
    expect = _merge_retry_config(
        file_config.expect,
        _retry_config_from_mapping(suite_config.get("expect") if suite_config else None),
    )

    return ResolvedConfig(
        config_file_path=config_file_path,
        suite_name=suite_name,
        server_addr=server_addr,
        browser_name=browser_name,
        browser_binary=browser_binary,
        launch_options=launch_options,
        expect=expect,
    )


def launch_configured_browser(config: ResolvedConfig) -> Browser:
    if config.server_addr:
        set_server_addr(config.server_addr)

    if config.browser_name == "firefox":
        return launch_firefox(config.launch_options)
    if config.browser_name == "chromium":
        return launch_chrome(config.launch_options)
    raise AllwrightError(
        f'unsupported browser.name "{config.browser_name}"; use "chromium" or "firefox"'
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


def _retry_options(timeout_ms: int | None) -> Any | None:
    if timeout_ms is None or timeout_ms <= 0:
        return None
    return engine_pb2.CommandRetryOptions(timeout_ms=timeout_ms)


def _validate_config_shape(config: dict[str, Any], source: Path) -> None:
    schema_version = config.get("schemaVersion")
    if schema_version is not None and schema_version != 1:
        raise AllwrightError(
            f"allwright config {source} has unsupported schemaVersion {schema_version}; expected 1"
        )

    browser_name = _browser_name_from_mapping(config.get("browser"))
    if browser_name and browser_name not in {"chromium", "firefox"}:
        raise AllwrightError(
            f'allwright config {source} has unsupported browser.name "{browser_name}"; use "chromium" or "firefox"'
        )


def _browser_name_from_mapping(browser: dict[str, Any] | None) -> str | None:
    if not isinstance(browser, dict):
        return None
    value = browser.get("name")
    return value.strip() if isinstance(value, str) and value.strip() else None


def _browser_binary_from_mapping(browser: dict[str, Any] | None) -> str | None:
    if not isinstance(browser, dict):
        return None
    value = browser.get("binary")
    return value.strip() if isinstance(value, str) and value.strip() else None


def _server_addr_from_mapping(server: dict[str, Any] | None) -> str | None:
    if not isinstance(server, dict):
        return None
    value = server.get("addr")
    return value.strip() if isinstance(value, str) and value.strip() else None


def _launch_options_from_mapping(browser: dict[str, Any] | None) -> LaunchOptions:
    if not isinstance(browser, dict):
        return LaunchOptions()
    launch_options = browser.get("launchOptions")
    if not isinstance(launch_options, dict):
        return LaunchOptions()
    return LaunchOptions(
        browser_binary=launch_options.get("browserBinary"),
        timeout_ms=launch_options.get("timeoutMs"),
    )


def _retry_config_from_mapping(value: dict[str, Any] | None) -> RetryConfig | None:
    if not isinstance(value, dict):
        return None
    return RetryConfig(
        timeout_ms=value.get("timeoutMs"),
        interval_ms=value.get("intervalMs"),
    )


def _merge_launch_options(base: LaunchOptions, override: LaunchOptions) -> LaunchOptions:
    return LaunchOptions(
        browser_binary=override.browser_binary or base.browser_binary,
        timeout_ms=override.timeout_ms if override.timeout_ms is not None else base.timeout_ms,
    )


def _merge_retry_config(base: RetryConfig | None, override: RetryConfig | None) -> RetryConfig:
    return RetryConfig(
        timeout_ms=(
            override.timeout_ms
            if override and override.timeout_ms is not None
            else (base.timeout_ms if base else None)
        ),
        interval_ms=(
            override.interval_ms
            if override and override.interval_ms is not None
            else (base.interval_ms if base else None)
        ),
    )


def _first_non_empty(*values: str | None) -> str | None:
    for value in values:
        if isinstance(value, str) and value.strip():
            return value.strip()
    return None


chromium = BrowserType(engine_pb2.BROWSER_KIND_CHROMIUM)
firefox = BrowserType(engine_pb2.BROWSER_KIND_FIREFOX)
