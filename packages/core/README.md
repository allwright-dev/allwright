# @allwright.dev/core

High-level TypeScript client for the allwright automation engine.

```ts
import { chromium } from "@allwright.dev/core";

const browser = await chromium.launch();
const page = browser.page();
await page.goto("https://example.com");
await browser.close();
```
