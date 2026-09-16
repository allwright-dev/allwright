import { expect, test } from "bun:test";
import { EventEmitter } from "node:events";

import { BrowserImpl } from "../src/browser.js";
import { hooks } from "../src/index.js";
import { EventQueue } from "../src/types.js";
import type {
  BrowserLaunchState,
  RuntimeClient,
  ContextSessionRequest,
  SurfaceSessionEvent,
  SurfaceSessionRequest,
  SurfaceSessionStream,
} from "../src/types.js";

test("generic new-page hook registers before actions and resolves to a managed page", async () => {
  const commands: ContextSessionRequest[] = [];
  const stream = {
    write(command: SurfaceSessionRequest) {
      commands.push(command);
      return true;
    },
    end() {},
  } as unknown as SurfaceSessionStream;
  const queue = new EventQueue<SurfaceSessionEvent>();
  const contextStream = Object.assign(new EventEmitter(), {
    write(command: ContextSessionRequest) {
      commands.push(command);
      if ("registerHook" in command) {
        queueMicrotask(() => contextStream.emit("data", { hookRegistered: { hookId: "hook-1" } }));
      } else if ("waitForHook" in command) {
        queueMicrotask(() => contextStream.emit("data", {
          hookCompleted: {
            hookId: "hook-1",
            newPage: { contextSessionId: "page-2", note: "completed" },
          },
        }));
      }
      return true;
    },
    end() {},
    cancel() {},
  });
  const runtime = {
    client: { ContextSession: () => contextStream },
    openStreams: new Set(),
    registerStream() {},
    unregisterStream() {},
  } as unknown as RuntimeClient;
  const state: BrowserLaunchState = {
    runtime,
    stream,
    queue,
    sessionId: "browser-1",
    launched: {
      browser: "Chromium",
      initialPageSessionId: "page-1",
    },
  };
  const browser = new BrowserImpl(state);

  const hook = await browser.page().registerHook(hooks.newPage);
  const page = await hook.wait();

  expect(page.sessionId).toBe("page-2");
  expect(commands).toEqual([
    { surfaceSessionId: "browser-1", contextSessionId: "page-1", registerHook: { newPage: {} } },
    { surfaceSessionId: "browser-1", contextSessionId: "page-1", waitForHook: { hookId: "hook-1", retryOptions: undefined } },
  ]);
  expect(browser.pages()).toHaveLength(2);
});
