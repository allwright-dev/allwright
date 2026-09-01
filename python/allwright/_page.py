from __future__ import annotations

import threading
from pathlib import Path
from typing import Any

from ._locator import Locator
from ._proto import engine_pb2
from ._selectors import normalize_selector_for_transport
from ._transport import RuntimeClient, StreamHandle
from ._types import (
    AllwrightError,
    ClickResult,
    CommandOptions,
    CountResult,
    ElementResult,
    FillResult,
    HighlightOptions,
    HighlightResult,
    NavigateResult,
    PressOptions,
    PressResult,
    ScreenshotOptions,
    ScreenshotResult,
    TextResult,
    WaitForSelectorOptions,
    WaitForSelectorResult,
)


class Page:
    def __init__(self, runtime: RuntimeClient, surface_session_id: str, session_id: str) -> None:
        self._runtime = runtime
        self._surface_session_id = surface_session_id
        self._session_id = session_id
        self._lock = threading.Lock()
        self._handle: StreamHandle | None = None
        self._closed = False

    @property
    def session_id(self) -> str:
        return self._session_id

    @property
    def surface_session_id(self) -> str:
        return self._surface_session_id

    def locator(self, selector: str) -> Locator:
        return Locator(page=self, selector=normalize_selector_for_transport(selector))

    def goto(self, url: str, options: CommandOptions | None = None) -> NavigateResult:
        from ._runtime import retry_options

        with self._lock:
            handle = self._ensure_handle()
            self._ensure_open()
            command_options = options or CommandOptions()
            handle.send(
                engine_pb2.ContextSessionCommand(
                    surface_session_id=self.surface_session_id,
                    context_session_id=self.session_id,
                    navigate=engine_pb2.NavigatePageCommand(
                        url=url,
                        retry_options=retry_options(command_options.timeout_ms),
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
        from ._runtime import retry_options

        with self._lock:
            handle = self._ensure_handle()
            self._ensure_open()
            command_options = options or CommandOptions()
            transport_selector = normalize_selector_for_transport(selector)
            handle.send(
                engine_pb2.ContextSessionCommand(
                    surface_session_id=self.surface_session_id,
                    context_session_id=self.session_id,
                    click_element=engine_pb2.ClickElementCommand(
                        css_selector=transport_selector,
                        retry_options=retry_options(command_options.timeout_ms),
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
        from ._runtime import retry_options

        with self._lock:
            handle = self._ensure_handle()
            self._ensure_open()
            command_options = options or CommandOptions()
            transport_selector = normalize_selector_for_transport(selector)
            handle.send(
                engine_pb2.ContextSessionCommand(
                    surface_session_id=self.surface_session_id,
                    context_session_id=self.session_id,
                    count_elements=engine_pb2.CountElementsCommand(
                        css_selector=transport_selector,
                        retry_options=retry_options(command_options.timeout_ms),
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
        from ._runtime import retry_options

        with self._lock:
            handle = self._ensure_handle()
            self._ensure_open()
            highlight_options = options or HighlightOptions()
            transport_selector = normalize_selector_for_transport(selector)
            handle.send(
                engine_pb2.ContextSessionCommand(
                    surface_session_id=self.surface_session_id,
                    context_session_id=self.session_id,
                    highlight_elements=engine_pb2.HighlightElementsCommand(
                        css_selector=transport_selector,
                        duration_ms=highlight_options.duration_ms,
                        retry_options=retry_options(highlight_options.timeout_ms),
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
        from ._runtime import retry_options

        transport_selector = normalize_selector_for_transport(selector)
        return self._element_command(
            action="focusing",
            event_name="element_focused",
            command=engine_pb2.ContextSessionCommand(
                surface_session_id=self.surface_session_id,
                context_session_id=self.session_id,
                focus_element=engine_pb2.FocusElementCommand(
                    css_selector=transport_selector,
                    retry_options=retry_options((options or CommandOptions()).timeout_ms),
                ),
            ),
        )

    def hover(self, selector: str, options: CommandOptions | None = None) -> ElementResult:
        from ._runtime import retry_options

        transport_selector = normalize_selector_for_transport(selector)
        return self._element_command(
            action="hovering",
            event_name="element_hovered",
            command=engine_pb2.ContextSessionCommand(
                surface_session_id=self.surface_session_id,
                context_session_id=self.session_id,
                hover_element=engine_pb2.HoverElementCommand(
                    css_selector=transport_selector,
                    retry_options=retry_options((options or CommandOptions()).timeout_ms),
                ),
            ),
        )

    def fill(
        self,
        selector: str,
        value: str,
        options: CommandOptions | None = None,
    ) -> FillResult:
        from ._runtime import retry_options

        with self._lock:
            handle = self._ensure_handle()
            self._ensure_open()
            command_options = options or CommandOptions()
            transport_selector = normalize_selector_for_transport(selector)
            handle.send(
                engine_pb2.ContextSessionCommand(
                    surface_session_id=self.surface_session_id,
                    context_session_id=self.session_id,
                    fill_element=engine_pb2.FillElementCommand(
                        css_selector=transport_selector,
                        value=value,
                        retry_options=retry_options(command_options.timeout_ms),
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
        from ._runtime import retry_options

        with self._lock:
            handle = self._ensure_handle()
            self._ensure_open()
            press_options = options or PressOptions()
            transport_selector = normalize_selector_for_transport(selector)
            handle.send(
                engine_pb2.ContextSessionCommand(
                    surface_session_id=self.surface_session_id,
                    context_session_id=self.session_id,
                    press_key=engine_pb2.PressKeyCommand(
                        css_selector=transport_selector,
                        key=key,
                        text=press_options.text,
                        retry_options=retry_options(press_options.timeout_ms),
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
        from ._runtime import retry_options

        with self._lock:
            handle = self._ensure_handle()
            self._ensure_open()
            wait_options = options or WaitForSelectorOptions()
            transport_selector = normalize_selector_for_transport(selector)
            handle.send(
                engine_pb2.ContextSessionCommand(
                    surface_session_id=self.surface_session_id,
                    context_session_id=self.session_id,
                    wait_for_selector=engine_pb2.WaitForSelectorCommand(
                        css_selector=transport_selector,
                        visible=wait_options.visible,
                        retry_options=retry_options(wait_options.timeout_ms),
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

    def screenshot(self, options: ScreenshotOptions | None = None) -> ScreenshotResult:
        from ._runtime import retry_options

        with self._lock:
            handle = self._ensure_handle()
            self._ensure_open()
            command_options = options or ScreenshotOptions()
            handle.send(
                engine_pb2.ContextSessionCommand(
                    surface_session_id=self.surface_session_id,
                    context_session_id=self.session_id,
                    screenshot=engine_pb2.ScreenshotCommand(
                        retry_options=retry_options(command_options.timeout_ms),
                        full_page=command_options.full_page,
                    ),
                )
            )

            while True:
                event = handle.recv("receive tab session event while capturing screenshot")
                match event.WhichOneof("event"):
                    case "screenshot_captured":
                        captured = event.screenshot_captured
                        screenshot = ScreenshotResult(
                            png_data=captured.png_data,
                            note=captured.note,
                        )
                        if command_options.path is not None:
                            Path(command_options.path).write_bytes(screenshot.png_data)
                        return screenshot
                    case "closed":
                        self._closed = True
                        raise AllwrightError(
                            f"page session {self.session_id} closed while capturing screenshot"
                        )
                    case "error":
                        raise AllwrightError(
                            f"page session error while capturing screenshot: {event.error.message}"
                        )

    def ping(self, message: str = "ping") -> str:
        with self._lock:
            handle = self._ensure_handle()
            self._ensure_open()
            handle.send(
                engine_pb2.ContextSessionCommand(
                    surface_session_id=self.surface_session_id,
                    context_session_id=self.session_id,
                    ping=engine_pb2.ContextSessionPingCommand(message=message),
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
                engine_pb2.ContextSessionCommand(
                    surface_session_id=self.surface_session_id,
                    context_session_id=self.session_id,
                    close=engine_pb2.CloseContextSessionCommand(),
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

    def _ensure_handle(self) -> StreamHandle:
        if self._handle is None:
            self._handle = StreamHandle(self._runtime.stub.ContextSession)
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
        from ._runtime import retry_options

        with self._lock:
            handle = self._ensure_handle()
            self._ensure_open()
            transport_selector = normalize_selector_for_transport(selector)
            if text_content:
                command = engine_pb2.ContextSessionCommand(
                    surface_session_id=self.surface_session_id,
                    context_session_id=self.session_id,
                    get_text_content=engine_pb2.GetTextContentCommand(
                        css_selector=transport_selector,
                        retry_options=retry_options(options.timeout_ms),
                    ),
                )
            else:
                command = engine_pb2.ContextSessionCommand(
                    surface_session_id=self.surface_session_id,
                    context_session_id=self.session_id,
                    get_inner_text=engine_pb2.GetInnerTextCommand(
                        css_selector=transport_selector,
                        retry_options=retry_options(options.timeout_ms),
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
