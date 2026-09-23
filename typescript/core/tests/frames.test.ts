import { expect, test } from "bun:test";
import { EventEmitter } from "node:events";
import { PageImpl } from "../src/page.js";
import type { RuntimeClient, ContextSessionRequest } from "../src/types.js";

test("Frame returns a page, forwards timeout, and scopes nested frames to their parent", async () => {
  const commands: ContextSessionRequest[] = [];
  let id = 0;
  const runtime = {
    client: { ContextSession: () => {
      const stream = Object.assign(new EventEmitter(), {
        write(command: ContextSessionRequest) {
          commands.push(command);
          if ("resolveFrame" in command) queueMicrotask(() => stream.emit("data", {frameResolved: {contextSessionId: `frame-${++id}`}}));
          return true;
        }, end() {}, cancel() {},
      });
      return stream;
    } },
    openStreams: new Set(), registerStream() {}, unregisterStream() {},
  } as unknown as RuntimeClient;
  const page = new PageImpl({ runtime, browserSessionId: "browser", sessionId: "parent" });
  const child = await page.locator("iframe").Frame({ timeoutMs: 4321 });
  expect(child).toBeInstanceOf(PageImpl);
  expect(child.sessionId).toBe("frame-1");
  await child.locator("iframe").frame({ timeoutMs: 1000 });
  expect(commands[0]).toMatchObject({ contextSessionId: "parent", resolveFrame: { retryOptions: { timeoutMs: 4321 } } });
  expect(commands[1]).toMatchObject({ contextSessionId: "frame-1", resolveFrame: { retryOptions: { timeoutMs: 1000 } } });
});
