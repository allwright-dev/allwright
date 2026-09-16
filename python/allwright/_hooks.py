from __future__ import annotations

from dataclasses import dataclass
from typing import TYPE_CHECKING, Generic, TypeVar

if TYPE_CHECKING:
    from ._browser import Browser
    from ._types import CommandOptions

T = TypeVar("T")


@dataclass(frozen=True)
class HookType(Generic[T]):
    name: str


class Hook(Generic[T]):
    def __init__(self, browser: Browser, hook_id: str, hook_type: HookType[T]) -> None:
        self._browser = browser
        self.id = hook_id
        self.type = hook_type

    def wait(self, options: CommandOptions | None = None) -> T:
        return self._browser._wait_for_hook(self.id, self.type, options)


class _Hooks:
    def __init__(self) -> None:
        from ._page import Page

        self.new_page: HookType[Page] = HookType("new_page")


hooks = _Hooks()
