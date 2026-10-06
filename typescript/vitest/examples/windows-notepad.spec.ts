import { expect, test } from "../dist/index.js";

test("captures the configured Windows application", { timeout: 60_000 }, async ({ windowsApp }) => {
  const screenshot = await windowsApp.screenshot();
  expect(screenshot.byteLength).toBeGreaterThan(0);
});
