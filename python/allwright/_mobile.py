from __future__ import annotations

import threading
from pathlib import Path

from ._proto import engine_pb2
from ._transport import RuntimeClient, StreamHandle
from ._types import (
    AllwrightError,
    ClickResult,
    CommandOptions,
    CountResult,
    ElementResult,
    FillResult,
    PressOptions,
    PressResult,
    ScreenshotOptions,
    ScreenshotResult,
    TextResult,
    WaitForSelectorOptions,
    WaitForSelectorResult,
)


class MobileAndroidConnectOptions:
    def __init__(
        self,
        device: str | None = None,
        adb_endpoint: str | None = None,
        preserve_app_state: bool = False,
        timeout_ms: int | None = None,
    ) -> None:
        self.device = device
        self.adb_endpoint = adb_endpoint
        self.preserve_app_state = preserve_app_state
        self.timeout_ms = timeout_ms


class MobileAndroidLaunchOptions:
    def __init__(
        self,
        apk_path: str | None = None,
        app_id: str | None = None,
        launch_activity: str | None = None,
        stop_before_launch: bool = False,
        timeout_ms: int | None = None,
    ) -> None:
        self.apk_path = apk_path
        self.app_id = app_id
        self.launch_activity = launch_activity
        self.stop_before_launch = stop_before_launch
        self.timeout_ms = timeout_ms


class AndroidApp:
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

    def locator(self, selector: str) -> AndroidLocator:
        return AndroidLocator(
            page=self,
            selector=normalize_mobile_selector_for_transport(selector),
        )

    def click(self, selector: str, options: CommandOptions | None = None) -> ClickResult:
        from ._runtime import retry_options

        with self._lock:
            handle = self._ensure_handle()
            self._ensure_open()
            transport_selector = normalize_mobile_selector_for_transport(selector)
            handle.send(
                engine_pb2.ContextSessionCommand(
                    surface_session_id=self._surface_session_id,
                    context_session_id=self._session_id,
                    click_element=engine_pb2.ClickElementCommand(
                        css_selector=transport_selector,
                        retry_options=retry_options((options or CommandOptions()).timeout_ms),
                    ),
                )
            )

            while True:
                event = handle.recv("receive tab session event while clicking Android element")
                match event.WhichOneof("event"):
                    case "attached":
                        pass
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
                            f"android app session {self._session_id} closed while clicking"
                        )
                    case "error":
                        raise AllwrightError(
                            f"android app session error while clicking: {event.error.message}"
                        )

    def count(self, selector: str, options: CommandOptions | None = None) -> CountResult:
        from ._runtime import retry_options

        with self._lock:
            handle = self._ensure_handle()
            self._ensure_open()
            transport_selector = normalize_mobile_selector_for_transport(selector)
            handle.send(
                engine_pb2.ContextSessionCommand(
                    surface_session_id=self._surface_session_id,
                    context_session_id=self._session_id,
                    count_elements=engine_pb2.CountElementsCommand(
                        css_selector=transport_selector,
                        retry_options=retry_options((options or CommandOptions()).timeout_ms),
                    ),
                )
            )

            while True:
                event = handle.recv("receive tab session event while counting Android elements")
                match event.WhichOneof("event"):
                    case "attached":
                        pass
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
                            f"android app session {self._session_id} closed while counting elements"
                        )
                    case "error":
                        raise AllwrightError(
                            f"android app session error while counting elements: {event.error.message}"
                        )

    def focus(self, selector: str, options: CommandOptions | None = None) -> ElementResult:
        from ._runtime import retry_options

        return self._element_command(
            action="focusing Android element",
            event_name="element_focused",
            command=engine_pb2.ContextSessionCommand(
                surface_session_id=self._surface_session_id,
                context_session_id=self._session_id,
                focus_element=engine_pb2.FocusElementCommand(
                    css_selector=normalize_mobile_selector_for_transport(selector),
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
            transport_selector = normalize_mobile_selector_for_transport(selector)
            handle.send(
                engine_pb2.ContextSessionCommand(
                    surface_session_id=self._surface_session_id,
                    context_session_id=self._session_id,
                    fill_element=engine_pb2.FillElementCommand(
                        css_selector=transport_selector,
                        value=value,
                        retry_options=retry_options((options or CommandOptions()).timeout_ms),
                    ),
                )
            )

            while True:
                event = handle.recv("receive tab session event while filling Android element")
                match event.WhichOneof("event"):
                    case "attached":
                        pass
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
                            f"android app session {self._session_id} closed while filling"
                        )
                    case "error":
                        raise AllwrightError(
                            f"android app session error while filling: {event.error.message}"
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
            resolved = options or PressOptions()
            handle.send(
                engine_pb2.ContextSessionCommand(
                    surface_session_id=self._surface_session_id,
                    context_session_id=self._session_id,
                    press_key=engine_pb2.PressKeyCommand(
                        css_selector=normalize_mobile_selector_for_transport(selector),
                        key=key,
                        text=resolved.text,
                        retry_options=retry_options(resolved.timeout_ms),
                    ),
                )
            )

            while True:
                event = handle.recv("receive tab session event while pressing Android key")
                match event.WhichOneof("event"):
                    case "attached":
                        pass
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
                            f"android app session {self._session_id} closed while pressing key"
                        )
                    case "error":
                        raise AllwrightError(
                            f"android app session error while pressing key: {event.error.message}"
                        )

    def text_content(
        self,
        selector: str,
        options: CommandOptions | None = None,
    ) -> TextResult:
        return self._read_text(
            selector,
            options or CommandOptions(),
            text_content=True,
        )

    def inner_text(
        self,
        selector: str,
        options: CommandOptions | None = None,
    ) -> TextResult:
        return self._read_text(
            selector,
            options or CommandOptions(),
            text_content=False,
        )

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
            transport_selector = normalize_mobile_selector_for_transport(selector)
            handle.send(
                engine_pb2.ContextSessionCommand(
                    surface_session_id=self._surface_session_id,
                    context_session_id=self._session_id,
                    wait_for_selector=engine_pb2.WaitForSelectorCommand(
                        css_selector=transport_selector,
                        visible=wait_options.visible,
                        retry_options=retry_options(wait_options.timeout_ms),
                    ),
                )
            )

            while True:
                event = handle.recv("receive tab session event while waiting for Android selector")
                match event.WhichOneof("event"):
                    case "attached":
                        pass
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
                            f"android app session {self._session_id} closed while waiting for selector"
                        )
                    case "error":
                        raise AllwrightError(
                            f"android app session error while waiting for selector: {event.error.message}"
                        )

    def screenshot(self, options: ScreenshotOptions | None = None) -> ScreenshotResult:
        from ._runtime import retry_options

        with self._lock:
            handle = self._ensure_handle()
            self._ensure_open()
            command_options = options or ScreenshotOptions()
            handle.send(
                engine_pb2.ContextSessionCommand(
                    surface_session_id=self._surface_session_id,
                    context_session_id=self._session_id,
                    screenshot=engine_pb2.ScreenshotCommand(
                        retry_options=retry_options(command_options.timeout_ms),
                        full_page=command_options.full_page,
                    ),
                )
            )

            while True:
                event = handle.recv("receive tab session event while capturing Android screenshot")
                match event.WhichOneof("event"):
                    case "attached":
                        pass
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
                            f"android app session {self._session_id} closed while capturing screenshot"
                        )
                    case "error":
                        raise AllwrightError(
                            f"android app session error while capturing screenshot: {event.error.message}"
                        )

    def _ensure_handle(self) -> StreamHandle:
        if self._handle is None:
            self._handle = StreamHandle(self._runtime.stub.ContextSession)
        return self._handle

    def _ensure_open(self) -> None:
        if self._closed:
            raise AllwrightError(f"android app session {self._session_id} is closed")


class AndroidLocator:
    def __init__(self, page: AndroidApp, selector: str) -> None:
        self.page = page
        self.selector = selector

    def locator(self, selector: str) -> AndroidLocator:
        return AndroidLocator(
            page=self.page,
            selector=chain_mobile_selector_for_transport(self.selector, selector),
        )

    def click(self, options: CommandOptions | None = None) -> ClickResult:
        return self.page.click(self.selector, options)

    def count(self, options: CommandOptions | None = None) -> CountResult:
        return self.page.count(self.selector, options)

    def focus(self, options: CommandOptions | None = None) -> ElementResult:
        return self.page.focus(self.selector, options)

    def fill(self, value: str, options: CommandOptions | None = None) -> FillResult:
        return self.page.fill(self.selector, value, options)

    def press(self, key: str, options: PressOptions | None = None) -> PressResult:
        return self.page.press(self.selector, key, options)

    def text_content(self, options: CommandOptions | None = None) -> TextResult:
        return self.page.text_content(self.selector, options)

    def inner_text(self, options: CommandOptions | None = None) -> TextResult:
        return self.page.inner_text(self.selector, options)

    def wait_for(self, options: WaitForSelectorOptions | None = None) -> WaitForSelectorResult:
        return self.page.wait_for_selector(self.selector, options)


class AndroidDevice:
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
        self._app = AndroidApp(runtime, surface_session_id, initial_app_session_id)

    @property
    def session_id(self) -> str:
        return self._session_id

    def app(self) -> AndroidApp:
        return self._app

    def initial_app(self) -> AndroidApp:
        return self._app

    def launch(self, options: MobileAndroidLaunchOptions | None = None) -> AndroidApp:
        from ._runtime import retry_options

        with self._lock:
            self._ensure_open()
            resolved = options or MobileAndroidLaunchOptions()
            self._stream.send(
                engine_pb2.SurfaceSessionCommand(
                    launch_app=engine_pb2.LaunchAppCommand(
                        apk_path=resolved.apk_path,
                        app_id=resolved.app_id,
                        launch_activity=resolved.launch_activity,
                        stop_before_launch=resolved.stop_before_launch,
                        retry_options=retry_options(resolved.timeout_ms),
                    )
                )
            )

            while True:
                event = self._stream.recv("receive browser session event while launching Android app")
                match event.WhichOneof("event"):
                    case "app_launched":
                        self._app = AndroidApp(
                            self._runtime,
                            self._surface_session_id,
                            event.app_launched.app_session_id,
                        )
                        return self._app
                    case "closed":
                        self._closed = True
                        raise AllwrightError(
                            f"android device session {self._session_id} closed while launching app"
                        )
                    case "error":
                        raise AllwrightError(
                            f"android device session error while launching app: {event.error.message}"
                        )

    def _ensure_open(self) -> None:
        if self._closed:
            raise AllwrightError(f"android device session {self._session_id} is closed")


class AndroidSurface:
    def connect(self, options: MobileAndroidConnectOptions | None = None) -> AndroidDevice:
        from ._runtime import get_runtime, retry_options

        runtime = get_runtime()
        stream = StreamHandle(runtime.stub.SurfaceSession)
        resolved = options or MobileAndroidConnectOptions()
        stream.send(
            engine_pb2.SurfaceSessionCommand(
                connect_mobile=engine_pb2.ConnectMobileCommand(
                    platform=engine_pb2.MOBILE_PLATFORM_ANDROID,
                    device=resolved.device,
                    adb_endpoint=resolved.adb_endpoint,
                    preserve_app_state=resolved.preserve_app_state,
                    retry_options=retry_options(resolved.timeout_ms),
                )
            )
        )

        while True:
            event = stream.recv("receive browser session event while connecting Android device")
            match event.WhichOneof("event"):
                case "mobile_connected":
                    connected = event.mobile_connected
                    session_id = connected.device_session_id or event.session_id
                    return AndroidDevice(
                        runtime=runtime,
                        stream=stream,
                        session_id=session_id,
                        surface_session_id=event.session_id,
                        initial_app_session_id=connected.initial_app_session_id,
                    )
                case "error":
                    raise AllwrightError(
                        f"device session error during Android connect: {event.error.message}"
                    )


class MobileNamespace:
    def __init__(self) -> None:
        self.android = AndroidSurface()


mobile = MobileNamespace()


class MobileSelectorFlavor:
    CSS = "css"
    XPATH = "xpath"
    UIA = "uia"


UIAUTOMATOR_SELECTOR_KEYS = {
    "text",
    "textcontains",
    "textmatches",
    "textstartswith",
    "classname",
    "classnamematches",
    "description",
    "desc",
    "descriptioncontains",
    "desccontains",
    "descriptionmatches",
    "descmatches",
    "descriptionstartswith",
    "descstartswith",
    "checkable",
    "checked",
    "clickable",
    "longclickable",
    "scrollable",
    "enabled",
    "focusable",
    "focused",
    "selected",
    "packagename",
    "package",
    "packagenamematches",
    "resourceid",
    "resourceidmatches",
    "index",
    "instance",
}


def parse_explicit_mobile_selector_prefix(selector: str) -> tuple[str, int] | None:
    lowered = selector.lower()
    if lowered.startswith("xpath=") or lowered.startswith("xpath:"):
        return (MobileSelectorFlavor.XPATH, 6)
    if lowered.startswith("uia=") or lowered.startswith("uia:"):
        return (MobileSelectorFlavor.UIA, 4)
    prefix_len = parse_ui_automator_selector_prefix(lowered)
    if prefix_len is not None:
        return (MobileSelectorFlavor.UIA, prefix_len)
    if lowered.startswith("text=") or lowered.startswith("text:"):
        return (MobileSelectorFlavor.UIA, 5)
    if lowered.startswith("id=") or lowered.startswith("id:"):
        return (MobileSelectorFlavor.CSS, 3)
    if lowered.startswith("css=") or lowered.startswith("css:"):
        return (MobileSelectorFlavor.CSS, 4)
    return None


def parse_ui_automator_selector_prefix(selector: str) -> int | None:
    for key in UIAUTOMATOR_SELECTOR_KEYS:
        if selector.startswith(key) and len(selector) > len(key) and selector[len(key)] in ("=", ":"):
            return len(key) + 1
    return None


def find_json_string_end(value: str) -> int | None:
    if not value.startswith('"'):
        return None
    escaped = False
    for index in range(1, len(value)):
        char = value[index]
        if escaped:
            escaped = False
            continue
        if char == "\\":
            escaped = True
            continue
        if char == '"':
            return index + 1
    return None


def is_normalized_mobile_transport_selector(selector: str) -> bool:
    trimmed = selector.strip()
    if not trimmed:
        return False

    index = 0
    while index < len(trimmed):
        parsed = parse_explicit_mobile_selector_prefix(trimmed[index:])
        if parsed is None:
            return False
        _, prefix_len = parsed
        index += prefix_len
        json_end = find_json_string_end(trimmed[index:])
        if json_end is None:
            return False
        index += json_end
        if index == len(trimmed):
            return True
        whitespace_start = index
        while index < len(trimmed) and trimmed[index].isspace():
            index += 1
        if index == whitespace_start:
            return False
        if parse_explicit_mobile_selector_prefix(trimmed[index:]) is None:
            return False
    return True


def decode_selector_body(body: str) -> str:
    candidate = body.strip()
    if len(candidate) >= 2 and candidate.startswith('"') and candidate.endswith('"'):
        import json

        try:
            return unescape_shell_escaped_selector(json.loads(candidate))
        except Exception:
            pass
    return unescape_shell_escaped_selector(candidate)


def unescape_shell_escaped_selector(value: str) -> str:
    result: list[str] = []
    index = 0
    while index < len(value):
        char = value[index]
        if char == "\\" and index + 1 < len(value) and value[index + 1] in "_ #:\\[]()\"'":
            result.append(value[index + 1])
            index += 2
            continue
        result.append(char)
        index += 1
    return "".join(result)


def parse_mobile_selector_for_transport(selector: str) -> tuple[str, str]:
    trimmed = selector.strip()
    parsed = parse_explicit_mobile_selector_prefix(trimmed)
    if parsed is not None:
        flavor, prefix_len = parsed
        body = decode_selector_body(trimmed[prefix_len:])
        if flavor == MobileSelectorFlavor.CSS and prefix_len == 3:
            return (flavor, body if body.startswith("#") else f"#{body}")
        if flavor == MobileSelectorFlavor.UIA and prefix_len == 5:
            return (flavor, f"text={body}")
        if flavor == MobileSelectorFlavor.UIA and prefix_len not in (4, 5):
            return (flavor, f"{trimmed[:prefix_len - 1]}={body}")
        return (flavor, body)
    if trimmed.startswith(("//", ".//", "../", "/", "(")):
        return (MobileSelectorFlavor.XPATH, trimmed)
    return (MobileSelectorFlavor.CSS, trimmed)


def normalize_mobile_selector_for_transport(selector: str) -> str:
    trimmed = selector.strip()
    if not trimmed:
        return ""
    if is_normalized_mobile_transport_selector(trimmed):
        return trimmed
    import json

    flavor, body = parse_mobile_selector_for_transport(selector)
    return f"{flavor}={json.dumps(body)}"


def chain_mobile_selector_for_transport(parent: str, child: str) -> str:
    normalized_parent = normalize_mobile_selector_for_transport(parent) if parent.strip() else ""
    normalized_child = normalize_mobile_selector_for_transport(child) if child.strip() else ""
    if not normalized_parent:
        return normalized_child
    if not normalized_child:
        return normalized_parent
    return f"{normalized_parent} {normalized_child}"
