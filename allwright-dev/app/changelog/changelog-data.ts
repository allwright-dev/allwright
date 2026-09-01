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
    version: "v0.0.57",
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
