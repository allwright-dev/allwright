import unittest

from allwright import ClickOptions, CommandOptions, Page
from allwright._proto import engine_pb2
from test_accessibility_snapshot import FakeStream


class PointerActionsTest(unittest.TestCase):
    def page(self) -> Page:
        page = Page(None, "browser", "page")
        page._handle = FakeStream(
            engine_pb2.ContextSessionEvent(
                element_clicked=engine_pb2.ElementClickedEvent()
            )
        )
        return page

    def test_click_options_and_double_click_are_sent(self):
        page = self.page()
        page.click(
            "#menu",
            ClickOptions(timeout_ms=250, button="right", click_count=3),
        )
        command = page._handle.commands[0].click_element
        self.assertEqual(command.button, "right")
        self.assertEqual(command.click_count, 3)
        self.assertEqual(command.retry_options.timeout_ms, 250)

        page = self.page()
        page.click("#legacy", CommandOptions(timeout_ms=125))
        command = page._handle.commands[0].click_element
        self.assertEqual(command.button, "left")
        self.assertEqual(command.click_count, 1)
        self.assertEqual(command.retry_options.timeout_ms, 125)

        page = self.page()
        page.locator("#row").dblclick(ClickOptions(button="middle"))
        command = page._handle.commands[0].click_element
        self.assertEqual(command.button, "middle")
        self.assertEqual(command.click_count, 2)


if __name__ == "__main__":
    unittest.main()
