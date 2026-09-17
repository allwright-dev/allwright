import type { Metadata } from "next";
import Link from "next/link";

import { GITHUB_URL, SITE_NAME } from "../brand";
import { references } from "./data";

const description =
  "Every type and method allwright ships today, one page per client language — Rust, Go, Java, Python, and TypeScript — with a version badge on every entry matching the changelog.";

export const metadata: Metadata = {
  title: "API Reference",
  description,
  alternates: { canonical: "/reference" },
  openGraph: {
    type: "website",
    url: "/reference",
    siteName: SITE_NAME,
    locale: "en_US",
    title: "API Reference: every client, one page each",
    description,
  },
  twitter: {
    card: "summary_large_image",
    title: "API Reference: every client, one page each",
    description,
  },
};

export default function ReferenceIndex() {
  return (
    <div className="relative mx-auto w-full max-w-5xl pb-6">
      <section className="mx-auto mt-10 max-w-3xl text-center sm:mt-14">
        <p className="mb-5 inline-flex items-center gap-2 rounded-full border border-[var(--line)] bg-[var(--card)] px-4 py-1.5 font-mono text-[0.78rem] uppercase tracking-[0.14em] text-[var(--accent-2)]">
          API Reference
        </p>
        <h1 className="text-[clamp(2.2rem,5vw,3.4rem)] leading-[1.05] font-semibold tracking-[-0.03em] text-[var(--ink)]">
          One page per client, every method.
        </h1>
        <p className="mt-5 text-[clamp(1rem,1.6vw,1.15rem)] leading-8 text-[var(--muted)]">
          allwright's five client languages share one API shape, so the reference is organized the same
          way across all of them — Browser, Page, Locator, Hooks, Mobile, and Config. Pick a language to
          see its exact signatures.
        </p>
      </section>

      <section aria-label="languages" className="mx-auto mt-14 w-full sm:mt-16">
        <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
          {references.map((reference) => (
            <Link
              key={reference.id}
              href={`/reference/${reference.id}`}
              className="flex h-full flex-col rounded-[1.5rem] border border-[var(--line)] bg-[var(--card)] p-6 backdrop-blur-xl transition hover:-translate-y-1 hover:border-[var(--accent-2)]"
            >
              <h2 className="font-mono text-base font-semibold text-[var(--ink)]">{reference.label}</h2>
              <p className="mt-2 text-xs leading-5 text-[var(--muted)]">{reference.packageName}</p>
              <p className="mt-3 text-sm leading-6 text-[var(--muted)]">
                {reference.modules.length} modules, from launching a browser to registering a hook.
              </p>
              <span className="mt-auto inline-flex items-center gap-1 pt-4 text-xs font-medium text-[var(--accent-2)]">
                View reference →
              </span>
            </Link>
          ))}
        </div>
      </section>

      <section aria-label="how this stays current" className="mx-auto mt-14 w-full sm:mt-16">
        <div className="rounded-[2rem] border border-[var(--line)] bg-[var(--card)] p-6 backdrop-blur-xl sm:p-10">
          <h2 className="text-xl font-semibold text-[var(--ink)] sm:text-2xl">How the version badges work</h2>
          <p className="mt-3 text-sm leading-6 text-[var(--muted)] sm:text-base">
            All five clients ship the same version number together, so every entry on every language page
            carries a single <strong className="font-semibold text-[var(--ink)]">Since</strong> badge —
            the release documented on the{" "}
            <Link href="/changelog" className="font-medium text-[var(--accent-2)] hover:underline">
              changelog
            </Link>{" "}
            where that capability (or its current shape) first shipped. This reference always reflects the
            latest release, not a frozen snapshot of an older version.
          </p>
        </div>
      </section>

      <section
        aria-label="get started"
        className="mx-auto mt-14 flex w-full flex-col items-center gap-4 rounded-[2rem] border border-[var(--line)] bg-[var(--card)] p-8 text-center backdrop-blur-xl sm:mt-16"
      >
        <h2 className="text-xl font-semibold text-[var(--ink)] sm:text-2xl">New to allwright?</h2>
        <p className="max-w-[46ch] text-sm leading-6 text-[var(--muted)]">
          Start with Quickstart for a working project in one command, then come back here once you need
          the exact shape of a call.
        </p>
        <div className="flex flex-wrap items-center justify-center gap-3">
          <Link
            href="/quickstart"
            className="inline-flex items-center rounded-full bg-[linear-gradient(120deg,var(--accent),var(--accent-2))] px-6 py-3 text-sm font-semibold text-white shadow-[0_18px_40px_var(--accent-soft)] transition hover:-translate-y-0.5"
          >
            Go to Quickstart
          </Link>
          <a
            href={GITHUB_URL}
            target="_blank"
            rel="noreferrer"
            className="inline-flex items-center rounded-full border border-[var(--line)] bg-[var(--card)] px-6 py-3 text-sm font-medium text-[var(--ink)] transition hover:-translate-y-0.5 hover:border-[var(--accent-2)]"
          >
            Star on GitHub
          </a>
        </div>
      </section>
    </div>
  );
}
