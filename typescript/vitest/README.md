# @allwright.dev/vitest

Vitest fixtures for allwright with Playwright-style `browser` and `page` injection, plus Android mobile fixtures for hybrid tests.

Wrap the Vitest configuration with `allwrightVitestConfig`. It starts Allwright once before workers run and shuts it down once after the complete test run. Existing user global setup files remain supported and run after Allwright starts, then tear down before Allwright stops.

```ts
import { allwrightVitestConfig } from "@allwright.dev/vitest/config";
import { defineConfig } from "vitest/config";

export default allwrightVitestConfig(defineConfig({
  test: {
    globalSetup: ["./test/my-global-setup.ts"],
  },
}));
```

Install:

```bash
bun add -d vitest @allwright.dev/vitest
```

```ts
import { expect, test } from "@allwright.dev/vitest";

test("opens a page", async ({ page }) => {
  await page.goto("https://themoderninternet.vercel.app");
  await page.click(
    "xpath=//div[contains(@class,'card')][.//h2[normalize-space()='Form Inputs']]//button[normalize-space()='Visit page']",
  );
  await expect(page.locator('xpath=//h1[text()="Form Inputs"]')).toHaveText("Form Inputs");
});

test("opens an Android app", async ({ androidApp }) => {
  await androidApp.click('Id=com.example.airticket:id/bottom_nav_account');
});
```

A checked-in example spec also lives in [examples/basic.spec.ts](./examples/basic.spec.ts).

The fixture package reads the shared stack-agnostic config format through `@allwright.dev/core`.
Use `allwright.config.yaml` by default, or `allwright.config.json` if you prefer. Both follow the same root schema in `allwright.schema.json`.

```yaml
schemaVersion: 1

server:
  addr: 127.0.0.1:50051

web:
  browser:
    name: firefox
    binary: /Applications/Firefox.app/Contents/MacOS/firefox

expect:
  timeoutMs: 7000
  intervalMs: 100

suites:
  smoke:
    web:
      browser:
        name: chromium
```

Vitest can override or select from that shared config without switching to a Vitest-specific file:

```ts
import { expect, test as base } from "@allwright.dev/vitest";

const test = base.extend({
  allwright: async ({}, use) => {
    await use({ suite: "smoke" });
  }
});
```

Android fixtures are available alongside the web fixtures:

```ts
import { expect, test as base } from "@allwright.dev/vitest";

const test = base.extend({
  allwright: async ({}, use) => {
    await use({
      android: {
        launchOptions: {
          apkPath: "/absolute/path/to/app.apk",
          appId: "com.example.airticket",
        },
      },
    });
  },
});

test("android only", async ({ androidApp }) => {
  await androidApp.fill('xpath=//*[@text="Email"]', "user@example.com");
  await expect(androidApp.locator('xpath=//*[@text="Email"]')).toHaveText("user@example.com");
});

test("hybrid web and android", async ({ page, androidApp }) => {
  await page.goto("https://themoderninternet.vercel.app");
  await androidApp.click('Id=com.example.airticket:id/bottom_nav_account');
});
```

Fixtures are lazy on first use. Injecting `browser`, `page`, `android`, or `androidApp` does not launch or connect immediately; the underlying session is created only when the test first performs an action through that fixture. Sync metadata properties like `browser.sessionId` or `android.sessionId` are only available after the lazy fixture has been realized by a prior awaited call.

Available fixtures:

- `browser`: launched web browser
- `page`: initial web page
- `android`: connected Android device session
- `androidApp`: launched Android app page

`androidApp` launches using `allwright.android.launchOptions` first, then falls back to `config.mobile.android`.

Android apps and locators support the Android-applicable action and expectation subset: `click`, `count`, `focus`, `fill`, `press`, `textContent`, `innerText`, `waitForSelector`, and `screenshot`. `expect(androidApp)` and `expect(androidApp.locator(...))` provide `toHaveText`, `toContainText`, `toHaveCount`, and `toBeVisible` with the same retry controls as web fixtures. Hover and highlight remain web-only.
