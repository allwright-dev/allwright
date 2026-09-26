import { expect, test } from 'bun:test';
import { EventEmitter } from 'node:events';
import { hooks } from '../src/index.js';
import { PageImpl } from '../src/page.js';
import type { ContextSessionRequest, RuntimeClient } from '../src/types.js';

for (const timeoutMs of [undefined, 0, 75])
for (const action of ['accept', 'empty', 'input', 'dismiss', 'error'] as const) {
  test(`dialog hook transport: ${action}, timeout=${timeoutMs}`, async () => {
    const commands: ContextSessionRequest[] = [];
    const stream = Object.assign(new EventEmitter(), {
      write(command: ContextSessionRequest) {
        commands.push(command);
        let event;
        if ('registerHook' in command) event = { hookRegistered: { hookId: 'hook' } };
        if ('waitForHook' in command) event = { hookCompleted: { hookId: 'hook', dialog: {
          dialogId: 'dialog', type: 'prompt', message: 'Name?', defaultValue: 'default',
        } } };
        if ('handleDialog' in command) event = action === 'error' ? { error: { message: 'already handled' } } : { dialogHandled: { dialogId: 'dialog' } };
        queueMicrotask(() => stream.emit('data', event));
        return true;
      }, end() {}, cancel() {},
    });
    const runtime = { client: { ContextSession: () => stream }, openStreams: new Set(), registerStream() {}, unregisterStream() {} } as unknown as RuntimeClient;
    const page = new PageImpl({ runtime, browserSessionId: 'browser', sessionId: 'page' });
    const options = { timeoutMs };
    const retry = timeoutMs !== undefined ? { retryOptions: { timeoutMs } } : {};
    const hook = await page.registerHook(hooks.dialog);
    const dialog = await hook.wait({ timeoutMs: 50 });
    expect(dialog.page).toBe(page);
    expect(dialog.message).toBe('Name?');
    expect(dialog.defaultValue).toBe('default');
    if (action === 'error') await expect(dialog.accept(undefined, options)).rejects.toThrow('already handled');
    else if (action === 'dismiss') await dialog.dismiss(options);
    else await dialog.accept(action === 'empty' ? '' : action === 'input' ? 'Ada' : undefined, options);
    expect(commands[0]).toEqual({ surfaceSessionId: 'browser', contextSessionId: 'page', registerHook: { dialog: {} } });
    expect(commands[2]).toEqual({ surfaceSessionId: 'browser', contextSessionId: 'page', handleDialog: {
      dialogId: 'dialog', accept: action !== 'dismiss', promptText: action === 'empty' ? '' : action === 'input' ? 'Ada' : undefined, ...retry,
    } });
  });
}
