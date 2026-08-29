from __future__ import annotations

import json

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


def parse_explicit_selector_prefix(selector: str) -> tuple[str, int] | None:
    lower = selector.lower()
    if lower.startswith("xpath=") or lower.startswith("xpath:"):
        return "xpath", 6
    if lower.startswith("css=") or lower.startswith("css:"):
        return "css", 4
    if lower.startswith("uia=") or lower.startswith("uia:"):
        return "uia", 4
    return None


def parse_uiautomator_selector_prefix(selector: str) -> int | None:
    for index, char in enumerate(selector):
        if char not in "=:":
            continue
        key = selector[:index].strip().lower()
        if key in UIAUTOMATOR_SELECTOR_KEYS:
            return index + 1
        return None
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


def is_normalized_transport_selector(selector: str) -> bool:
    trimmed = selector.strip()
    if not trimmed:
        return False

    index = 0
    while index < len(trimmed):
        prefix = parse_explicit_selector_prefix(trimmed[index:])
        if prefix is None:
            return False
        _, prefix_len = prefix
        index += prefix_len

        remainder = trimmed[index:]
        json_end = find_json_string_end(remainder)
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
        if parse_explicit_selector_prefix(trimmed[index:]) is None:
            return False

    return True


def decode_selector_body(body: str) -> str:
    candidate = body.strip()
    if len(candidate) >= 2 and candidate[0] == '"' and candidate[-1] == '"':
        try:
            decoded = json.loads(candidate)
        except json.JSONDecodeError:
            return candidate
        if isinstance(decoded, str):
            return decoded
    return candidate


def parse_selector_for_transport(selector: str) -> tuple[str, str]:
    trimmed = selector.strip()
    explicit = parse_explicit_selector_prefix(trimmed)
    if explicit is not None:
        flavor, prefix_len = explicit
        return flavor, decode_selector_body(trimmed[prefix_len:])
    uia_prefix_len = parse_uiautomator_selector_prefix(trimmed)
    if uia_prefix_len is not None:
        return "uia", f"{trimmed[:uia_prefix_len - 1]}={trimmed[uia_prefix_len:]}"
    if (
        trimmed.startswith("//")
        or trimmed.startswith(".//")
        or trimmed.startswith("../")
        or trimmed.startswith("/")
        or trimmed.startswith("(")
    ):
        return "xpath", trimmed
    return "css", trimmed


def normalize_mobile_selector_for_transport(selector: str) -> str:
    trimmed = selector.strip()
    if not trimmed:
        return ""
    if is_normalized_transport_selector(trimmed):
        return trimmed
    flavor, body = parse_selector_for_transport(selector)
    return f"{flavor}={json.dumps(body)}"


def chain_mobile_selector_for_transport(parent: str, child: str) -> str:
    parent_selector = normalize_mobile_selector_for_transport(parent) if parent.strip() else ""
    child_selector = normalize_mobile_selector_for_transport(child) if child.strip() else ""
    if not parent_selector:
        return child_selector
    if not child_selector:
        return parent_selector
    return f"{parent_selector} {child_selector}"
