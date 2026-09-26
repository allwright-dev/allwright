from __future__ import annotations

from dataclasses import dataclass
from typing import TYPE_CHECKING, Generic, Iterable, TypeVar

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


class Dialog:
    def __init__(self, page: Page, dialog_id: str, kind: str, message: str, default_value: str) -> None:
        self.page, self.id, self.type = page, dialog_id, kind
        self.message, self.default_value = message, default_value

    def accept(self, prompt_text: str | None = None, options: CommandOptions | None = None) -> None:
        self.page._handle_dialog(self.id, True, prompt_text, options)

    def dismiss(self, options: CommandOptions | None = None) -> None:
        self.page._handle_dialog(self.id, False, None, options)


class FileChooser:
    def __init__(self, page: Page, file_chooser_id: str, is_multiple: bool) -> None:
        self.page = page
        self.id = file_chooser_id
        self.is_multiple = is_multiple

    def set_files(
        self,
        files: str | Iterable[str],
        options: CommandOptions | None = None,
    ) -> None:
        paths = [files] if isinstance(files, str) else list(files)
        if not self.is_multiple and len(paths) > 1:
            raise ValueError("file chooser does not accept multiple files")
        self.page._set_file_chooser_files(self.id, paths, options)


class Download:
    def __init__(
        self,
        page: Page,
        download_id: str,
        url: str,
        suggested_filename: str,
    ) -> None:
        self.page = page
        self.id = download_id
        self.url = url
        self.suggested_filename = suggested_filename

    def save_as(
        self,
        path: str,
        options: CommandOptions | None = None,
    ) -> None:
        self.page._save_download(self.id, path, options)


class _Hooks:
    dialog: HookType[Dialog]
    new_page: HookType[Page]
    file_chooser: HookType[FileChooser]
    download: HookType[Download]

    def __init__(self) -> None:
        self.dialog = HookType("dialog")
        self.new_page = HookType("new_page")
        self.file_chooser = HookType("file_chooser")
        self.download = HookType("download")


hooks = _Hooks()
