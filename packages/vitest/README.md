# @allwright.dev/vitest

Vitest fixtures for allwright with Playwright-style `browser` and `page` injection.

```ts
import { expect, test } from "@allwright.dev/vitest";

test("opens a page", async ({ page }) => {
  await page.goto("https://example.com");
  const title = await page.textContent("h1");
  expect(title.text).toContain("Example");
});
```
