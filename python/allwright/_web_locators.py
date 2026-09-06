"""Web locator construction; evaluation is owned by the web surface."""
from __future__ import annotations
import json
import re
from typing import TYPE_CHECKING
if TYPE_CHECKING:
    from ._locator import Locator

TextMatcher = str | re.Pattern[str]

def semantic_selector(spec: dict) -> str:
    def encode(value):
        if isinstance(value, re.Pattern):
            unsupported = value.flags & ~(re.I | re.M | re.S | re.U)
            if unsupported:
                raise ValueError('Unsupported locator regular expression flags')
            return {'regex': value.pattern, 'flags': ''.join(flag for mask, flag in [(re.I, 'i'), (re.M, 'm'), (re.S, 's')] if value.flags & mask)}
        raise TypeError(f'Unsupported selector value: {type(value)}')
    return 'aw=' + json.dumps(json.dumps(spec, default=encode))

class WebLocators:
    def locator(self, selector: str) -> Locator:
        raise NotImplementedError

    def get_by_role(self, role: str, *, name: TextMatcher | None = None, exact: bool = False,
                    checked: bool | None = None, disabled: bool | None = None,
                    expanded: bool | None = None, include_hidden: bool = False,
                    level: int | None = None, pressed: bool | None = None,
                    selected: bool | None = None) -> Locator:
        if level is not None and (isinstance(level, bool) or not isinstance(level, int) or level < 1):
            raise ValueError('Role level must be a positive integer')
        spec = dict(kind='role', role=role, name=name, exact=exact, checked=checked,
                    disabled=disabled, expanded=expanded, includeHidden=include_hidden,
                    level=level, pressed=pressed, selected=selected)
        return self.locator(semantic_selector({k: v for k, v in spec.items() if v is not None}))

    def get_by_text(self, text: TextMatcher, *, exact: bool = False) -> Locator:
        return self.locator(semantic_selector(dict(kind='text', text=text, exact=exact)))

    def get_by_label(self, text: TextMatcher, *, exact: bool = False) -> Locator:
        return self.locator(semantic_selector(dict(kind='label', text=text, exact=exact)))

    def get_by_placeholder(self, text: TextMatcher, *, exact: bool = False) -> Locator:
        return self.locator(semantic_selector(dict(kind='placeholder', text=text, exact=exact)))

    def get_by_alt_text(self, text: TextMatcher, *, exact: bool = False) -> Locator:
        return self.locator(semantic_selector(dict(kind='altText', text=text, exact=exact)))

    def get_by_title(self, text: TextMatcher, *, exact: bool = False) -> Locator:
        return self.locator(semantic_selector(dict(kind='title', text=text, exact=exact)))

    def get_by_test_id(self, text: TextMatcher) -> Locator:
        return self.locator(semantic_selector(dict(kind='testId', text=text)))
