import { expect, test } from "../dist/index.js";

test("captures the configured macOS application", { timeout: 60_000 }, async ({ macosApp }) => {
  const screenshot = await macosApp.screenshot();
  expect(screenshot.byteLength).toBeGreaterThan(0);
});
