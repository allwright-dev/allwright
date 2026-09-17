import { expect, test } from "bun:test";
import { EventEmitter } from "node:events";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

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
      } else if ("uploadFileChunk" in command && command.uploadFileChunk.last) {
        queueMicrotask(() => contextStream.emit("data", {
          fileUploaded: {
            transferId: command.uploadFileChunk.transferId,
            fileId: `file-${commands.filter((item) => "uploadFileChunk" in item).length}`,
          },
        }));
      } else if ("setFileChooserFiles" in command) {
        queueMicrotask(() => contextStream.emit("data", {
          fileChooserFilesSet: { fileChooserId: "chooser-1", fileIds: command.setFileChooserFiles.fileIds },
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

  const directory = await mkdtemp(join(tmpdir(), "allwright-hook-test-"));
  const one = join(directory, "one.txt");
  const two = join(directory, "two.txt");
  await writeFile(one, "one");
  await writeFile(two, "two");
  const hook = await page.registerHook(hooks.fileChooser);
  const chooser = await hook.wait();
  await chooser.setFiles([one, two]);

  expect(chooser.isMultiple()).toBe(true);
  expect(chooser.page).toBe(page);
  const setCommand = commands.find((command) => "setFileChooserFiles" in command);
  expect(setCommand && "setFileChooserFiles" in setCommand
    ? setCommand.setFileChooserFiles.fileIds
    : []).toEqual(["file-1", "file-2"]);
  expect(commands.filter((command) => "uploadFileChunk" in command)).toHaveLength(2);
  await rm(directory, { recursive: true });
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
          downloadSaved: { downloadId: "download-1", fileId: "file-download-1" },
        }));
      } else if ("readFileChunk" in command) {
        queueMicrotask(() => contextStream.emit("data", {
          fileChunk: {
            fileId: "file-download-1",
            offset: command.readFileChunk.offset,
            data: Buffer.from("report contents"),
            last: true,
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
  const page = new PageImpl({ runtime, browserSessionId: "browser-1", sessionId: "page-1" });

  const directory = await mkdtemp(join(tmpdir(), "allwright-hook-test-"));
  const destination = join(directory, "report.csv");
  const hook = await page.registerHook(hooks.download);
  const download = await hook.wait();
  await download.saveAs(destination);

  expect(download.page).toBe(page);
  expect(download.url).toBe("https://example.test/report.csv");
  expect(download.suggestedFilename).toBe("report.csv");
  expect(await readFile(destination, "utf8")).toBe("report contents");
  const saveCommand = commands.find((command) => "saveDownload" in command);
  expect(saveCommand && "saveDownload" in saveCommand ? saveCommand.saveDownload : null)
    .toEqual({ downloadId: "download-1", retryOptions: undefined });
  expect(commands.some((command) => "readFileChunk" in command)).toBe(true);
  await rm(directory, { recursive: true });
});
