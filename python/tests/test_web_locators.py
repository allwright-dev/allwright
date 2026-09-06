import json
import re
import unittest
from allwright import Page
from allwright._selectors import normalize_selector_for_transport
from test_accessibility_snapshot import FakeStream
from allwright._proto import engine_pb2

class WebLocatorTest(unittest.TestCase):
    def test_regex_false_and_quoted_chains_reach_transport(self):
        page = Page(None, 'browser', 'page')
        inner = page.get_by_role('button', name=re.compile(r'^Save "now"$', re.I), pressed=False)
        locator = page.locator('article').filter(has=inner, has_not_text='xpath="trap"', visible=False).get_by_test_id('a"b').last
        self.assertEqual(normalize_selector_for_transport(locator.selector), locator.selector)
        self.assertIn('"kind"', json.loads(inner.selector[3:]))
        spec = json.loads(json.loads(inner.selector[3:]))
        self.assertFalse(spec['pressed'])
        self.assertEqual(spec['name'], {'regex': '^Save "now"$', 'flags': 'i'})
        stream = FakeStream(engine_pb2.ContextSessionEvent(element_counted=engine_pb2.ElementCountedEvent(count=2)))
        page._handle = stream
        self.assertEqual(locator.count().count, 2)
        self.assertEqual(stream.commands[0].count_elements.css_selector, locator.selector)

    def test_filter_rejects_another_page(self):
        page, other = Page(None, 'browser', 'one'), Page(None, 'browser', 'two')
        with self.assertRaisesRegex(ValueError, 'same page'):
            page.locator('li').filter(has=other.get_by_text('No'))
        with self.assertRaises(ValueError):
            page.get_by_role('heading', level=0)

    def test_exclusion_is_immutable_and_rejects_other_pages(self):
        page, other = Page(None, 'browser', 'one'), Page(None, 'browser', 'two')
        all_buttons = page.get_by_role('button')
        result = all_buttons.not_(page.get_by_text('Cancel')).first
        self.assertIn('exclude', result.selector)
        self.assertNotIn('exclude', all_buttons.selector)
        self.assertEqual(normalize_selector_for_transport(result.selector), result.selector)
        with self.assertRaisesRegex(ValueError, 'same page'):
            all_buttons.not_(other.get_by_text('Cancel'))
