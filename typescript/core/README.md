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
await page.goto("https://example.com");
await browser.close();
```

A small runnable example also lives in [examples/basic.ts](./examples/basic.ts), and the fuller end-to-end playground lives in [examples/playground.ts](./examples/playground.ts).

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
