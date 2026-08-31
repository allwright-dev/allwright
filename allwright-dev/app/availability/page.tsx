import type { Metadata } from "next";
import Link from "next/link";

import { languages, surfaceStatus } from "../availability-data";
import { GITHUB_URL } from "../brand";
import { StatusPill } from "../status-pill";

const description =
  "The honest, current picture of allwright: what web automation can do today, what mobile, desktop, and API testing still need, and which client languages are published versus build-from-source.";

export const metadata: Metadata = {
  title: "Availability",
  description,
  alternates: { canonical: "/availability" },
  openGraph: {
    type: "website",
    url: "/availability",
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
  "Navigate to a URL",
  "Click an element",
  "Type into a field",
  "Hover over an element",
  "Press a key on an element",
  "Focus an element",
  "Highlight matching elements for debugging",
  "Count matching elements",
  "Read visible or raw text from an element",
  "Wait for an element to appear or become visible",
  "Capture screenshots",
  "Retrying, Playwright-style text/count/visibility assertions (via @allwright.dev/vitest)",
];

const webNotYetAvailable = [
  "File upload and download handling",
  "Browser dialogs (alerts, confirms, prompts)",
  "Network mocking or request interception",
  "Cookies and saved session state",
  "Geolocation and other device permissions",
  "Mobile viewport and device emulation",
  "Drag and drop",
  "Multiple isolated browser profiles per session",
  "Safari / WebKit (Chromium and Firefox only today)",
];

const otherSurfaces = surfaceStatus.filter((surface) => surface.label !== "Web");

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
          allwright is being built in public, and &ldquo;available&rdquo;
          should mean something specific: real and working, not finished.
          Web automation runs today against real Chromium and Firefox
          browsers, but only through a small, minimal set of actions —
          nowhere near full web test coverage yet. This page is the
          detailed, continuously updated picture behind the status pills you
          see elsewhere on the site — surface by surface, capability by
          capability, and language by language.
        </p>
      </section>

      <section aria-label="surface availability" className="mx-auto mt-14 w-full sm:mt-16">
        <div className="mx-auto max-w-2xl text-center">
          <h2 className="text-xl font-semibold text-[var(--ink)] sm:text-2xl">
            Surfaces
          </h2>
          <p className="mt-3 text-sm leading-6 text-[var(--muted)] sm:text-base">
            Web is the only surface with a real, installable plugin today.
            The rest have a reserved place in the plugin catalog but no
            runtime build yet — installing them isn&apos;t possible until
            that changes.
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

      <section aria-label="web capabilities" className="mx-auto mt-14 w-full sm:mt-16">
        <div className="mx-auto max-w-2xl text-center">
          <h2 className="text-xl font-semibold text-[var(--ink)] sm:text-2xl">
            Web, capability by capability
          </h2>
          <p className="mt-3 text-sm leading-6 text-[var(--muted)] sm:text-base">
            &ldquo;Available now&rdquo; means the web plugin is real,
            installable, and the actions below genuinely work against
            Chromium and Firefox &mdash; not that web automation is done.
            This is a small, deliberately minimal core action set today, and
            on its own it is not yet enough to cover a real web test suite.
            The list on the right is what&apos;s still missing before it is.
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

      <section aria-label="planned surfaces" className="mx-auto mt-14 w-full sm:mt-16">
        <div className="mx-auto max-w-2xl text-center">
          <h2 className="text-xl font-semibold text-[var(--ink)] sm:text-2xl">
            Mobile, desktop, and API
          </h2>
          <p className="mt-3 text-sm leading-6 text-[var(--muted)] sm:text-base">
            These have a reserved slot in the plugin catalog and are part of
            the direction, but there is no installable plugin and nothing to
            try yet.
          </p>
        </div>
        <div className="mt-8 grid gap-4 sm:grid-cols-3">
          {otherSurfaces.map((surface) => (
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
          This page changes as allwright ships
        </h2>
        <p className="max-w-[46ch] text-sm leading-6 text-[var(--muted)]">
          Follow the repository to see new capabilities and surfaces land in
          real time, or head back to see how the plugin model fits together.
        </p>
        <div className="flex flex-wrap items-center justify-center gap-3">
          <a
            href={GITHUB_URL}
            target="_blank"
            rel="noreferrer"
            className="inline-flex items-center rounded-full bg-[linear-gradient(120deg,var(--accent),var(--accent-2))] px-6 py-3 text-sm font-semibold text-white shadow-[0_18px_40px_var(--accent-soft)] transition hover:-translate-y-0.5"
          >
            Star on GitHub
          </a>
          <Link
            href="/how-it-works"
            className="inline-flex items-center rounded-full border border-[var(--line)] bg-[var(--card)] px-6 py-3 text-sm font-medium text-[var(--ink)] transition hover:-translate-y-0.5 hover:border-[var(--accent-2)]"
          >
            See how it works
          </Link>
        </div>
      </section>
    </div>
  );
}
