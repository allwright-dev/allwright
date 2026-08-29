#!/usr/bin/env python3

from __future__ import annotations

import argparse
import json
import re
import sys
import time


def decode_selector_body(body: str) -> str:
    candidate = body.strip()
    if candidate.startswith('"') and candidate.endswith('"'):
        try:
            decoded = json.loads(candidate)
        except json.JSONDecodeError:
            return candidate
        if isinstance(decoded, str):
            return unescape_shell_escaped_selector(decoded)
    return unescape_shell_escaped_selector(candidate)


def unescape_shell_escaped_selector(value: str) -> str:
    result: list[str] = []
    index = 0
    while index < len(value):
        char = value[index]
        if char == "\\" and index + 1 < len(value):
            next_char = value[index + 1]
            if next_char in "_ #:[]()\"'":
                result.append(next_char)
                index += 2
                continue
        result.append(char)
        index += 1
    return "".join(result)


def parse_selector(selector: str) -> tuple[str, str]:
    trimmed = selector.strip()
    lowered = trimmed.lower()
    if lowered.startswith("xpath=") or lowered.startswith("xpath:"):
        return "xpath", decode_selector_body(trimmed[6:])
    if lowered.startswith("id=") or lowered.startswith("id:"):
        body = decode_selector_body(trimmed[3:])
        return "css", body if body.startswith("#") else f"#{body}"
    if lowered.startswith("css=") or lowered.startswith("css:"):
        return "css", decode_selector_body(trimmed[4:])
    return "xpath", trimmed


def quote_xpath_literal(value: str) -> str:
    if "'" not in value:
        return f"'{value}'"
    if '"' not in value:
        return f'"{value}"'
    parts = value.split("'")
    segments: list[str] = []
    for index, part in enumerate(parts):
        if part:
            segments.append(f"'{part}'")
        if index < len(parts) - 1:
            segments.append("\"'\"")
    return "concat(" + ", ".join(segments) + ")"


def css_to_xpath(selector: str) -> str:
    trimmed = selector.strip()
    if not trimmed:
        raise ValueError("empty css selector")

    attr_filters: list[str] = []
    for attr_name, _, attr_value in re.findall(
        r"\[(text|content-desc|resource-id)=(['\"])(.*?)\2\]",
        trimmed,
    ):
        attr_filters.append(f"@{attr_name}={quote_xpath_literal(attr_value)}")
    trimmed = re.sub(r"\[(text|content-desc|resource-id)=(['\"])(.*?)\2\]", "", trimmed).strip()

    class_filter = None
    resource_filter = None

    if trimmed.startswith("#"):
        resource_id = trimmed[1:]
        resource_literal = quote_xpath_literal(resource_id)
        if ":" in resource_id:
            resource_filter = f"@resource-id={resource_literal}"
        else:
            suffix_literal = quote_xpath_literal(f":id/{resource_id}")
            resource_filter = (
                f"@resource-id={resource_literal} or substring(@resource-id, "
                f"string-length(@resource-id) - string-length({suffix_literal}) + 1) = {suffix_literal}"
            )
        trimmed = ""
    elif trimmed.startswith("."):
        class_name = trimmed[1:]
        if class_name:
            class_filter = f"@class={quote_xpath_literal(class_name)}"
        trimmed = ""
    elif trimmed:
        class_filter = f"@class={quote_xpath_literal(trimmed)}"

    filters = []
    if class_filter:
        filters.append(class_filter)
    if resource_filter:
        filters.append(f"({resource_filter})")
    filters.extend(attr_filters)

    if not filters:
        raise ValueError(
            "unsupported css selector for Android first cut; use xpath=..., #resource-id, .class, class name, or [text='...']"
        )

    return "//*[" + " and ".join(filters) + "]"


def import_uiautomator2():
    try:
        import uiautomator2 as u2  # type: ignore
    except ImportError as exc:  # pragma: no cover - runtime only
        raise RuntimeError(
            "uiautomator2 is not installed in the selected Python environment. Run `pip install uiautomator2`."
        ) from exc
    return u2


def call_with_retry(action, retries: int = 3, delay: float = 1.0):
    last_error: Exception | None = None
    for attempt in range(retries):
        try:
            return action()
        except Exception as exc:
            last_error = exc
            if attempt + 1 < retries:
                time.sleep(delay)
    raise RuntimeError(str(last_error) if last_error else "uiautomator2 action failed")


def connect_device(serial: str):
    u2 = import_uiautomator2()
    def bootstrap():
        device = u2.connect(serial)
        healthcheck = getattr(device, "healthcheck", None)
        if callable(healthcheck):
            healthcheck = getattr(device, "healthcheck", None)
            if callable(healthcheck):
                try:
                    healthcheck()
                except Exception:
                    pass
        _ = device.info
        _ = device.app_current()
        return device

    return call_with_retry(bootstrap, retries=4, delay=1.0)


def cmd_connect(args: argparse.Namespace) -> int:
    device = connect_device(args.serial)
    info = dict(call_with_retry(lambda: device.info, retries=3, delay=0.5) or {})
    current = dict(call_with_retry(lambda: device.app_current(), retries=3, delay=0.5) or {})
    payload = {
        "serial": args.serial,
        "deviceName": info.get("productName"),
        "currentPackage": current.get("package"),
        "currentActivity": current.get("activity"),
    }
    print(json.dumps(payload), flush=True)
    return 0


def cmd_click(args: argparse.Namespace) -> int:
    device = connect_device(args.serial)
    selector_kind, selector_body = parse_selector(args.selector)
    resolved_selector, current, target = resolve_target(device, selector_kind, selector_body, args.timeout)

    def click_target():
        target.click()
        return True

    call_with_retry(click_target, retries=3, delay=1.0)
    payload = {
        "resolvedSelector": resolved_selector,
        "note": (
            f"clicked Android element via UiAutomator2 {selector_kind} "
            f"on {current.get('package') or '<unknown>'}/{current.get('activity') or '<unknown>'}"
        ),
    }
    print(json.dumps(payload), flush=True)
    return 0


def cmd_fill(args: argparse.Namespace) -> int:
    device = connect_device(args.serial)
    selector_kind, selector_body = parse_selector(args.selector)
    resolved_selector, current, target = resolve_target(device, selector_kind, selector_body, args.timeout)

    def fill_target():
        target.set_text(args.value)
        return True

    call_with_retry(fill_target, retries=3, delay=1.0)
    payload = {
        "resolvedSelector": resolved_selector,
        "value": args.value,
        "note": (
            f"filled Android element via UiAutomator2 {selector_kind} "
            f"on {current.get('package') or '<unknown>'}/{current.get('activity') or '<unknown>'}"
        ),
    }
    print(json.dumps(payload), flush=True)
    return 0


def cmd_source(args: argparse.Namespace) -> int:
    device = connect_device(args.serial)
    current = dict(call_with_retry(lambda: device.app_current(), retries=3, delay=0.5) or {})
    hierarchy = call_with_retry(
        lambda: device.dump_hierarchy(compressed=False),
        retries=3,
        delay=1.0,
    )
    payload = {
        "source": hierarchy,
        "currentPackage": current.get("package"),
        "currentActivity": current.get("activity"),
    }
    print(json.dumps(payload), flush=True)
    return 0


def resolve_target(device, selector_kind: str, selector_body: str, timeout: float):
    resolved_selector = selector_body if selector_kind == "xpath" else css_to_xpath(selector_body)
    current = dict(call_with_retry(lambda: device.app_current(), retries=3, delay=0.5) or {})
    target = device.xpath(resolved_selector)

    def wait_for_target() -> bool:
        return bool(target.wait(timeout=timeout))

    if not call_with_retry(wait_for_target, retries=3, delay=1.0):
        package_name = current.get("package") or "<unknown>"
        activity_name = current.get("activity") or "<unknown>"
        raise RuntimeError(
            f"selector not found within {timeout}s: {resolved_selector} "
            f"(current app: {package_name}/{activity_name})"
        )

    return resolved_selector, current, target


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Thin UiAutomator2 bridge for the allwright Android surface")
    subparsers = parser.add_subparsers(dest="command", required=True)

    connect = subparsers.add_parser("connect", help="Validate UiAutomator2 connectivity for a device")
    connect.add_argument("serial", help="ADB device serial")
    connect.set_defaults(handler=cmd_connect)

    click = subparsers.add_parser("click", help="Click an element using a normalized selector")
    click.add_argument("serial", help="ADB device serial")
    click.add_argument("selector", help="Normalized selector such as xpath=\"...\" or css=\"...\"")
    click.add_argument("--timeout", type=float, default=10.0, help="Wait timeout in seconds before clicking")
    click.set_defaults(handler=cmd_click)

    fill = subparsers.add_parser("fill", help="Fill an element using a normalized selector")
    fill.add_argument("serial", help="ADB device serial")
    fill.add_argument("selector", help="Normalized selector such as xpath=\"...\" or css=\"...\"")
    fill.add_argument("value", help="Text to enter")
    fill.add_argument("--timeout", type=float, default=10.0, help="Wait timeout in seconds before filling")
    fill.set_defaults(handler=cmd_fill)

    source = subparsers.add_parser("source", help="Dump the current UI hierarchy source")
    source.add_argument("serial", help="ADB device serial")
    source.set_defaults(handler=cmd_source)

    return parser


def main() -> int:
    parser = build_parser()
    args = parser.parse_args()
    try:
        return args.handler(args)
    except Exception as exc:  # pragma: no cover - runtime only
        print(str(exc), file=sys.stderr, flush=True)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
