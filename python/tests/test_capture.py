import unittest

from allwright import BoundingBox, CapturedOption, CommandOptions, Page
from allwright._proto import engine_pb2
from allwright._types import AllwrightError
from test_accessibility_snapshot import FakeStream


class CaptureTest(unittest.TestCase):
    def page(self, **result):
        page = Page(None, 'browser', 'page')
        page._handle = FakeStream(engine_pb2.ContextSessionEvent(
            capture_resolved=engine_pb2.CaptureResolvedEvent(**result)))
        return page

    def test_attribute_absence_is_distinct_from_empty(self):
        self.assertIsNone(self.page().locator('a').get_attribute('href'))
        page = self.page(value='')
        self.assertEqual(page.locator('a').get_attribute('href', CommandOptions(timeout_ms=25)), '')
        command = page._handle.commands[0].capture
        self.assertEqual(command.kind, engine_pb2.CAPTURE_KIND_ATTRIBUTE)
        self.assertEqual(command.attribute_name, 'href')
        self.assertEqual(command.retry_options.timeout_ms, 25)

    def test_typed_results_and_false_state(self):
        self.assertFalse(self.page(checked=False).is_checked('input'))
        self.assertEqual(self.page(value='https://example.com/').url(), 'https://example.com/')
        self.assertEqual(self.page(value='changed').input_value('textarea'), 'changed')
        self.assertIsNone(self.page().selected_text('input[type=number]'))
        self.assertEqual(self.page(value='').selected_text('textarea'), '')
        self.assertEqual(self.page(selected_options=[{'value': 'a', 'label': 'Alpha', 'index': 2}]).selected_options('select'), [CapturedOption('a', 'Alpha', 2)])
        self.assertIsNone(self.page().bounding_box('div'))
        self.assertEqual(self.page(bounding_box={'x': -5.5, 'y': 2, 'width': 100, 'height': 20}).bounding_box('div'), BoundingBox(-5.5, 2, 100, 20))

    def test_closed_and_error_propagate(self):
        for event in [engine_pb2.ContextSessionEvent(error=engine_pb2.ContextSessionErrorEvent(message='detached')),
                      engine_pb2.ContextSessionEvent(closed=engine_pb2.ContextSessionClosedEvent())]:
            page = self.page()
            page._handle = FakeStream(event)
            with self.assertRaises(AllwrightError):
                page.url()
            if event.HasField('closed'):
                self.assertTrue(page._closed)
