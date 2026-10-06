import { shutdown } from "@allwright.dev/core";
import { afterAll, expect as vitestExpect } from "vitest";
import { expect, test } from "../dist/index.js";

afterAll(() => shutdown());

test("windows fixtures stay lazy until first use", async ({ windows, windowsApp }) => {
  vitestExpect(windows).toBeTruthy();
  vitestExpect(windowsApp).toBeTruthy();
});

test.skipIf(!!process.env.ALLWRIGHT_WINDOWS_APP_ID)(
  "windowsApp reports missing launch configuration before connecting",
  async ({ windowsApp }) => {
    await vitestExpect(windowsApp.screenshot()).rejects.toThrow(
      "windowsApp fixture requires Windows launch options",
    );
  },
);

test.skipIf(process.platform !== "win32" || !process.env.ALLWRIGHT_WINDOWS_APP_ID)(
  "windowsApp lazily starts the FlaUI agent and launches the configured app",
  { timeout: 90_000 },
  async ({ windowsApp }) => {
    const screenshot = await windowsApp.screenshot();
    expect(screenshot.byteLength).toBeGreaterThan(0);
  },
);
