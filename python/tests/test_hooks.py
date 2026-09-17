import unittest
import tempfile
from pathlib import Path

from allwright import Browser, Page, hooks
from allwright._proto import engine_pb2


class FakeStream:
    def __init__(self, *events):
        self.events = iter(events)
        self.commands = []

    def send(self, command):
        self.commands.append(command)

    def recv(self, _description):
        return next(self.events)


class HookTest(unittest.TestCase):
    def test_generic_new_page_hook_registers_then_returns_managed_page(self):
        stream = FakeStream(
            engine_pb2.ContextSessionEvent(
                hook_registered=engine_pb2.HookRegisteredEvent(hook_id="hook-1")
            ),
            engine_pb2.ContextSessionEvent(
                hook_completed=engine_pb2.HookCompletedEvent(
                    hook_id="hook-1",
                    new_page=engine_pb2.NewPageHookResult(
                        context_session_id="page-2",
                        note="completed",
                    ),
                )
            ),
        )
        initial_page = Page(None, "browser-1", "page-1")
        initial_page._handle = stream
        browser = Browser(None, FakeStream(), "browser-1", "Chromium", "", "", "", initial_page)

        hook = initial_page.register_hook(hooks.new_page)
        page = hook.wait()

        self.assertEqual(page.session_id, "page-2")
        self.assertEqual(stream.commands[0].WhichOneof("command"), "register_hook")
        self.assertEqual(stream.commands[0].context_session_id, "page-1")
        self.assertEqual(stream.commands[0].register_hook.WhichOneof("hook"), "new_page")
        self.assertEqual(stream.commands[1].WhichOneof("command"), "wait_for_hook")
        self.assertEqual(stream.commands[1].wait_for_hook.hook_id, "hook-1")

    def test_file_chooser_hook_returns_typed_chooser_and_sets_files(self):
        stream = FakeStream(
            engine_pb2.ContextSessionEvent(
                hook_registered=engine_pb2.HookRegisteredEvent(hook_id="hook-2")
            ),
            engine_pb2.ContextSessionEvent(
                hook_completed=engine_pb2.HookCompletedEvent(
                    hook_id="hook-2",
                    file_chooser=engine_pb2.FileChooserHookResult(
                        file_chooser_id="chooser-1",
                        is_multiple=True,
                    ),
                )
            ),
            engine_pb2.ContextSessionEvent(
                file_uploaded=engine_pb2.FileUploadedEvent(
                    transfer_id="upload-one", file_id="file-1"
                )
            ),
            engine_pb2.ContextSessionEvent(
                file_uploaded=engine_pb2.FileUploadedEvent(
                    transfer_id="upload-two", file_id="file-2"
                )
            ),
            engine_pb2.ContextSessionEvent(
                file_chooser_files_set=engine_pb2.FileChooserFilesSetEvent(
                    file_chooser_id="chooser-1",
                    file_ids=["file-1", "file-2"],
                )
            ),
        )
        page = Page(None, "browser-1", "page-1")
        page._handle = stream

        with tempfile.TemporaryDirectory() as directory:
            one = Path(directory, "one.txt")
            two = Path(directory, "two.txt")
            one.write_text("one")
            two.write_text("two")
            hook = page.register_hook(hooks.file_chooser)
            chooser = hook.wait()
            # Match the fake transfer ids to the client-generated values.
            original_recv = stream.recv
            upload_index = 0
            def recv(description):
                nonlocal upload_index
                event = original_recv(description)
                if event.WhichOneof("event") == "file_uploaded":
                    event.file_uploaded.transfer_id = stream.commands[-1].upload_file_chunk.transfer_id
                    upload_index += 1
                return event
            stream.recv = recv
            chooser.set_files([str(one), str(two)])

        self.assertTrue(chooser.is_multiple)
        self.assertIs(chooser.page, page)
        self.assertEqual(stream.commands[0].register_hook.WhichOneof("hook"), "file_chooser")
        self.assertEqual(stream.commands[4].WhichOneof("command"), "set_file_chooser_files")
        self.assertEqual(
            list(stream.commands[4].set_file_chooser_files.file_ids),
            ["file-1", "file-2"],
        )

    def test_download_hook_returns_typed_download_and_saves_it(self):
        stream = FakeStream(
            engine_pb2.ContextSessionEvent(
                hook_registered=engine_pb2.HookRegisteredEvent(hook_id="hook-3")
            ),
            engine_pb2.ContextSessionEvent(
                hook_completed=engine_pb2.HookCompletedEvent(
                    hook_id="hook-3",
                    download=engine_pb2.DownloadHookResult(
                        download_id="download-1",
                        url="https://example.test/report.csv",
                        suggested_filename="report.csv",
                    ),
                )
            ),
            engine_pb2.ContextSessionEvent(
                download_saved=engine_pb2.DownloadSavedEvent(
                    download_id="download-1",
                    file_id="file-download-1",
                )
            ),
            engine_pb2.ContextSessionEvent(
                file_chunk=engine_pb2.FileChunkEvent(
                    file_id="file-download-1",
                    offset=0,
                    data=b"report contents",
                    last=True,
                )
            ),
        )
        page = Page(None, "browser-1", "page-1")
        page._handle = stream

        with tempfile.TemporaryDirectory() as directory:
            destination = Path(directory, "report.csv")
            hook = page.register_hook(hooks.download)
            download = hook.wait()
            download.save_as(str(destination))
            self.assertEqual(destination.read_text(), "report contents")

        self.assertIs(download.page, page)
        self.assertEqual(download.url, "https://example.test/report.csv")
        self.assertEqual(download.suggested_filename, "report.csv")
        self.assertEqual(stream.commands[0].register_hook.WhichOneof("hook"), "download")
        self.assertEqual(stream.commands[2].WhichOneof("command"), "save_download")
        self.assertEqual(stream.commands[2].save_download.download_id, "download-1")
        self.assertEqual(stream.commands[3].WhichOneof("command"), "read_file_chunk")


if __name__ == "__main__":
    unittest.main()
