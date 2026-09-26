import unittest

from allwright import CommandOptions, Dialog, Page, hooks
from allwright._proto import engine_pb2 as pb
from allwright._types import AllwrightError
from test_accessibility_snapshot import FakeStream


class DialogTest(unittest.TestCase):
    def test_register_wait_and_handle(self):
        for accept, text in [(True, None), (True, ''), (True, 'Ada'), (False, None)]:
            with self.subTest(accept=accept, text=text):
                page = Page(None, 'browser', 'page')
                page._handle = FakeStream(
                    pb.ContextSessionEvent(hook_registered=pb.HookRegisteredEvent(hook_id='hook')),
                    pb.ContextSessionEvent(hook_completed=pb.HookCompletedEvent(hook_id='hook', dialog=pb.DialogHookResult(
                        dialog_id='dialog', type='prompt', message='Name?', default_value='default'))),
                    pb.ContextSessionEvent(dialog_handled=pb.DialogHandledEvent(dialog_id='dialog')))
                options = CommandOptions(timeout_ms=75)
                dialog = page.register_hook(hooks.dialog).wait()
                self.assertIsInstance(dialog, Dialog)
                self.assertEqual((dialog.type, dialog.message, dialog.default_value), ('prompt', 'Name?', 'default'))
                if accept:
                    dialog.accept(text, options)
                else:
                    dialog.dismiss(options)
                commands = page._handle.commands
                self.assertEqual(commands[0].register_hook.WhichOneof('hook'), 'dialog')
                self.assertEqual(commands[2].handle_dialog.retry_options.timeout_ms, 75)
                self.assertEqual(commands[2].handle_dialog.accept, accept)
                self.assertEqual(commands[2].handle_dialog.HasField('prompt_text'), text is not None)
                if text is not None:
                    self.assertEqual(commands[2].handle_dialog.prompt_text, text)

    def test_error(self):
        page = Page(None, 'browser', 'page')
        page._handle = FakeStream(pb.ContextSessionEvent(error=pb.ContextSessionErrorEvent(message='already handled')))
        with self.assertRaisesRegex(AllwrightError, 'already handled'):
            Dialog(page, 'dialog', 'alert', 'Hello', '').dismiss()
