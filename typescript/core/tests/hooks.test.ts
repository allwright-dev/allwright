import { expect, test } from "bun:test";
import { EventEmitter } from "node:events";

import { BrowserImpl } from "../src/browser.js";
import { PageImpl } from "../src/page.js";
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

test("file-chooser hook resolves to a typed chooser that sets files", async () => {
  const commands: ContextSessionRequest[] = [];
  const contextStream = Object.assign(new EventEmitter(), {
    write(command: ContextSessionRequest) {
      commands.push(command);
      if ("registerHook" in command) {
        queueMicrotask(() => contextStream.emit("data", { hookRegistered: { hookId: "hook-2" } }));
      } else if ("waitForHook" in command) {
        queueMicrotask(() => contextStream.emit("data", {
          hookCompleted: {
            hookId: "hook-2",
            fileChooser: { fileChooserId: "chooser-1", isMultiple: true },
          },
        }));
      } else if ("setFileChooserFiles" in command) {
        queueMicrotask(() => contextStream.emit("data", {
          fileChooserFilesSet: { fileChooserId: "chooser-1", files: command.setFileChooserFiles.files },
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
  const page = new PageImpl({ runtime, browserSessionId: "browser-1", sessionId: "page-1" });

  const hook = await page.registerHook(hooks.fileChooser);
  const chooser = await hook.wait();
  await chooser.setFiles(["one.txt", "two.txt"]);

  expect(chooser.isMultiple()).toBe(true);
  expect(chooser.page).toBe(page);
  expect(commands).toEqual([
    { surfaceSessionId: "browser-1", contextSessionId: "page-1", registerHook: { fileChooser: {} } },
    { surfaceSessionId: "browser-1", contextSessionId: "page-1", waitForHook: { hookId: "hook-2", retryOptions: undefined } },
    {
      surfaceSessionId: "browser-1",
      contextSessionId: "page-1",
      setFileChooserFiles: {
        fileChooserId: "chooser-1",
        files: ["one.txt", "two.txt"],
        retryOptions: undefined,
      },
    },
  ]);
});

test("download hook resolves when the download starts and saves it", async () => {
  const commands: ContextSessionRequest[] = [];
  const contextStream = Object.assign(new EventEmitter(), {
    write(command: ContextSessionRequest) {
      commands.push(command);
      if ("registerHook" in command) {
        queueMicrotask(() => contextStream.emit("data", { hookRegistered: { hookId: "hook-3" } }));
      } else if ("waitForHook" in command) {
        queueMicrotask(() => contextStream.emit("data", {
          hookCompleted: {
            hookId: "hook-3",
            download: {
              downloadId: "download-1",
              url: "https://example.test/report.csv",
              suggestedFilename: "report.csv",
            },
          },
        }));
      } else if ("saveDownload" in command) {
        queueMicrotask(() => contextStream.emit("data", {
          downloadSaved: { downloadId: "download-1", path: command.saveDownload.path },
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
  const page = new PageImpl({ runtime, browserSessionId: "browser-1", sessionId: "page-1" });

  const hook = await page.registerHook(hooks.download);
  const download = await hook.wait();
  await download.saveAs("artifacts/report.csv");

  expect(download.page).toBe(page);
  expect(download.url).toBe("https://example.test/report.csv");
  expect(download.suggestedFilename).toBe("report.csv");
  expect(commands).toEqual([
    { surfaceSessionId: "browser-1", contextSessionId: "page-1", registerHook: { download: {} } },
    { surfaceSessionId: "browser-1", contextSessionId: "page-1", waitForHook: { hookId: "hook-3", retryOptions: undefined } },
    {
      surfaceSessionId: "browser-1",
      contextSessionId: "page-1",
      saveDownload: {
        downloadId: "download-1",
        path: "artifacts/report.csv",
        retryOptions: undefined,
      },
    },
  ]);
});
