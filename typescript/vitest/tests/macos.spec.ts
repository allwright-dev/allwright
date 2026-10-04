import { shutdown } from "@allwright.dev/core";
import { afterAll } from "vitest";
import { expect, test } from "../dist/index.js";

afterAll(() => shutdown());

test.skipIf(process.platform !== "darwin" || !process.env.ALLWRIGHT_MAC_APP_ID)(
  "macosApp lazily starts the desktop runner and launches the configured app",
  { timeout: 90_000 },
  async ({ macosApp }) => {
    const screenshot = await macosApp.screenshot();
    expect(screenshot.byteLength).toBeGreaterThan(0);
  },
);
