# @allwright.dev/vitest

Vitest fixtures for allwright with Playwright-style `browser` and `page` injection.

Install:

```bash
bun add -d vitest @allwright.dev/vitest
```

```ts
import { expect, test } from "@allwright.dev/vitest";

test("opens a page", async ({ page }) => {
  await page.goto("https://example.com");
  const title = await page.textContent("h1");
  expect(title.text).toContain("Example");
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
import { test } from "@allwright.dev/vitest";

test.use({
  allwright: {
    suite: "smoke"
  }
});
```
