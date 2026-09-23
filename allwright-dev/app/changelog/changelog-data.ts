// User-facing release history, grouped by shipped capability rather than by
// every individual tag. allwright cuts a lot of small releases (packaging
// fixes, internal hardening), so this groups consecutive tags that shipped
// one real capability into a single entry — newest first. Anyone who wants
// the raw, tag-by-tag commit history can follow the GitHub link on the page.

export type ChangelogEntry = {
  /** A single tag ("v0.0.57") or an inclusive range ("v0.0.45 – v0.0.52"). */
  version: string;
  /** ISO date of the entry's latest tag, e.g. "2026-09-01". */
  date: string;
  title: string;
  highlights: string[];
};

export const changelog: ChangelogEntry[] = [
  {
    version: "v0.1.13",
    date: "2026-09-23",
    title: "Iframes as pages",
    highlights: [
      "locator.frame() resolves an iframe to a normal Page across all five clients (TypeScript also accepts Frame(), Go spells it Frame(ctx, ...)) — the same locators, actions, and retrying assertions as any top-level page, and it can resolve nested frames the same way.",
      "Cross-origin frames work without reading the child DOM from the parent: the frame is resolved to its own WebDriver BiDi browsing context, over the existing Chromium mapper or Firefox's native BiDi.",
      "Resolution waits for exactly one matching iframe whose document has reached readyState complete and gone 200 ms without DOM mutations, retrying within the command timeout (default 10 seconds, configurable per call). This is a document readiness check, not a network-idle guarantee.",
      "Closing a frame page releases its session without closing the parent tab or removing the iframe; a detached frame fails clearly, so resolve the locator again after the iframe is replaced.",
    ],
  },
  {
    version: "v0.1.12",
    date: "2026-09-19",
    title: "No native dialogs, and every Chromium operation over BiDi",
    highlights: [
      "File-chooser sessions now tell the browser to dismiss its file prompt, and download hooks set an explicit download destination before the triggering action, so neither an OS file picker nor a native Save As dialog can ever appear during a run.",
      "Chromium tab discovery, creation, and closing moved from CDP onto WebDriver BiDi, joining navigation, input, screenshots, and hooks — CDP is now used only to bootstrap and carry messages for the BiDi mapper.",
    ],
  },
  {
    version: "v0.1.10 – v0.1.11",
    date: "2026-09-18",
    title: "Clearer installs and plugin versions",
    highlights: [
      "The CLI installer verifies the downloaded binary's version and, if an older package-manager-installed allwright appears earlier on PATH, names the exact executable shadowing it.",
      "allwright plugin list now reports outdated@<version> (expected@<version>) for a surface plugin that doesn't match the engine, and plugin installs print download and unpack progress.",
    ],
  },
  {
    version: "v0.1.8 – v0.1.9",
    date: "2026-09-17",
    title: "File hooks reach Android, and paths stay client-local",
    highlights: [
      "Android apps gained the same fileChooser and download hooks as web, registered on the app instead of a page — the system DocumentsUI picker with a single file, and downloads written to the device's public Downloads directory. newPage is web-only.",
      "Upload and download paths are always resolved on the machine running the test: files stream through the engine in chunks, so setFiles and saveAs work the same when the allwright server runs on another machine.",
    ],
  },
  {
    version: "v0.1.7",
    date: "2026-09-17",
    title: "File chooser and download hooks",
    highlights: [
      "Two new typed hooks join newPage on the same register-then-wait lifecycle: fileChooser intercepts the native file picker (no OS dialog ever renders) and resolves to a typed chooser whose setFiles/set_files/SetFiles hands it one or more paths.",
      "download resolves as soon as a download starts, exposing url and suggestedFilename immediately; its saveAs/save_as/SaveAs waits for the download to finish and copies it to the given path.",
      "Shipped across all five clients at once, reusing the same generic RegisterHookCommand/WaitForHookCommand engine primitives newPage already used.",
    ],
  },
  {
    version: "v0.1.6",
    date: "2026-09-16",
    title: "New-page hook made page-scoped, and correctly attributed",
    highlights: [
      "Fixed a real correctness bug: registering the new-page hook on the browser and resolving to \"the first new page since registration\" could hand back an unrelated tab if more than one could plausibly open around the same time.",
      "Hooks now register on the page that triggers the event, not the browser, and resolve to the new page whose opener is specifically that page — page.registerHook(...) replaces browser.registerHook(...) across all five clients.",
    ],
  },
  {
    version: "v0.1.5",
    date: "2026-09-16",
    title: "Hooks arrive: coordinating browser-native events",
    highlights: [
      "Introduced typed hooks — a generic register-then-wait lifecycle for browser-native events that fire asynchronously outside any direct call, so a test can start listening before the triggering action instead of racing it.",
      "The first hook type: newPage, for tabs opened by a click — hook.wait() resolves to a normal Page, with the same actions and retrying assertions as any other.",
    ],
  },
  {
    version: "v0.1.4",
    date: "2026-09-07",
    title: "Accessibility snapshots reach Android",
    highlights: [
      "Android gained accessibility_snapshot, sharing the same versioned JSON/YAML document structure web already had — one accessibility model across both surfaces instead of two.",
    ],
  },
  {
    version: "v0.1.3",
    date: "2026-09-06",
    title: "Semantic web locators, and excluding matches",
    highlights: [
      "Web pages and locators gained Playwright-style semantic builders across all five clients — getByRole, getByText, getByLabel, getByPlaceholder, getByAltText, getByTitle, and getByTestId — chainable with CSS/XPath and filterable by has, hasNot, hasText, hasNotText, and visible.",
      "locator.not(otherLocator) excludes matching elements, and Vitest assertions gained matching negation for both web and Android — expect(locator).not.toHaveText(...) and expect(locator).not().toBeVisible() — with the same retrying behavior as every other assertion.",
    ],
  },
  {
    version: "v0.1.2",
    date: "2026-09-06",
    title: "Snapshot modes, including an AI-ready one",
    highlights: [
      "Accessibility snapshots gained a mode alongside format: default (accessible content), ai (adds a stable aria-ref to every rendered, clickable element so it can be located by reference), autoexpect, and codegen.",
    ],
  },
  {
    version: "v0.1.1",
    date: "2026-09-06",
    title: "Web accessibility snapshots",
    highlights: [
      "Web pages in all five clients gained accessibility_snapshot / AccessibilitySnapshot / accessibilitySnapshot, returning a structured document of every node's role, name, states, properties, and children — including cross-origin iframes — as JSON (default) or YAML.",
      "Accessible-name and role computation is Allwright-owned code informed by the W3C accessibility specs and by studying Playwright's behavior as a reference — no vendored Playwright code, no dom-accessibility-api dependency.",
    ],
  },
  {
    version: "v0.1.0",
    date: "2026-09-04",
    title: "First minor version: a real milestone",
    highlights: [
      "allwright's first minor release after more than sixty 0.0.x patch releases — the point where what had already landed (web and Android automation, five client languages, one API shape, project scaffolding) added up to a real milestone instead of another patch bump.",
      "No new surfaces shipped in the tag itself; see the entries below for what landed in the days right after.",
    ],
  },
  {
    version: "v0.0.61",
    date: "2026-09-04",
    title: "Web actions hardened onto WebDriver BiDi",
    highlights: [
      "Fixed web element actions — click, count, and highlight — that were still going through raw CDP script evaluation instead of WebDriver BiDi, which could make them inconsistent with allwright's driverless automation model.",
      "Chromium's DevTools Protocol is now used strictly for browser/tab lifecycle and bootstrapping the BiDi mapper; every element interaction runs over BiDi.",
    ],
  },
  {
    version: "v0.0.59 – v0.0.60",
    date: "2026-09-03",
    title: "npm init allwright: a project initializer",
    highlights: [
      "New create-allwright package: run npm init allwright@latest (or npm create allwright@latest) to scaffold a working TypeScript or JavaScript project — config, starter tests for the surfaces you pick, and dependencies installed — in one command.",
      "Auto-detects your package manager from an existing lockfile, or falls back to whichever manager invoked it.",
      "Fully scriptable for CI: every prompt has a matching flag (--yes, --typescript/--javascript, --web/--mobile/--both, --package-manager, --no-install, --force).",
    ],
  },
  {
    version: "v0.0.57 – v0.0.58",
    date: "2026-09-01",
    title: "Android automation reaches parity with web",
    highlights: [
      "Android locators gained count, focus, press-key, read-text, and wait-for-selector — the same actions already available on web, now available on mobile too.",
      "Full-page Android screenshots now stitch several scrolled captures into one seamless image instead of cropping at the visible screen.",
      "The Vitest integration gained a config helper that starts the engine once for a whole test run and shuts it down when the suite finishes, instead of every test bootstrapping its own.",
    ],
  },
  {
    version: "v0.0.54 – v0.0.55",
    date: "2026-09-01",
    title: "Screenshots, everywhere",
    highlights: [
      "Screenshot capture shipped for every client language, on both web pages and Android apps.",
      "Added full-page capture and the option to save a screenshot straight to a file.",
    ],
  },
  {
    version: "v0.0.53",
    date: "2026-08-29",
    title: "One shared automation model for web and mobile",
    highlights: [
      "Unified the underlying session model so web and Android automation describe \"what to act on\" the same way, instead of two separate mental models.",
      "Added a consistent, matching getting-started example for every client language, for both the web and Android surfaces.",
      "Started early groundwork toward iOS support.",
    ],
  },
  {
    version: "v0.0.45 – v0.0.52",
    date: "2026-08-29",
    title: "Mobile automation debuts: Android",
    highlights: [
      "Android joined web as a real automation surface: connect to a device or emulator, install and launch an app, and drive it with the same locator model already used for the browser.",
      "Android selector support expanded to match common testing patterns — by visible text, resource id, class name, or state (like whether an element is clickable).",
      "Apps can now be installed straight from a URL, not just a local file.",
      "Web automation got more reliable: actions now wait for an element to be visible and settled before clicking or filling it, cutting down on flaky failures.",
    ],
  },
  {
    version: "v0.0.35",
    date: "2026-08-27",
    title: "Zero-install experience",
    highlights: [
      "Client libraries now download and manage the engine and the surface plugin automatically the first time they're used — no separate CLI install step required before writing a test.",
    ],
  },
  {
    version: "v0.0.33",
    date: "2026-08-26",
    title: "Firefox support, and a Playwright-style API redesign",
    highlights: [
      "Every client — Rust, Go, Java, Python, and TypeScript — gained a proper Browser / Page / Locator object model.",
      "Firefox joined Chromium as a fully supported browser.",
    ],
  },
  {
    version: "v0.0.27 – v0.0.30",
    date: "2026-08-26",
    title: "Config-file driven setup",
    highlights: [
      "Introduced allwright.config.yaml for describing how to launch a browser — binary, timeouts, named profiles — without writing it in code.",
      "Wired the config file into every client so it's picked up automatically.",
    ],
  },
  {
    version: "v0.0.31",
    date: "2026-08-26",
    title: "Java client published",
    highlights: [
      "The Java client became a real, installable Maven Central package, with its own test suite.",
    ],
  },
  {
    version: "v0.0.24",
    date: "2026-08-26",
    title: "Python client, and first-class Vitest support",
    highlights: [
      "The Python client was published to PyPI.",
      "Launched a dedicated Vitest integration package for TypeScript/JavaScript teams, with retrying, Playwright-style assertions.",
    ],
  },
  {
    version: "v0.0.19",
    date: "2026-08-25",
    title: "TypeScript client published to npm",
    highlights: ["The TypeScript client became a normal npm install."],
  },
  {
    version: "v0.0.16",
    date: "2026-08-25",
    title: "Rust crates published to crates.io",
    highlights: ["The Rust engine and client became a normal cargo install."],
  },
  {
    version: "v0.0.14",
    date: "2026-08-24",
    title: "First real browser actions ship",
    highlights: [
      "Launch a browser, open/navigate a page, click, fill, hover, focus, press a key, highlight, count, read text, and wait for an element to appear — allwright's first real automation commands.",
      "These actions retry automatically until they succeed or time out, instead of failing on the first attempt.",
    ],
  },
  {
    version: "v0.0.11",
    date: "2026-08-24",
    title: "Plugin architecture",
    highlights: [
      "Automation surfaces became installable plugins loaded on demand — starting with web — instead of one fixed, all-or-nothing engine.",
      "This is the architectural foundation every later surface (Android, and eventually iOS, desktop, and API) builds on.",
    ],
  },
  {
    version: "v0.0.7",
    date: "2026-08-24",
    title: "Initial public release",
    highlights: [
      "allwright launched with an installable CLI, a plugin system for automation surfaces, and client libraries for five languages at once: Rust, Go, Java, Python, and TypeScript.",
    ],
  },
];
