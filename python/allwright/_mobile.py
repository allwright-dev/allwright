from __future__ import annotations

from dataclasses import dataclass
from typing import Any

from ._bootstrap import invoke_plugin
from ._mobile_selectors import (
    chain_mobile_selector_for_transport,
    normalize_mobile_selector_for_transport,
)
from ._types import AllwrightError, ClickResult, CommandOptions, FillResult


@dataclass(slots=True)
class MobileAndroidConnectOptions:
    device: str | None = None
    adb_endpoint: str | None = None
    preserve_app_state: bool = False
    timeout_ms: int | None = None


@dataclass(slots=True)
class MobileAndroidLaunchOptions:
    apk_path: str | None = None
    app_id: str | None = None
    launch_activity: str | None = None
    stop_before_launch: bool = False
    timeout_ms: int | None = None


def _unwrap_mobile_result(command_name: str, payload: dict[str, Any]) -> dict[str, Any]:
    if not payload.get("ok"):
        raise AllwrightError(
            str(payload.get("error") or f"mobile-android plugin {command_name} failed")
        )
    result = payload.get("result")
    if not isinstance(result, dict):
        raise AllwrightError(
            f"mobile-android plugin {command_name} returned no result"
        )
    return result


class AndroidPage:
    def __init__(self, browser_session: dict[str, Any], page_session: dict[str, Any]) -> None:
        self._browser_session = browser_session
        self._page_session = page_session

    @property
    def session_id(self) -> str:
        return str(self._page_session.get("page_id") or "")

    def locator(self, selector: str) -> AndroidLocator:
        return AndroidLocator(
            page=self,
            selector=normalize_mobile_selector_for_transport(selector),
        )

    def click(self, selector: str, options: CommandOptions | None = None) -> ClickResult:
        payload = invoke_plugin(
            "mobile-android",
            {
                "command": "click_element",
                "browser_session": self._browser_session,
                "page_session": self._page_session,
                "selector": normalize_mobile_selector_for_transport(selector),
                "timeout_ms": (options or CommandOptions()).timeout_ms,
            },
        )
        result = _unwrap_mobile_result("click", payload)
        return ClickResult(
            selector=str(result.get("selector") or ""),
            note=str(result.get("note") or ""),
            bidi_session_id=str(result.get("session_id") or ""),
        )

    def fill(
        self,
        selector: str,
        value: str,
        options: CommandOptions | None = None,
    ) -> FillResult:
        payload = invoke_plugin(
            "mobile-android",
            {
                "command": "fill_element",
                "browser_session": self._browser_session,
                "page_session": self._page_session,
                "selector": normalize_mobile_selector_for_transport(selector),
                "value": value,
                "timeout_ms": (options or CommandOptions()).timeout_ms,
            },
        )
        result = _unwrap_mobile_result("fill", payload)
        return FillResult(
            selector=str(result.get("selector") or ""),
            value=str(result.get("value") or ""),
            note=str(result.get("note") or ""),
        )


@dataclass(slots=True)
class AndroidLocator:
    page: AndroidPage
    selector: str

    def locator(self, selector: str) -> AndroidLocator:
        return AndroidLocator(
            page=self.page,
            selector=chain_mobile_selector_for_transport(self.selector, selector),
        )

    def click(self, options: CommandOptions | None = None) -> ClickResult:
        return self.page.click(self.selector, options)

    def fill(self, value: str, options: CommandOptions | None = None) -> FillResult:
        return self.page.fill(self.selector, value, options)


class AndroidDevice:
    def __init__(self, connect_info: dict[str, Any]) -> None:
        self._connect_info = connect_info
        self._page = AndroidPage(
            dict(connect_info.get("browser_session") or {}),
            dict((connect_info.get("initial_page") or {}).get("page_session") or {}),
        )

    @property
    def session_id(self) -> str:
        automation = dict((self._connect_info.get("browser_session") or {}).get("automation") or {})
        return str(automation.get("session_id") or "")

    def page(self) -> AndroidPage:
        return self._page

    def initial_page(self) -> AndroidPage:
        return self._page

    def launch(self, options: MobileAndroidLaunchOptions | None = None) -> AndroidPage:
        resolved = options or MobileAndroidLaunchOptions()
        payload = invoke_plugin(
            "mobile-android",
            {
                "command": "launch_app",
                "browser_session": self._connect_info.get("browser_session"),
                "options": {
                    "apk_path": resolved.apk_path,
                    "app_id": resolved.app_id,
                    "launch_activity": resolved.launch_activity,
                    "stop_before_launch": resolved.stop_before_launch,
                    "timeout_ms": resolved.timeout_ms,
                },
            },
        )
        result = _unwrap_mobile_result("launch", payload)
        self._page = AndroidPage(
            dict(self._connect_info.get("browser_session") or {}),
            dict(result.get("page_session") or {}),
        )
        return self._page


class AndroidSurface:
    def connect(self, options: MobileAndroidConnectOptions | None = None) -> AndroidDevice:
        resolved = options or MobileAndroidConnectOptions()
        payload = invoke_plugin(
            "mobile-android",
            {
                "command": "connect",
                "platform": "android",
                "device": resolved.device,
                "adb_endpoint": resolved.adb_endpoint,
                "preserve_app_state": resolved.preserve_app_state,
                "timeout_ms": resolved.timeout_ms,
            },
        )
        return AndroidDevice(_unwrap_mobile_result("connect", payload))


class MobileNamespace:
    def __init__(self) -> None:
        self.android = AndroidSurface()


mobile = MobileNamespace()
