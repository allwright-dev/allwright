import { expect, test } from "bun:test";
import { EventEmitter } from "node:events";
import { PageImpl } from "../src/page.js";
import type { ContextSessionRequest, RuntimeClient } from "../src/types.js";

test("click forwards mouse options and dblclick forces two clicks", async () => {
  const commands: ContextSessionRequest[] = [];
  const runtime = {
    client: { ContextSession: () => {
      const stream = Object.assign(new EventEmitter(), {
        write(command: ContextSessionRequest) {
          commands.push(command);
          queueMicrotask(() => stream.emit("data", { elementClicked: {} }));
          return true;
        },
        end() {}, cancel() {},
      });
      return stream;
    } },
    openStreams: new Set(), registerStream() {}, unregisterStream() {},
  } as unknown as RuntimeClient;
  const page = new PageImpl({ runtime, browserSessionId: "browser", sessionId: "page" });

  await page.click("#menu", { button: "right", clickCount: 3, timeoutMs: 250 });
  await page.locator("#row").dblclick({ button: "left", timeoutMs: 500 });

  expect(commands[0]).toMatchObject({ clickElement: {
    cssSelector: 'css="#menu"', button: "right", clickCount: 3, retryOptions: { timeoutMs: 250 },
  } });
  expect(commands[1]).toMatchObject({ clickElement: {
    cssSelector: 'css="#row"', button: "left", clickCount: 2, retryOptions: { timeoutMs: 500 },
  } });
});
