import type { Metadata } from "next";
import Link from "next/link";

import { languages, surfaceStatus } from "../availability-data";
import { SITE_NAME } from "../brand";
import { StatusPill } from "../status-pill";

const description =
  "The honest, current picture of allwright: what web, Android, iOS, macOS, and Windows desktop automation can do today, what Linux and API testing still need, and which client languages are published.";

export const metadata: Metadata = {
  title: "Availability",
  description,
  alternates: { canonical: "/availability" },
  openGraph: {
    type: "website",
    url: "/availability",
    siteName: SITE_NAME,
    locale: "en_US",
    title: "Availability: what's real today, what isn't yet",
    description,
  },
  twitter: {
    card: "summary_large_image",
    title: "Availability: what's real today, what isn't yet",
    description,
  },
};

const webAvailable = [
  "Launch a real Chromium or Firefox browser — no separate driver to install or version-match",
  "Open and close tabs within a browser session",
  "Work with nested and cross-origin iframes as pages, with automatic load waits and configurable timeouts",
  "Register a typed hook before an action and wait for the new tab that action opens",
  "Register a typed file-chooser hook and upload one or multiple local files",
  "Register a typed download hook and save the completed download to a local path",
  "Handle JavaScript alerts, confirms, and prompts with typed hooks — accept, dismiss, or enter prompt text",
  "Navigate to a URL",
  "Click, double-click, middle-click, or right-click an element with configurable click counts",
  "Type into a field",
  "Hover over an element",
  "Press a key on an element",
  "Focus an element",
  "Highlight matching elements for debugging",
  "Find elements by role, text, label, placeholder, alt text, title, or test ID, with filters, exclusions, and chained locators",
  "Count matching elements",
  "Read native accessibility snapshots as JSON or YAML and act through cached AI element references",
  "Read visible or raw text from an element",
  "Read input values, checked state, native accessibility attributes, and on-screen bounding boxes",
  "Capture the current page URL, live input values, selected options and text, checkbox/radio state, attributes, and element bounding boxes",
  "Wait for an element to appear or become visible",
  "Capture viewport or full-page screenshots",
  "Read page accessibility snapshots as JSON or standard YAML, with queryable element references in AI mode",
  "Retrying URL, value, selection, checked state, text, attribute, bounding-box, count, attached/hidden, enabled/disabled, focused, and editable assertions, including negation (via @allwright.dev/vitest)",
];

const webNotYetAvailable = [
  "Network mocking or request interception",
  "Cookies and saved session state",
  "Geolocation and other device permissions",
  "Mobile viewport and device emulation",
  "Drag and drop",
  "Multiple isolated browser profiles per session",
  "Video recording, tracing, coverage, or PDF export",
  "Safari / WebKit (Chromium and Firefox only today)",
];

const androidAvailable = [
  "Connect to a running emulator or a real device over adb — no separate driver server to run",
  "Install and launch a real app from a local APK or a URL",
  "Open universal links and custom URL schemes with app.goto / app.navigate",
  "Click an element",
  "Type into a field",
  "Focus an element",
  "Press a key",
  "Read visible or raw text from an element",
  "Wait for an element to appear",
  "Count matching elements",
  "Read native accessibility snapshots as JSON or YAML and act through cached AI element references",
  "Capture screenshots, including a full-page scroll-and-stitch capture",
  "Register typed file-chooser and public-download hooks with files streamed to and from the test machine",
  "Text, partial-text, resource id, class name, XPath, and state-based (e.g. clickable) selectors",
  "Playwright-style getByRole, getByText, getByLabel, and getByTestId native accessibility locators",
  "Retrying text, value, checked-state, attribute, bounding-box, count, attached/hidden, enabled/disabled, focused, and editable assertions, including negation (via @allwright.dev/vitest)",
];

const androidNotYetAvailable = [
  "Hover and highlight (web-only for now)",
  "Explicit swipe, scroll, drag-and-drop, pinch, and multi-touch gesture APIs",
  "Selected-option and selected-text reads",
  "Typed dialog and newly opened app-context hooks (file chooser and download hooks are available)",
  "Multiple isolated app sessions or switching among several foreground app contexts",
  "General CSS/DOM, iframe, or arbitrary in-app JavaScript sessions for WebViews",
  "Network mocking, request interception, cookies, permissions, tracing, and video recording",
];

const iosAvailable = [
  "Connect to an available iOS Simulator with no separate driver setup",
  "Connect to a registered physical device; the prebuilt agent is discovered, re-signed, installed, and forwarded automatically",
  "Download a simulator .ipa or ZIP, or use a local .app bundle, then install and launch it automatically",
  "Install and launch a signed physical-device .ipa, ZIP, or .app bundle without a manual install step",
  "Click, focus, fill, press keys, read text, wait for selectors, count elements, and capture screenshots",
  "Read input values, checked state, native accessibility attributes, and on-screen bounding boxes",
  "Open universal links and custom URL schemes with app.goto / app.navigate",
  "Capture full-page scroll-and-stitch screenshots",
  "Read native accessibility snapshots as JSON or YAML and act through cached AI element references",
  "Register typed file-chooser and download hooks with files streamed to and from the test machine",
  "Automate WebView content exposed through the native XCTest accessibility tree",
  "Accessibility-id, text, XCTest element type, and basic XPath selectors",
  "Playwright-style getByRole, getByText, getByLabel, and getByTestId native accessibility locators",
  "Playwright-style action auto-waiting and retrying text, value, checked-state, attribute, bounding-box, count, attached/hidden, enabled/disabled, focused, and editable assertions",
  "The same Rust, Go, Java, Python, and TypeScript client shape used by Android",
];

const iosNotYetAvailable = [
  "Hover and highlight (web-only for now)",
  "Explicit swipe, scroll, drag-and-drop, pinch, and multi-touch gesture APIs",
  "Selected-option and selected-text reads",
  "Typed dialog and newly opened app-context hooks (file chooser and download hooks are available)",
  "Multiple isolated app sessions or switching among several foreground app contexts",
  "Arbitrary in-page JavaScript and general CSS/DOM sessions inside WebViews",
  "Network mocking, request interception, cookies, permissions, tracing, and video recording",
];

const macAvailable = [
  "Connect to the local macOS desktop with an automatically started XCUITest runner",
  "Launch or terminate an application by bundle identifier",
  "Open universal links and custom URL schemes with app.goto / app.navigate",
  "Click, focus, fill, press keys, read text, wait for selectors, and count elements",
  "Read input values, checked state, native accessibility attributes, and on-screen bounding boxes",
  "Capture native application screenshots",
  "Read native accessibility snapshots as JSON or YAML and act through cached AI element references",
  "Use accessibility-id, text, XCTest element type, basic XPath, and Playwright-style semantic locators",
  "Use retrying text, value, checked-state, attribute, bounding-box, count, attached/hidden, enabled/disabled, focused, and editable assertions through @allwright.dev/vitest",
  "Use the same Rust, Go, Java, Python, and TypeScript server-only client shape as the other surfaces",
];

const macNotYetAvailable = [
  "Application installation or distribution — launch targets must already be installed",
  "Full-page or scroll-and-stitch screenshots",
  "File-chooser, download, dialog, and newly opened window hooks",
  "Selected-option and selected-text reads",
  "Hover, highlight, drag-and-drop, multi-click, and other pointer gesture APIs",
  "Multiple isolated desktop sessions or switching among several application windows",
  "Web-style DOM, iframe, arbitrary JavaScript, network, cookie, permission, tracing, and video APIs",
];

const winAvailable = [
  "Connect to the local Windows desktop with an automatically started UI Automation agent",
  "Launch or terminate an application by executable path, a command Windows can resolve, or a packaged-app ID",
  "Open universal links and custom URL schemes with app.goto / app.navigate",
  "Automate Win32, WinForms, WPF, UWP, and WinUI applications",
  "Click, focus, fill, press keys, read text, wait for selectors, and count elements",
  "Read input values, checked state, native UI Automation attributes, and on-screen bounding boxes",
  "Capture native application screenshots",
  "Read native accessibility snapshots as JSON or YAML and act through cached AI element references",
  "Use native UI Automation selectors and Playwright-style semantic locators",
  "Use retrying text, value, checked-state, attribute, bounding-box, count, attached/hidden, enabled/disabled, focused, and editable assertions through @allwright.dev/vitest",
  "Use the same Rust, Go, Java, Python, and TypeScript server-only client shape as the other surfaces",
];

const winNotYetAvailable = [
  "Windows on ARM and 32-bit Windows",
  "Application installation or distribution — launch targets must already be installed",
  "Full-page or scroll-and-stitch screenshots",
  "File-chooser, download, dialog, and newly opened window hooks",
  "Selected-option and selected-text reads",
  "Hover, highlight, drag-and-drop, multi-click, and other pointer gesture APIs",
  "Multiple isolated desktop sessions or switching among several application windows",
  "Web-style DOM, iframe, arbitrary JavaScript, network, cookie, permission, tracing, and video APIs",
];

const capabilityParity = [
  {
    capability: "Click, focus, fill, key input, text, count, and waits",
    web: "Available",
    android: "Available",
    ios: "Available",
    mac: "Available",
    windows: "Available",
  },
  {
    capability: "Semantic locators",
    web: "Available",
    android: "Available",
    ios: "Available",
    mac: "Available",
    windows: "Available",
  },
  {
    capability: "Accessibility snapshots and AI references",
    web: "Available",
    android: "Available",
    ios: "Available",
    mac: "Available",
    windows: "Available",
  },
  {
    capability: "Viewport screenshots",
    web: "Available",
    android: "Available",
    ios: "Available",
    mac: "Available",
    windows: "Available",
  },
  {
    capability: "Full-page / scroll-and-stitch screenshots",
    web: "Available",
    android: "Available",
    ios: "Available",
    mac: "Not yet",
    windows: "Not yet",
  },
  {
    capability: "URL navigation and deep links",
    web: "Available",
    android: "Available",
    ios: "Available",
    mac: "Available",
    windows: "Available",
  },
  {
    capability: "File chooser and download hooks",
    web: "Available",
    android: "Available",
    ios: "Available",
    mac: "Not yet",
    windows: "Not yet",
  },
  {
    capability: "Dialogs and newly opened contexts",
    web: "Available",
    android: "Not yet",
    ios: "Not yet",
    mac: "Not yet",
    windows: "Not yet",
  },
  {
    capability: "Input, checked, attribute, and box reads",
    web: "Available",
    android: "Available",
    ios: "Available",
    mac: "Available",
    windows: "Available",
  },
  {
    capability: "Selected-option and selected-text reads",
    web: "Available",
    android: "Not yet",
    ios: "Not yet",
    mac: "Not yet",
    windows: "Not yet",
  },
  {
    capability: "Retrying text, count, attached/hidden, enabled/disabled, focused, and editable assertions",
    web: "Available",
    android: "Available",
    ios: "Available",
    mac: "Available",
    windows: "Available",
  },
  {
    capability: "Retrying value, checked-state, attribute, and box assertions",
    web: "Available",
    android: "Available",
    ios: "Available",
    mac: "Available",
    windows: "Available",
  },
  {
    capability: "Hover, highlight, double-click, and right-click",
    web: "Available",
    android: "Not yet",
    ios: "Not yet",
    mac: "Not yet",
    windows: "Not yet",
  },
  {
    capability: "DOM, frames, and in-page JavaScript",
    web: "Available",
    android: "Accessibility tree only",
    ios: "Accessibility tree only",
    mac: "Native tree only",
    windows: "Native tree only",
  },
  {
    capability: "Install the application under test",
    web: "Not applicable",
    android: "Available",
    ios: "Available",
    mac: "Not yet",
    windows: "Not yet",
  },
] as const;

const plannedSurfaces = [
  {
    label: "Desktop — Linux",
    detail: "The shared desktop contract exists, but there is no installable Linux automation runtime yet.",
    status: "Not yet available" as const,
  },
  ...surfaceStatus.filter((surface) => surface.label === "API"),
];

export default function Availability() {
  return (
    <div className="relative mx-auto w-full max-w-6xl pb-6">
      <section className="mx-auto mt-10 max-w-3xl text-center sm:mt-14">
        <p className="mb-5 inline-flex items-center gap-2 rounded-full border border-[var(--line)] bg-[var(--card)] px-4 py-1.5 font-mono text-[0.78rem] uppercase tracking-[0.14em] text-[var(--accent-2)]">
          Availability
        </p>
        <h1 className="text-[clamp(2.2rem,5vw,3.4rem)] leading-[1.05] font-semibold tracking-[-0.03em] text-[var(--ink)]">
          What&apos;s real today, what isn&apos;t yet.
        </h1>
        <p className="mt-5 text-[clamp(1rem,1.6vw,1.15rem)] leading-8 text-[var(--muted)]">
          &ldquo;Available&rdquo; should mean something specific: installed,
          working, and ready for your test suite. Web automation runs today
          against real Chromium and Firefox browsers, Android automation runs
          on real devices and emulators, and iOS automation runs natively on
          Simulators and registered devices. Native macOS application automation
          runs through XCUITest, and native Windows application automation
          runs through UI Automation — each with the actions, locators, and retrying
          assertions everyday tests rely on. This page is the detailed,
          continuously updated picture behind the status pills you see
          elsewhere on the site — surface by surface, capability by
          capability, and language by language, including what&apos;s still
          missing.
        </p>
      </section>

      <section aria-label="surface availability" className="mx-auto mt-14 w-full sm:mt-16">
        <div className="mx-auto max-w-2xl text-center">
          <h2 className="text-xl font-semibold text-[var(--ink)] sm:text-2xl">
            Surfaces
          </h2>
          <p className="mt-3 text-sm leading-6 text-[var(--muted)] sm:text-base">
            Web, Mobile Android, Mobile iOS, Desktop macOS, and Desktop Windows have real,
            installable plugins today. The rest have a reserved place in the
            plugin catalog but no runtime build yet — installing them
            isn&apos;t possible until that changes.
          </p>
        </div>
        <div className="mt-8 grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
          {surfaceStatus.map((surface) => (
            <div
              key={surface.label}
              className="rounded-[1.5rem] border border-[var(--line)] bg-[var(--card)] p-6 backdrop-blur-xl"
            >
              <div className="flex flex-wrap items-start justify-between gap-2">
                <h3 className="text-base font-semibold text-[var(--ink)]">{surface.label}</h3>
                <StatusPill status={surface.status} />
              </div>
              <p className="mt-2 text-sm leading-6 text-[var(--muted)]">{surface.detail}</p>
            </div>
          ))}
        </div>
      </section>

      <section aria-label="cross-surface feature parity" className="mx-auto mt-14 w-full sm:mt-16">
        <div className="mx-auto max-w-2xl text-center">
          <h2 className="text-xl font-semibold text-[var(--ink)] sm:text-2xl">
            Cross-surface feature parity
          </h2>
          <p className="mt-3 text-sm leading-6 text-[var(--muted)] sm:text-base">
            The core action and locator model is shared everywhere. Features
            tied to browsers, mobile operating systems, or desktop windowing
            still differ; this matrix makes those boundaries explicit.
          </p>
        </div>
        <div className="mt-8 overflow-x-auto rounded-[2rem] border border-[var(--line)] bg-[var(--card)] backdrop-blur-xl">
          <table className="min-w-[920px] w-full border-collapse text-left text-sm">
            <thead>
              <tr className="border-b border-[var(--line)] text-[var(--ink)]">
                <th className="px-5 py-4 font-semibold">Capability</th>
                <th className="px-4 py-4 font-semibold">Web</th>
                <th className="px-4 py-4 font-semibold">Android</th>
                <th className="px-4 py-4 font-semibold">iOS</th>
                <th className="px-4 py-4 font-semibold">macOS</th>
                <th className="px-4 py-4 font-semibold">Windows</th>
              </tr>
            </thead>
            <tbody>
              {capabilityParity.map((row) => (
                <tr key={row.capability} className="border-b border-[var(--line)] last:border-b-0">
                  <th className="px-5 py-4 font-medium leading-6 text-[var(--ink)]">{row.capability}</th>
                  {[row.web, row.android, row.ios, row.mac, row.windows].map((value, index) => (
                    <td
                      key={`${row.capability}-${index}`}
                      className={`px-4 py-4 leading-6 ${
                        value === "Available"
                          ? "font-medium text-[var(--accent-2)]"
                          : value === "Not yet"
                            ? "text-[var(--muted)]"
                            : "text-[var(--ink)]"
                      }`}
                    >
                      {value}
                    </td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </section>

      <section aria-label="web capabilities" className="mx-auto mt-14 w-full sm:mt-16">
        <div className="mx-auto max-w-2xl text-center">
          <h2 className="text-xl font-semibold text-[var(--ink)] sm:text-2xl">
            Web, capability by capability
          </h2>
          <p className="mt-3 text-sm leading-6 text-[var(--muted)] sm:text-base">
            &ldquo;Available now&rdquo; means the web plugin is installable
            and everything below works against Chromium and Firefox today
            &mdash; enough to cover the core of a real web test suite. The
            list on the right is what hasn&apos;t landed yet, so you can check
            whether your suite depends on any of it.
          </p>
        </div>
        <div className="mt-8 grid gap-6 sm:grid-cols-2">
          <div className="rounded-[2rem] border border-[var(--line)] bg-[var(--card)] p-6 backdrop-blur-xl sm:p-8">
            <div className="flex items-center gap-2">
              <span className="h-2 w-2 rounded-full bg-[var(--accent)]" />
              <h3 className="text-sm font-semibold uppercase tracking-[0.08em] text-[var(--ink)]">
                Available now
              </h3>
            </div>
            <ul className="mt-5 space-y-3">
              {webAvailable.map((item) => (
                <li key={item} className="flex gap-2.5 text-sm leading-6 text-[var(--muted)]">
                  <span className="mt-1 h-1.5 w-1.5 shrink-0 rounded-full bg-[var(--accent)]" aria-hidden="true" />
                  <span>{item}</span>
                </li>
              ))}
            </ul>
          </div>
          <div className="rounded-[2rem] border border-dashed border-[var(--line)] bg-[var(--card)] p-6 backdrop-blur-xl sm:p-8">
            <div className="flex items-center gap-2">
              <span className="h-2 w-2 rounded-full border border-dashed border-[var(--muted)]" />
              <h3 className="text-sm font-semibold uppercase tracking-[0.08em] text-[var(--ink)]">
                Not yet available
              </h3>
            </div>
            <ul className="mt-5 space-y-3">
              {webNotYetAvailable.map((item) => (
                <li key={item} className="flex gap-2.5 text-sm leading-6 text-[var(--muted)]">
                  <span
                    className="mt-1 h-1.5 w-1.5 shrink-0 rounded-full border border-dashed border-[var(--muted)]"
                    aria-hidden="true"
                  />
                  <span>{item}</span>
                </li>
              ))}
            </ul>
          </div>
        </div>
        <p className="mx-auto mt-6 max-w-[56ch] text-center text-sm leading-6 text-[var(--muted)]">
          Every client language exposes this same capability set — there is
          no language-exclusive functionality on the web surface today.
        </p>
      </section>

      <section aria-label="android capabilities" className="mx-auto mt-14 w-full sm:mt-16">
        <div className="mx-auto max-w-2xl text-center">
          <h2 className="text-xl font-semibold text-[var(--ink)] sm:text-2xl">
            Mobile — Android, capability by capability
          </h2>
          <p className="mt-3 text-sm leading-6 text-[var(--muted)] sm:text-base">
            &ldquo;Available now&rdquo; means the mobile-android plugin is
            real, installable, and drives a genuine app over adb — no Appium,
            no separate driver server. It covers the core actions, reads,
            hooks, and the shared retrying assertion set. URL and selection
            matchers remain web-only.
          </p>
        </div>
        <div className="mt-8 grid gap-6 sm:grid-cols-2">
          <div className="rounded-[2rem] border border-[var(--line)] bg-[var(--card)] p-6 backdrop-blur-xl sm:p-8">
            <div className="flex items-center gap-2">
              <span className="h-2 w-2 rounded-full bg-[var(--accent)]" />
              <h3 className="text-sm font-semibold uppercase tracking-[0.08em] text-[var(--ink)]">
                Available now
              </h3>
            </div>
            <ul className="mt-5 space-y-3">
              {androidAvailable.map((item) => (
                <li key={item} className="flex gap-2.5 text-sm leading-6 text-[var(--muted)]">
                  <span className="mt-1 h-1.5 w-1.5 shrink-0 rounded-full bg-[var(--accent)]" aria-hidden="true" />
                  <span>{item}</span>
                </li>
              ))}
            </ul>
          </div>
          <div className="rounded-[2rem] border border-dashed border-[var(--line)] bg-[var(--card)] p-6 backdrop-blur-xl sm:p-8">
            <div className="flex items-center gap-2">
              <span className="h-2 w-2 rounded-full border border-dashed border-[var(--muted)]" />
              <h3 className="text-sm font-semibold uppercase tracking-[0.08em] text-[var(--ink)]">
                Not yet available
              </h3>
            </div>
            <ul className="mt-5 space-y-3">
              {androidNotYetAvailable.map((item) => (
                <li key={item} className="flex gap-2.5 text-sm leading-6 text-[var(--muted)]">
                  <span
                    className="mt-1 h-1.5 w-1.5 shrink-0 rounded-full border border-dashed border-[var(--muted)]"
                    aria-hidden="true"
                  />
                  <span>{item}</span>
                </li>
              ))}
            </ul>
          </div>
        </div>
        <p className="mx-auto mt-6 max-w-[56ch] text-center text-sm leading-6 text-[var(--muted)]">
          Every client language exposes this same Android capability set —
          same as web, nothing here is language-exclusive.
        </p>
      </section>

      <section aria-label="ios capabilities" className="mx-auto mt-14 w-full sm:mt-16">
        <div className="mx-auto max-w-2xl text-center">
          <h2 className="text-xl font-semibold text-[var(--ink)] sm:text-2xl">
            Mobile — iOS, capability by capability
          </h2>
          <p className="mt-3 text-sm leading-6 text-[var(--muted)] sm:text-base">
            The mobile-ios plugin is installable on macOS and provisions the
            runtime and app under test automatically, so users do not install
            a separate driver or sample IPA manually.
          </p>
        </div>
        <div className="mt-8 grid gap-6 sm:grid-cols-2">
          <div className="rounded-[2rem] border border-[var(--line)] bg-[var(--card)] p-6 backdrop-blur-xl sm:p-8">
            <h3 className="text-sm font-semibold uppercase tracking-[0.08em] text-[var(--ink)]">Available now</h3>
            <ul className="mt-5 space-y-3">
              {iosAvailable.map((item) => (
                <li key={item} className="flex gap-2.5 text-sm leading-6 text-[var(--muted)]">
                  <span className="mt-1 h-1.5 w-1.5 shrink-0 rounded-full bg-[var(--accent)]" aria-hidden="true" />
                  <span>{item}</span>
                </li>
              ))}
            </ul>
          </div>
          <div className="rounded-[2rem] border border-dashed border-[var(--line)] bg-[var(--card)] p-6 backdrop-blur-xl sm:p-8">
            <h3 className="text-sm font-semibold uppercase tracking-[0.08em] text-[var(--ink)]">Not yet available</h3>
            <ul className="mt-5 space-y-3">
              {iosNotYetAvailable.map((item) => (
                <li key={item} className="flex gap-2.5 text-sm leading-6 text-[var(--muted)]">
                  <span className="mt-1 h-1.5 w-1.5 shrink-0 rounded-full border border-dashed border-[var(--muted)]" aria-hidden="true" />
                  <span>{item}</span>
                </li>
              ))}
            </ul>
          </div>
        </div>
      </section>

      <section aria-label="macos desktop capabilities" className="mx-auto mt-14 w-full sm:mt-16">
        <div className="mx-auto max-w-2xl text-center">
          <h2 className="text-xl font-semibold text-[var(--ink)] sm:text-2xl">
            Desktop — macOS, capability by capability
          </h2>
          <p className="mt-3 text-sm leading-6 text-[var(--muted)] sm:text-base">
            The desktop-mac plugin is installable on macOS 14 or newer and
            starts its bundled XCUITest runner automatically. Xcode must be
            installed, UI automation permission must be enabled for the process
            running Xcode, and the app under test must already be installed.
          </p>
        </div>
        <div className="mt-8 grid gap-6 sm:grid-cols-2">
          <div className="rounded-[2rem] border border-[var(--line)] bg-[var(--card)] p-6 backdrop-blur-xl sm:p-8">
            <h3 className="text-sm font-semibold uppercase tracking-[0.08em] text-[var(--ink)]">Available now</h3>
            <ul className="mt-5 space-y-3">
              {macAvailable.map((item) => (
                <li key={item} className="flex gap-2.5 text-sm leading-6 text-[var(--muted)]">
                  <span className="mt-1 h-1.5 w-1.5 shrink-0 rounded-full bg-[var(--accent)]" aria-hidden="true" />
                  <span>{item}</span>
                </li>
              ))}
            </ul>
          </div>
          <div className="rounded-[2rem] border border-dashed border-[var(--line)] bg-[var(--card)] p-6 backdrop-blur-xl sm:p-8">
            <h3 className="text-sm font-semibold uppercase tracking-[0.08em] text-[var(--ink)]">Not yet available</h3>
            <ul className="mt-5 space-y-3">
              {macNotYetAvailable.map((item) => (
                <li key={item} className="flex gap-2.5 text-sm leading-6 text-[var(--muted)]">
                  <span className="mt-1 h-1.5 w-1.5 shrink-0 rounded-full border border-dashed border-[var(--muted)]" aria-hidden="true" />
                  <span>{item}</span>
                </li>
              ))}
            </ul>
          </div>
        </div>
      </section>

      <section aria-label="windows desktop capabilities" className="mx-auto mt-14 w-full sm:mt-16">
        <div className="mx-auto max-w-2xl text-center">
          <h2 className="text-xl font-semibold text-[var(--ink)] sm:text-2xl">
            Desktop — Windows, capability by capability
          </h2>
          <p className="mt-3 text-sm leading-6 text-[var(--muted)] sm:text-base">
            The desktop-windows plugin is installable on Windows x64 and starts
            its bundled, self-contained UI Automation agent automatically — no
            Appium, WinAppDriver, Developer Mode, or separate .NET install. The
            app under test must already be installed.
          </p>
        </div>
        <div className="mt-8 grid gap-6 sm:grid-cols-2">
          <div className="rounded-[2rem] border border-[var(--line)] bg-[var(--card)] p-6 backdrop-blur-xl sm:p-8">
            <h3 className="text-sm font-semibold uppercase tracking-[0.08em] text-[var(--ink)]">Available now</h3>
            <ul className="mt-5 space-y-3">
              {winAvailable.map((item) => (
                <li key={item} className="flex gap-2.5 text-sm leading-6 text-[var(--muted)]">
                  <span className="mt-1 h-1.5 w-1.5 shrink-0 rounded-full bg-[var(--accent)]" aria-hidden="true" />
                  <span>{item}</span>
                </li>
              ))}
            </ul>
          </div>
          <div className="rounded-[2rem] border border-dashed border-[var(--line)] bg-[var(--card)] p-6 backdrop-blur-xl sm:p-8">
            <h3 className="text-sm font-semibold uppercase tracking-[0.08em] text-[var(--ink)]">Not yet available</h3>
            <ul className="mt-5 space-y-3">
              {winNotYetAvailable.map((item) => (
                <li key={item} className="flex gap-2.5 text-sm leading-6 text-[var(--muted)]">
                  <span className="mt-1 h-1.5 w-1.5 shrink-0 rounded-full border border-dashed border-[var(--muted)]" aria-hidden="true" />
                  <span>{item}</span>
                </li>
              ))}
            </ul>
          </div>
        </div>
      </section>

      <section aria-label="planned surfaces" className="mx-auto mt-14 w-full sm:mt-16">
        <div className="mx-auto max-w-2xl text-center">
          <h2 className="text-xl font-semibold text-[var(--ink)] sm:text-2xl">
            Still to come
          </h2>
          <p className="mt-3 text-sm leading-6 text-[var(--muted)] sm:text-base">
            The Linux desktop plugin plus API testing remain part of
            the direction, but there is no installable runtime for them yet.
          </p>
        </div>
        <div className="mt-8 grid gap-4 sm:grid-cols-1">
          {plannedSurfaces.map((surface) => (
            <div
              key={surface.label}
              className="rounded-[1.5rem] border border-dashed border-[var(--line)] bg-[var(--card)] p-6 backdrop-blur-xl"
            >
              <div className="flex flex-wrap items-start justify-between gap-2">
                <h3 className="text-base font-semibold text-[var(--ink)]">{surface.label}</h3>
                <StatusPill status={surface.status} />
              </div>
              <p className="mt-2 text-sm leading-6 text-[var(--muted)]">{surface.detail}</p>
            </div>
          ))}
        </div>
      </section>

      <section aria-label="language client availability" className="mx-auto mt-14 w-full sm:mt-16">
        <div className="mx-auto max-w-2xl text-center">
          <h2 className="text-xl font-semibold text-[var(--ink)] sm:text-2xl">
            Client languages
          </h2>
          <p className="mt-3 text-sm leading-6 text-[var(--muted)] sm:text-base">
            &ldquo;Published&rdquo; means a normal package-manager install.
            &ldquo;From source&rdquo; means the client is complete and
            working, but you build it from the repository instead of pulling
            it from a package registry.
          </p>
        </div>
        <div className="mt-8 grid gap-4 sm:grid-cols-2 lg:grid-cols-5">
          {languages.map((lang) => (
            <a
              key={lang.name}
              href={lang.href}
              target="_blank"
              rel="noreferrer"
              className="flex h-full flex-col rounded-[1.25rem] border border-[var(--line)] bg-[var(--card)] p-5 backdrop-blur-xl transition hover:-translate-y-1 hover:border-[var(--accent-2)]"
            >
              <div className="flex flex-wrap items-start justify-between gap-2">
                <h3 className="font-mono text-sm font-semibold text-[var(--ink)]">{lang.name}</h3>
                <StatusPill status={lang.status} />
              </div>
              <p className="mt-2 text-xs leading-5 text-[var(--muted)]">{lang.note}</p>
              <span className="mt-auto inline-flex items-center gap-1 pt-3 text-xs font-medium text-[var(--accent-2)]">
                View example →
              </span>
            </a>
          ))}
        </div>
      </section>

      <section
        aria-label="get started"
        className="mx-auto mt-14 flex w-full flex-col items-center gap-4 rounded-[2rem] border border-[var(--line)] bg-[var(--card)] p-8 text-center backdrop-blur-xl sm:mt-16"
      >
        <h2 className="text-xl font-semibold text-[var(--ink)] sm:text-2xl">
          Ready to try it?
        </h2>
        <p className="max-w-[46ch] text-sm leading-6 text-[var(--muted)]">
          Scaffold a project and run your first test in a minute. The
          changelog tracks each new capability as it lands.
        </p>
        <div className="flex flex-wrap items-center justify-center gap-3">
          <Link
            href="/quickstart"
            className="inline-flex items-center rounded-full bg-[linear-gradient(120deg,var(--accent),var(--accent-2))] px-6 py-3 text-sm font-semibold text-white shadow-[0_18px_40px_var(--accent-soft)] transition hover:-translate-y-0.5"
          >
            Get started
          </Link>
          <Link
            href="/changelog"
            className="inline-flex items-center rounded-full border border-[var(--line)] bg-[var(--card)] px-6 py-3 text-sm font-medium text-[var(--ink)] transition hover:-translate-y-0.5 hover:border-[var(--accent-2)]"
          >
            Read the changelog
          </Link>
        </div>
      </section>
    </div>
  );
}
