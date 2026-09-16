import unittest

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


if __name__ == "__main__":
    unittest.main()
