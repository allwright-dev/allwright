from __future__ import annotations

from ._web_locators import WebLocators, TextMatcher, semantic_selector

from dataclasses import dataclass
from typing import TYPE_CHECKING

from ._selectors import chain_selector_for_transport
from ._types import (
    CapturedOption, BoundingBox,
    ClickResult,
    CommandOptions,
    CountResult,
    ElementResult,
    FillResult,
    HighlightOptions,
    HighlightResult,
    PressOptions,
    PressResult,
    TextResult,
    WaitForSelectorOptions,
    WaitForSelectorResult,
)

if TYPE_CHECKING:
    from ._page import Page


@dataclass(slots=True)
class Locator(WebLocators):
    page: Page
    selector: str

    def frame(self, options: CommandOptions | None = None) -> Page:
        return self.page.frame(self.selector, options)

    def locator(self, selector: str) -> Locator:
        return Locator(page=self.page, selector=chain_selector_for_transport(self.selector, selector))

    def not_(self, other: Locator) -> Locator:
        if other.page is not self.page:
            raise ValueError('Excluded locators must belong to the same page')
        return self.locator(semantic_selector(dict(kind='exclude', selector=other.selector)))

    def filter(self, *, has: Locator | None = None, has_not: Locator | None = None,
               has_text: TextMatcher | None = None, has_not_text: TextMatcher | None = None,
               visible: bool | None = None) -> Locator:
        for inner in (has, has_not):
            if inner is not None and inner.page is not self.page:
                raise ValueError('Filter locators must belong to the same page')
        spec = dict(kind='filter', has=has.selector if has else None,
                    hasNot=has_not.selector if has_not else None, hasText=has_text,
                    hasNotText=has_not_text, visible=visible)
        return self.locator(semantic_selector({k: v for k, v in spec.items() if v is not None}))

    def nth(self, index: int) -> Locator:
        if isinstance(index, bool) or not isinstance(index, int):
            raise ValueError('Locator index must be an integer')
        return self.locator(semantic_selector(dict(kind='nth', index=index)))

    @property
    def first(self) -> Locator:
        return self.nth(0)

    @property
    def last(self) -> Locator:
        return self.nth(-1)

    def click(self, options: CommandOptions | None = None) -> ClickResult:
        return self.page.click(self.selector, options)

    def count(self, options: CommandOptions | None = None) -> CountResult:
        return self.page.count(self.selector, options)

    def highlight(self, options: HighlightOptions | None = None) -> HighlightResult:
        return self.page.highlight(self.selector, options)

    def focus(self, options: CommandOptions | None = None) -> ElementResult:
        return self.page.focus(self.selector, options)

    def fill(self, value: str, options: CommandOptions | None = None) -> FillResult:
        return self.page.fill(self.selector, value, options)

    def hover(self, options: CommandOptions | None = None) -> ElementResult:
        return self.page.hover(self.selector, options)

    def press(self, key: str, options: PressOptions | None = None) -> PressResult:
        return self.page.press(self.selector, key, options)

    def input_value(self, options: CommandOptions | None = None) -> str:
        return self.page.input_value(self.selector, options)

    def selected_options(self, options: CommandOptions | None = None) -> list[CapturedOption]:
        return self.page.selected_options(self.selector, options)

    def selected_text(self, options: CommandOptions | None = None) -> str | None:
        return self.page.selected_text(self.selector, options)

    def is_checked(self, options: CommandOptions | None = None) -> bool:
        return self.page.is_checked(self.selector, options)

    def get_attribute(self, name: str, options: CommandOptions | None = None) -> str | None:
        return self.page.get_attribute(self.selector, name, options)

    def bounding_box(self, options: CommandOptions | None = None) -> BoundingBox | None:
        return self.page.bounding_box(self.selector, options)

    def text_content(self, options: CommandOptions | None = None) -> TextResult:
        return self.page.text_content(self.selector, options)

    def inner_text(self, options: CommandOptions | None = None) -> TextResult:
        return self.page.inner_text(self.selector, options)

    def wait_for(self, options: WaitForSelectorOptions | None = None) -> WaitForSelectorResult:
        return self.page.wait_for_selector(self.selector, options)
