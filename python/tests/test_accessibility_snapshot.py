import unittest

from allwright import AccessibilitySnapshotOptions, Page
from allwright._proto import engine_pb2
from allwright._types import AllwrightError


class FakeStream:
    def __init__(self, *events):
        self.events = iter(events)
        self.commands = []

    def send(self, command):
        self.commands.append(command)

    def recv(self, _description):
        return next(self.events)


class AccessibilitySnapshotTest(unittest.TestCase):
    def page(self, *events):
        page = Page(None, "browser", "page")
        stream = FakeStream(*events)
        page._handle = stream
        return page, stream

    def test_default_json_and_yaml_options_preserve_serialized_result(self):
        for options, format, text in [
            (None, "json", '{"version":1,"documents":[]}'),
            (AccessibilitySnapshotOptions(format="yaml", timeout_ms=500, mode="ai"), "yaml", '"name": "yes"\n'),
        ]:
            with self.subTest(format=format):
                page, stream = self.page(engine_pb2.ContextSessionEvent(
                    accessibility_snapshot_captured=engine_pb2.AccessibilitySnapshotCapturedEvent(
                        snapshot=text, format=format,
                    ),
                ))
                self.assertEqual(page.accessibility_snapshot(options), text)
                command = stream.commands[0]
                self.assertEqual(command.surface_session_id, "browser")
                self.assertEqual(command.context_session_id, "page")
                self.assertEqual(command.WhichOneof("command"), "accessibility_snapshot")
                self.assertEqual(command.accessibility_snapshot.format, format)
                self.assertEqual(command.accessibility_snapshot.mode, options.mode if options else "default")
                if options:
                    self.assertEqual(command.accessibility_snapshot.retry_options.timeout_ms, 500)

    def test_invalid_format_does_not_send(self):
        page, stream = self.page()
        with self.assertRaisesRegex(ValueError, "format"):
            page.accessibility_snapshot(AccessibilitySnapshotOptions(format="xml"))
        self.assertFalse(stream.commands)

    def test_invalid_mode_does_not_send(self):
        page, stream = self.page()
        with self.assertRaisesRegex(ValueError, "mode"):
            page.accessibility_snapshot(AccessibilitySnapshotOptions(mode="invalid"))
        self.assertFalse(stream.commands)

    def test_closed_and_error_events_are_reported(self):
        page, _ = self.page(engine_pb2.ContextSessionEvent(
            closed=engine_pb2.ContextSessionClosedEvent(reason="closed"),
        ))
        with self.assertRaisesRegex(AllwrightError, "closed"):
            page.accessibility_snapshot()
        self.assertTrue(page._closed)
        page, _ = self.page(engine_pb2.ContextSessionEvent(
            error=engine_pb2.ContextSessionErrorEvent(message="frame detached"),
        ))
        with self.assertRaisesRegex(AllwrightError, "frame detached"):
            page.accessibility_snapshot()


if __name__ == "__main__":
    unittest.main()
