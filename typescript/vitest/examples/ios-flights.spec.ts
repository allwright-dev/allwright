import { expect, test } from "../dist/index.js";

test("shows a validation alert for an unknown account", { timeout: 60_000 }, async ({ iosApp }) => {
  await iosApp.locator("text=Login").click();
  await expect(iosApp.locator("text=Welcome back")).toHaveText("Welcome back");

  await iosApp.locator("className=XCUIElementTypeTextField").fill("allwright@example.com");
  await iosApp.locator("className=XCUIElementTypeSecureTextField").fill("not-a-real-password");
  await iosApp.locator("text=Submit").click();

  await expect(iosApp.locator("text=No account found. Please sign up first.")).toHaveText(
    "No account found. Please sign up first.",
  );
});
