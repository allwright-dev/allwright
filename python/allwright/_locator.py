from __future__ import annotations

from dataclasses import dataclass
from typing import TYPE_CHECKING

from ._selectors import chain_selector_for_transport
from ._types import (
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
class Locator:
    page: Page
    selector: str

    def locator(self, selector: str) -> Locator:
        return Locator(page=self.page, selector=chain_selector_for_transport(self.selector, selector))

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

    def text_content(self, options: CommandOptions | None = None) -> TextResult:
        return self.page.text_content(self.selector, options)

    def inner_text(self, options: CommandOptions | None = None) -> TextResult:
        return self.page.inner_text(self.selector, options)

    def wait_for(self, options: WaitForSelectorOptions | None = None) -> WaitForSelectorResult:
        return self.page.wait_for_selector(self.selector, options)
