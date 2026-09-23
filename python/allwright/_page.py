from __future__ import annotations

from ._web_locators import WebLocators, TextMatcher, semantic_selector

import threading
import os
import uuid
from pathlib import Path
from typing import Any, Callable, TypeVar

from ._locator import Locator
from ._hooks import Download, FileChooser, Hook, HookType
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
    AccessibilitySnapshotOptions,
    ScreenshotOptions,
    ScreenshotResult,
    TextResult,
    WaitForSelectorOptions,
    WaitForSelectorResult,
)


T = TypeVar("T")


class Page(WebLocators):
    def __init__(
        self,
        runtime: RuntimeClient,
        surface_session_id: str,
        session_id: str,
        page_factory: Callable[[str], Page] | None = None,
    ) -> None:
        self._runtime = runtime
        self._surface_session_id = surface_session_id
        self._session_id = session_id
        self._lock = threading.Lock()
        self._handle: StreamHandle | None = None
        self._closed = False
        self._page_factory = page_factory

    @property
    def session_id(self) -> str:
        return self._session_id

    @property
    def surface_session_id(self) -> str:
        return self._surface_session_id

    def locator(self, selector: str) -> Locator:
        return Locator(page=self, selector=normalize_selector_for_transport(selector))

    def frame(self, selector: str, options: CommandOptions | None = None) -> Page:
        from ._runtime import retry_options
        with self._lock:
            handle = self._ensure_handle()
            self._ensure_open()
            handle.send(engine_pb2.ContextSessionCommand(
                surface_session_id=self.surface_session_id, context_session_id=self.session_id,
                resolve_frame=engine_pb2.ResolveFrameCommand(
                    css_selector=normalize_selector_for_transport(selector),
                    retry_options=retry_options((options or CommandOptions()).timeout_ms))))
            while True:
                event = handle.recv("resolve frame")
                if event.WhichOneof("event") == "frame_resolved":
                    session_id = event.frame_resolved.context_session_id
                    return self._page_factory(session_id) if self._page_factory else Page(self._runtime, self.surface_session_id, session_id)
                if event.WhichOneof("event") == "error":
                    raise AllwrightError(event.error.message)
                if event.WhichOneof("event") == "closed":
                    self._closed = True
                    raise AllwrightError("page closed while resolving frame")

    def register_hook(self, hook_type: HookType[T]) -> Hook[T]:
        with self._lock:
            handle = self._ensure_handle()
            self._ensure_open()
            if hook_type.name not in {"new_page", "file_chooser", "download"}:
                raise AllwrightError(f"unsupported hook type: {hook_type.name}")
            if hook_type.name == "new_page":
                register_hook = engine_pb2.RegisterHookCommand(
                    new_page=engine_pb2.RegisterNewPageHook()
                )
            elif hook_type.name == "file_chooser":
                register_hook = engine_pb2.RegisterHookCommand(
                    file_chooser=engine_pb2.RegisterFileChooserHook()
                )
            else:
                register_hook = engine_pb2.RegisterHookCommand(
                    download=engine_pb2.RegisterDownloadHook()
                )
            handle.send(
                engine_pb2.ContextSessionCommand(
                    surface_session_id=self.surface_session_id,
                    context_session_id=self.session_id,
                    register_hook=register_hook,
                )
            )
            while True:
                event = handle.recv("receive page session event while registering hook")
                match event.WhichOneof("event"):
                    case "hook_registered":
                        return Hook(self, event.hook_registered.hook_id, hook_type)
                    case "error":
                        raise AllwrightError(
                            f"page session error while registering hook: {event.error.message}"
                        )

    def _wait_for_hook(
        self,
        hook_id: str,
        hook_type: HookType[T],
        options: CommandOptions | None = None,
    ) -> T:
        from ._runtime import retry_options

        with self._lock:
            handle = self._ensure_handle()
            self._ensure_open()
            command_options = options or CommandOptions()
            handle.send(
                engine_pb2.ContextSessionCommand(
                    surface_session_id=self.surface_session_id,
                    context_session_id=self.session_id,
                    wait_for_hook=engine_pb2.WaitForHookCommand(
                        hook_id=hook_id,
                        retry_options=retry_options(command_options.timeout_ms),
                    ),
                )
            )
            while True:
                event = handle.recv("receive page session event while waiting for hook")
                match event.WhichOneof("event"):
                    case "hook_completed":
                        completed = event.hook_completed
                        if completed.hook_id != hook_id:
                            continue
                        if hook_type.name == "new_page" and completed.WhichOneof("result") == "new_page":
                            session_id = completed.new_page.context_session_id
                            page = (
                                self._page_factory(session_id)
                                if self._page_factory is not None
                                else Page(self._runtime, self.surface_session_id, session_id)
                            )
                            return page  # type: ignore[return-value]
                        if hook_type.name == "file_chooser" and completed.WhichOneof("result") == "file_chooser":
                            return FileChooser(
                                self,
                                completed.file_chooser.file_chooser_id,
                                completed.file_chooser.is_multiple,
                            )  # type: ignore[return-value]
                        if hook_type.name == "download" and completed.WhichOneof("result") == "download":
                            return Download(
                                self,
                                completed.download.download_id,
                                completed.download.url,
                                completed.download.suggested_filename,
                            )  # type: ignore[return-value]
                        raise AllwrightError("hook completed with an invalid result")
                    case "error":
                        raise AllwrightError(
                            f"page session error while waiting for hook: {event.error.message}"
                        )

    def _set_file_chooser_files(
        self,
        file_chooser_id: str,
        files: list[str],
        options: CommandOptions | None = None,
    ) -> None:
        from ._runtime import retry_options

        with self._lock:
            handle = self._ensure_handle()
            self._ensure_open()
            command_options = options or CommandOptions()
            file_ids = [self._upload_local_file(handle, path) for path in files]
            handle.send(
                engine_pb2.ContextSessionCommand(
                    surface_session_id=self.surface_session_id,
                    context_session_id=self.session_id,
                    set_file_chooser_files=engine_pb2.SetFileChooserFilesCommand(
                        file_chooser_id=file_chooser_id,
                        file_ids=file_ids,
                        retry_options=retry_options(command_options.timeout_ms),
                    ),
                )
            )
            while True:
                event = handle.recv("receive page session event while setting chooser files")
                match event.WhichOneof("event"):
                    case "file_chooser_files_set":
                        if event.file_chooser_files_set.file_chooser_id == file_chooser_id:
                            return
                    case "error":
                        raise AllwrightError(
                            f"page session error while setting chooser files: {event.error.message}"
                        )

    def _save_download(
        self,
        download_id: str,
        path: str,
        options: CommandOptions | None = None,
    ) -> None:
        from ._runtime import retry_options

        with self._lock:
            handle = self._ensure_handle()
            self._ensure_open()
            command_options = options or CommandOptions()
            handle.send(
                engine_pb2.ContextSessionCommand(
                    surface_session_id=self.surface_session_id,
                    context_session_id=self.session_id,
                    save_download=engine_pb2.SaveDownloadCommand(
                        download_id=download_id,
                        retry_options=retry_options(command_options.timeout_ms),
                    ),
                )
            )
            while True:
                event = handle.recv("receive page session event while saving download")
                match event.WhichOneof("event"):
                    case "download_saved":
                        if event.download_saved.download_id == download_id:
                            self._download_to_local_file(
                                handle, event.download_saved.file_id, str(path)
                            )
                            return
                    case "error":
                        raise AllwrightError(
                            f"page session error while saving download: {event.error.message}"
                        )

    def _upload_local_file(self, handle: StreamHandle, path: str) -> str:
        transfer_id = str(uuid.uuid4())
        size = os.path.getsize(path)
        offset = 0
        with open(path, "rb") as source:
            while True:
                data = source.read(256 * 1024)
                last = offset + len(data) >= size
                handle.send(
                    engine_pb2.ContextSessionCommand(
                        surface_session_id=self.surface_session_id,
                        context_session_id=self.session_id,
                        upload_file_chunk=engine_pb2.UploadFileChunkCommand(
                            transfer_id=transfer_id,
                            name=os.path.basename(path),
                            offset=offset,
                            data=data,
                            last=last,
                        ),
                    )
                )
                offset += len(data)
                if last:
                    break
        while True:
            event = handle.recv("receive page session event while uploading file")
            match event.WhichOneof("event"):
                case "file_uploaded":
                    if event.file_uploaded.transfer_id == transfer_id:
                        return event.file_uploaded.file_id
                case "error":
                    raise AllwrightError(
                        f"page session error while uploading file: {event.error.message}"
                    )

    def _download_to_local_file(
        self, handle: StreamHandle, file_id: str, path: str
    ) -> None:
        temporary_path = f"{path}.allwright-{uuid.uuid4()}.tmp"
        offset = 0
        try:
            with open(temporary_path, "xb") as destination:
                while True:
                    handle.send(
                        engine_pb2.ContextSessionCommand(
                            surface_session_id=self.surface_session_id,
                            context_session_id=self.session_id,
                            read_file_chunk=engine_pb2.ReadFileChunkCommand(
                                file_id=file_id,
                                offset=offset,
                                max_bytes=256 * 1024,
                            ),
                        )
                    )
                    event = handle.recv("receive page session event while downloading file")
                    if event.WhichOneof("event") == "error":
                        raise AllwrightError(
                            f"page session error while downloading file: {event.error.message}"
                        )
                    if (
                        event.WhichOneof("event") != "file_chunk"
                        or event.file_chunk.file_id != file_id
                        or event.file_chunk.offset != offset
                    ):
                        continue
                    destination.write(event.file_chunk.data)
                    offset += len(event.file_chunk.data)
                    if event.file_chunk.last:
                        break
            os.replace(temporary_path, path)
        except Exception:
            try:
                os.unlink(temporary_path)
            except FileNotFoundError:
                pass
            raise

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

    def accessibility_snapshot(self, options: AccessibilitySnapshotOptions | None = None) -> str:
        from ._runtime import retry_options

        options = options or AccessibilitySnapshotOptions()
        if options.mode not in ("default", "ai", "autoexpect", "codegen"):
            raise ValueError("invalid accessibility snapshot mode")
        if options.format not in ("json", "yaml"):
            raise ValueError("accessibility snapshot format must be 'json' or 'yaml'")
        with self._lock:
            handle = self._ensure_handle()
            self._ensure_open()
            handle.send(engine_pb2.ContextSessionCommand(
                surface_session_id=self.surface_session_id,
                context_session_id=self.session_id,
                accessibility_snapshot=engine_pb2.AccessibilitySnapshotCommand(
                    format=options.format,
                    mode=options.mode,
                    retry_options=retry_options(options.timeout_ms),
                ),
            ))
            while True:
                event = handle.recv("receive accessibility snapshot")
                match event.WhichOneof("event"):
                    case "accessibility_snapshot_captured":
                        return event.accessibility_snapshot_captured.snapshot
                    case "closed":
                        self._closed = True
                        raise AllwrightError(f"page session {self.session_id} closed while capturing accessibility snapshot")
                    case "error":
                        raise AllwrightError(f"accessibility snapshot failed: {event.error.message}")

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
