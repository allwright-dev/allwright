from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import Any


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
    web: dict[str, Any] | None = None
    mobile: dict[str, Any] | None = None
    desktop: dict[str, Any] | None = None
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
    browser_name: str | None
    browser_binary: str | None
    launch_options: LaunchOptions
    expect: RetryConfig
    web: dict[str, Any] | None = None
    mobile: dict[str, Any] | None = None
    desktop: dict[str, Any] | None = None


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


@dataclass(slots=True)
class ScreenshotResult:
    png_data: bytes
    note: str
