import { expect, test } from "../dist/index.js";

test.use({
  allwright: {
    suite: "firefox-dev",
  },
});

test("renders the Example Domain heading", async ({ page }) => {
  await page.goto("https://example.com");
  await expect(page.locator("h1")).toHaveText("Example Domain");
});
