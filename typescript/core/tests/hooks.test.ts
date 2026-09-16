import { expect, test } from "bun:test";

import { BrowserImpl } from "../src/browser.js";
import { hooks } from "../src/index.js";
import { EventQueue } from "../src/types.js";
import type {
  BrowserLaunchState,
  RuntimeClient,
  SurfaceSessionEvent,
  SurfaceSessionRequest,
  SurfaceSessionStream,
} from "../src/types.js";

test("generic new-page hook registers before actions and resolves to a managed page", async () => {
  const commands: SurfaceSessionRequest[] = [];
  const stream = {
    write(command: SurfaceSessionRequest) {
      commands.push(command);
      return true;
    },
    end() {},
  } as unknown as SurfaceSessionStream;
  const queue = new EventQueue<SurfaceSessionEvent>();
  const state: BrowserLaunchState = {
    runtime: {} as RuntimeClient,
    stream,
    queue,
    sessionId: "browser-1",
    launched: {
      browser: "Chromium",
      initialPageSessionId: "page-1",
    },
  };
  const browser = new BrowserImpl(state);

  queue.push({ hookRegistered: { hookId: "hook-1" } });
  const hook = await browser.registerHook(hooks.newPage);
  queue.push({
    hookCompleted: {
      hookId: "hook-1",
      newPage: { contextSessionId: "page-2", note: "completed" },
    },
  });
  const page = await hook.wait();

  expect(page.sessionId).toBe("page-2");
  expect(commands).toEqual([
    { registerHook: { newPage: {} } },
    { waitForHook: { hookId: "hook-1", retryOptions: undefined } },
  ]);
  expect(browser.pages()).toHaveLength(2);
});
