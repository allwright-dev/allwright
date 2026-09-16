from __future__ import annotations

from dataclasses import dataclass
from typing import TYPE_CHECKING, Generic, TypeVar

if TYPE_CHECKING:
    from ._page import Page
    from ._types import CommandOptions

T = TypeVar("T")


@dataclass(frozen=True)
class HookType(Generic[T]):
    name: str


class Hook(Generic[T]):
    def __init__(self, page: Page, hook_id: str, hook_type: HookType[T]) -> None:
        self._page = page
        self.id = hook_id
        self.type = hook_type

    def wait(self, options: CommandOptions | None = None) -> T:
        return self._page._wait_for_hook(self.id, self.type, options)


class _Hooks:
    new_page: HookType[Page]

    def __init__(self) -> None:
        self.new_page = HookType("new_page")


hooks = _Hooks()
