from __future__ import annotations

import json

def parse_explicit_selector_prefix(selector: str) -> tuple[str, int] | None:
    lower = selector.lower()
    if lower.startswith("xpath=") or lower.startswith("xpath:"):
        return "xpath", 6
    if lower.startswith("css=") or lower.startswith("css:"):
        return "css", 4
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
    lowered = trimmed.lower()
    if lowered.startswith("xpath=") or lowered.startswith("xpath:"):
        return "xpath", decode_selector_body(trimmed[6:])
    if lowered.startswith("css=") or lowered.startswith("css:"):
        return "css", decode_selector_body(trimmed[4:])
    if (
        trimmed.startswith("//")
        or trimmed.startswith(".//")
        or trimmed.startswith("../")
        or trimmed.startswith("/")
        or trimmed.startswith("(")
    ):
        return "xpath", trimmed
    return "css", trimmed


def normalize_selector_for_transport(selector: str) -> str:
    trimmed = selector.strip()
    if is_normalized_transport_selector(trimmed):
        return trimmed
    flavor, body = parse_selector_for_transport(selector)
    return f"{flavor}={json.dumps(body)}"


def chain_selector_for_transport(parent: str, child: str) -> str:
    parent_selector = normalize_selector_for_transport(parent) if parent.strip() else ""
    child_selector = normalize_selector_for_transport(child) if child.strip() else ""
    if not parent_selector:
        return child_selector
    if not child_selector:
        return parent_selector
    return f"{parent_selector} {child_selector}"
