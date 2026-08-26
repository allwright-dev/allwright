from __future__ import annotations

import json


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
