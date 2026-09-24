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

Paths passed to `setFiles` and `saveAs` belong to the test process, even when
the Allwright server is remote. Android apps support the same `fileChooser` and
`download` hook shape; Android `newPage` hooks are not applicable.

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

### Iframes

An iframe resolves to a normal `Page`, including nested and cross-origin frames:

```ts
const frame = await page.locator("#checkout-frame").Frame({ timeoutMs: 10_000 });
await frame.locator("input[name=email]").fill("buyer@example.com");
const nested = await frame.locator("iframe").frame({ timeoutMs: 5_000 });
```

`Frame()` and `frame()` are aliases. The timeout covers finding exactly one iframe,
resolving its browsing context, and waiting for a fully loaded document with 200 ms
without DOM mutations. The default is 10 seconds. Closing the frame page releases
its session; it does not remove the iframe or close the parent tab. Resolve a new
frame page after the iframe is replaced.

### Read page and element state

```ts
const url = await page.url();
const value = await page.locator('input, textarea').inputValue();
const options = await page.locator('select').selectedOptions(); // [{ value, label, index }]
const selectedText = await page.locator('textarea').selectedText();
const checked = await page.locator('input[type=checkbox]').isChecked();
const text = await page.locator('h1').innerText(); // { selector, text, note }
const attribute = await page.locator('a').getAttribute('href');
const box = await page.locator('button').boundingBox(); // { x, y, width, height }
```

Element reads also have page methods such as `page.inputValue(selector, options)` and accept `timeoutMs`. Reads use the first locator match and work in frame pages. `inputValue` reads the live property of an input, textarea, or select. `selectedOptions` supports native selects (including multiple selections) and ARIA combobox/listbox options marked `aria-selected="true"`. `selectedText` returns the textbox's selected substring, an empty string for a caret, or `null` for input types without selection support. `isChecked` supports native checkbox/radio and ARIA checked controls; ARIA mixed state returns false.

Missing attributes return `null` (an empty attribute remains `""`). Bounding boxes use CSS pixels relative to the page/frame viewport and return `null` for hidden or zero-area elements. Missing elements and incompatible control types raise errors. Use `textContent()` for raw text or `innerText()` for rendered text.
