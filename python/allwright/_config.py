from __future__ import annotations

import json
from pathlib import Path
from typing import Any

import yaml

from ._types import (
    AllwrightConfig,
    AllwrightError,
    LaunchOptions,
    ResolveConfigOptions,
    ResolvedConfig,
    RetryConfig,
)

CONFIG_FILENAMES = (
    "allwright.config.yaml",
    "allwright.config.yml",
    "allwright.config.json",
    ".allwright/config.yaml",
    ".allwright/config.yml",
    ".allwright/config.json",
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

    validate_config_shape(parsed, resolved)
    return AllwrightConfig(
        schema_version=parsed.get("schemaVersion"),
        server=parsed.get("server"),
        web=parsed.get("web"),
        mobile=parsed.get("mobile"),
        desktop=parsed.get("desktop"),
        expect=retry_config_from_mapping(parsed.get("expect")),
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

    server_addr = first_non_empty(
        server_addr_from_mapping(suite_config.get("server") if suite_config else None),
        server_addr_from_mapping(file_config.server),
    )
    resolved_web = merge_surface_mapping(file_config.web, suite_config.get("web") if suite_config else None)
    resolved_mobile = merge_surface_mapping(file_config.mobile, suite_config.get("mobile") if suite_config else None)
    resolved_desktop = merge_surface_mapping(file_config.desktop, suite_config.get("desktop") if suite_config else None)
    browser_name = first_non_empty(
        browser_name_from_mapping(browser_mapping_from_web(suite_config.get("web") if suite_config else None)),
        browser_name_from_mapping(browser_mapping_from_web(file_config.web)),
    )
    browser_binary = first_non_empty(
        browser_binary_from_mapping(browser_mapping_from_web(suite_config.get("web") if suite_config else None)),
        browser_binary_from_mapping(browser_mapping_from_web(file_config.web)),
    )
    launch_options = merge_launch_options(
        launch_options_from_mapping(browser_mapping_from_web(file_config.web)),
        launch_options_from_mapping(browser_mapping_from_web(suite_config.get("web") if suite_config else None)),
    )
    if browser_binary:
        launch_options.browser_binary = browser_binary
    if not browser_name and not resolved_mobile and not resolved_desktop:
        browser_name = "chromium"
    expect = merge_retry_config(
        file_config.expect,
        retry_config_from_mapping(suite_config.get("expect") if suite_config else None),
    )

    return ResolvedConfig(
        config_file_path=config_file_path,
        suite_name=suite_name,
        server_addr=server_addr,
        browser_name=browser_name,
        browser_binary=browser_binary,
        launch_options=launch_options,
        expect=expect,
        web=resolved_web,
        mobile=resolved_mobile,
        desktop=resolved_desktop,
    )


def validate_config_shape(config: dict[str, Any], source: Path) -> None:
    schema_version = config.get("schemaVersion")
    if schema_version is not None and schema_version != 1:
        raise AllwrightError(
            f"allwright config {source} has unsupported schemaVersion {schema_version}; expected 1"
        )

    browser_name = browser_name_from_mapping(browser_mapping_from_web(config.get("web")))
    if browser_name and browser_name not in {"chromium", "firefox"}:
        raise AllwrightError(
            f'allwright config {source} has unsupported browser.name "{browser_name}"; use "chromium" or "firefox"'
        )


def browser_mapping_from_web(web: dict[str, Any] | None) -> dict[str, Any] | None:
    if not isinstance(web, dict):
        return None
    browser = web.get("browser")
    return browser if isinstance(browser, dict) else None


def browser_name_from_mapping(browser: dict[str, Any] | None) -> str | None:
    if not isinstance(browser, dict):
        return None
    value = browser.get("name")
    return value.strip() if isinstance(value, str) and value.strip() else None


def browser_binary_from_mapping(browser: dict[str, Any] | None) -> str | None:
    if not isinstance(browser, dict):
        return None
    value = browser.get("binary")
    return value.strip() if isinstance(value, str) and value.strip() else None


def server_addr_from_mapping(server: dict[str, Any] | None) -> str | None:
    if not isinstance(server, dict):
        return None
    value = server.get("addr")
    return value.strip() if isinstance(value, str) and value.strip() else None


def launch_options_from_mapping(browser: dict[str, Any] | None) -> LaunchOptions:
    if not isinstance(browser, dict):
        return LaunchOptions()
    launch_options = browser.get("launchOptions")
    if not isinstance(launch_options, dict):
        return LaunchOptions()
    return LaunchOptions(
        browser_binary=launch_options.get("browserBinary"),
        timeout_ms=launch_options.get("timeoutMs"),
    )


def retry_config_from_mapping(value: dict[str, Any] | None) -> RetryConfig | None:
    if not isinstance(value, dict):
        return None
    return RetryConfig(
        timeout_ms=value.get("timeoutMs"),
        interval_ms=value.get("intervalMs"),
    )


def merge_launch_options(base: LaunchOptions, override: LaunchOptions) -> LaunchOptions:
    return LaunchOptions(
        browser_binary=override.browser_binary or base.browser_binary,
        timeout_ms=override.timeout_ms if override.timeout_ms is not None else base.timeout_ms,
    )


def merge_surface_mapping(
    base: dict[str, Any] | None,
    override: dict[str, Any] | None,
) -> dict[str, Any] | None:
    if not isinstance(base, dict) and not isinstance(override, dict):
        return None
    merged: dict[str, Any] = {}
    for source in (base, override):
        if isinstance(source, dict):
            for key, value in source.items():
                if isinstance(value, dict) and isinstance(merged.get(key), dict):
                    merged[key] = {**merged[key], **value}
                else:
                    merged[key] = value
    return merged


def merge_retry_config(base: RetryConfig | None, override: RetryConfig | None) -> RetryConfig:
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


def first_non_empty(*values: str | None) -> str | None:
    for value in values:
        if isinstance(value, str) and value.strip():
            return value.strip()
    return None
