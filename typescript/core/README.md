# @allwright.dev/core

High-level TypeScript client for the allwright automation engine.

Install:

```bash
bun add @allwright.dev/core
```

```ts
import { firefox } from "@allwright.dev/core";

const browser = await firefox.launch();
const page = browser.page();
await page.goto("https://themoderninternet.vercel.app");
await page.click(
  "xpath=//div[contains(@class,'card')][.//h2[normalize-space()='Form Inputs']]//button[normalize-space()='Visit page']",
);
await browser.close();
```

Register a typed hook before an action that opens a tab, then wait for its
typed result afterward:

```ts
import { firefox, hooks } from "@allwright.dev/core";

const browser = await firefox.launch();
const page = browser.page();
const hook = await page.registerHook(hooks.newPage);
await page.click("a[target=_blank]");
const newPage = await hook.wait();
```

```ts
const hook = await page.registerHook(hooks.fileChooser);
await page.click("button.open-upload");
const chooser = await hook.wait();
await chooser.setFiles("fixtures/document.pdf");
```

```ts
const hook = await page.registerHook(hooks.download);
await page.click("a.download-report");
const download = await hook.wait();
await download.saveAs(`artifacts/${download.suggestedFilename}`);
```

Runnable examples live in [examples/web-basic.ts](./examples/web-basic.ts) and [examples/android-basic.ts](./examples/android-basic.ts).

Shared config files are stack-agnostic and can live in `allwright.config.yaml` or `allwright.config.json`.
The shared schema lives at the repo root in `allwright.schema.json`.

```yaml
schemaVersion: 1

server:
  addr: 127.0.0.1:50051

browser:
  name: firefox
  binary: /Applications/Firefox.app/Contents/MacOS/firefox
  launchOptions:
    timeoutMs: 30000

expect:
  timeoutMs: 5000
  intervalMs: 100

suites:
  smoke:
    browser:
      name: chromium
```

The TypeScript package exports `findConfigFile()`, `loadConfigFile()`, `resolveConfig()`, and `launchConfiguredBrowser()` so runner packages can consume the same config model without inventing language-specific config files.
