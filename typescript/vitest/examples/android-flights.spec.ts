import { expect, test } from "../dist/index.js";

test("opens the Flights sign-up flow", { timeout: 60_000 }, async ({ androidApp }) => {
  await androidApp.getByText("Account", { exact: true }).click();
  await androidApp.getByText("Login", { exact: true }).click();
  await androidApp.getByText("Sign Up", { exact: true }).click();

  const screenshot = await androidApp.screenshot();
  expect(screenshot.byteLength).toBeGreaterThan(0);
});
