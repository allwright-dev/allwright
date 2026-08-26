// Shared "what's real today" data, reused by both /how-it-works (which
// explains the plugin model) and /availability (which spells out exactly
// what each surface and client can and can't do yet). Keeping this in one
// place means a status can't drift between pages.

import { GITHUB_URL } from "./brand";

export const languages = [
  {
    name: "Rust",
    note: "The engine itself, plus a high-level client.",
    status: "Published",
    href: `${GITHUB_URL}/blob/main/rust/allwright/examples/playground.rs`,
  },
  {
    name: "Go",
    note: "A Playwright-style Browser and Page API.",
    status: "Published",
    href: `${GITHUB_URL}/tree/main/go/examples/playground`,
  },
  {
    name: "Java",
    note: "A Gradle-based client for JVM test suites.",
    status: "From source",
    href: `${GITHUB_URL}/tree/main/java`,
  },
  {
    name: "Python",
    note: "A client that feels at home in pytest.",
    status: "From source",
    href: `${GITHUB_URL}/tree/main/python`,
  },
  {
    name: "TypeScript",
    note: "Works from TypeScript or plain JavaScript.",
    status: "Published",
    href: `${GITHUB_URL}/blob/main/typescript/examples/playground.ts`,
  },
] as const;

export const surfaceStatus = [
  {
    label: "Web",
    detail: "Chromium and Firefox both work, but only a small core action set — not yet full web test coverage.",
    status: "Available now" as const,
  },
  { label: "Mobile", detail: "Android & iOS, native and hybrid apps.", status: "Not yet available" as const },
  { label: "Desktop", detail: "macOS, Windows, and Linux applications.", status: "Not yet available" as const },
  { label: "API", detail: "Backend checks in the same test run.", status: "Not yet available" as const },
];
