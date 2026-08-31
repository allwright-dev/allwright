from __future__ import annotations

from typing import overload

from ._browser import Browser, BrowserType
from ._config import find_config_file, load_config_file, resolve_config
from ._locator import Locator
from ._mobile import (
    AndroidLocator,
    AndroidDevice,
    AndroidApp,
    MobileAndroidConnectOptions,
    MobileAndroidLaunchOptions,
    mobile,
)
from ._page import Page
from ._proto import engine_pb2
from ._runtime import launch_browser_with_kind, ping, set_server_addr, shutdown
from ._types import (
    AllwrightConfig,
    ClickResult,
    CommandOptions,
    CountResult,
    ElementResult,
    FillResult,
    HighlightOptions,
    HighlightResult,
    LaunchOptions,
    NavigateResult,
    PressOptions,
    PressResult,
    ResolveConfigOptions,
    ResolvedConfig,
    ScreenshotResult,
    TextResult,
    WaitForSelectorOptions,
    WaitForSelectorResult,
)


def launch_chrome(options: LaunchOptions | None = None) -> Browser:
    return launch_browser(engine_pb2.BROWSER_KIND_CHROMIUM, options)


def launch_firefox(options: LaunchOptions | None = None) -> Browser:
    return launch_browser(engine_pb2.BROWSER_KIND_FIREFOX, options)


@overload
def launch_browser() -> Browser: ...


@overload
def launch_browser(options: ResolveConfigOptions) -> Browser: ...


@overload
def launch_browser(browser_kind: int, options: LaunchOptions | None = None) -> Browser: ...


def launch_browser(
    browser_kind_or_options: int | ResolveConfigOptions | None = None,
    options: LaunchOptions | None = None,
) -> Browser:
    if isinstance(browser_kind_or_options, ResolveConfigOptions) or browser_kind_or_options is None:
        return launch_configured_browser(resolve_config(browser_kind_or_options))
    return launch_browser_with_kind(browser_kind_or_options, options)


def launch_configured_browser(config: ResolvedConfig) -> Browser:
    if config.server_addr:
        set_server_addr(config.server_addr)

    if config.browser_name == "firefox":
        return launch_firefox(config.launch_options)
    if config.browser_name == "chromium":
        return launch_chrome(config.launch_options)
    if config.browser_name is None:
        raise ValueError("resolved config does not define web.browser.name and includes only non-web surfaces")
    raise ValueError(
        f'unsupported browser.name "{config.browser_name}"; use "chromium" or "firefox"'
    )


chromium = BrowserType(engine_pb2.BROWSER_KIND_CHROMIUM)
firefox = BrowserType(engine_pb2.BROWSER_KIND_FIREFOX)
