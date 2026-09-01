import type { Metadata } from "next";
import Link from "next/link";

import { GITHUB_URL, SITE_NAME, SITE_URL } from "../brand";
import { changelog } from "./changelog-data";

const description =
  "What allwright has actually shipped, release by release: browser and Android automation, new client languages, and the plugin architecture underneath all of it.";

export const metadata: Metadata = {
  title: "Changelog",
  description,
  alternates: { canonical: "/changelog" },
  openGraph: {
    type: "website",
    url: "/changelog",
    siteName: SITE_NAME,
    locale: "en_US",
    title: "Changelog: what shipped, release by release",
    description,
  },
  twitter: {
    card: "summary_large_image",
    title: "Changelog: what shipped, release by release",
    description,
  },
};

function formatEntryDate(date: string): string {
  return new Date(`${date}T00:00:00Z`).toLocaleDateString("en-US", {
    year: "numeric",
    month: "short",
    day: "numeric",
    timeZone: "UTC",
  });
}

export default function Changelog() {
  const jsonLd = {
    "@context": "https://schema.org",
    "@type": "ItemList",
    name: "allwright changelog",
    url: `${SITE_URL}/changelog`,
    itemListElement: changelog.map((entry, i) => ({
      "@type": "ListItem",
      position: i + 1,
      name: `${entry.version} — ${entry.title}`,
    })),
  };

  return (
    <div className="relative mx-auto w-full max-w-4xl pb-6">
      <script
        type="application/ld+json"
        // eslint-disable-next-line react/no-danger
        dangerouslySetInnerHTML={{ __html: JSON.stringify(jsonLd) }}
      />

      <section className="mx-auto mt-10 max-w-3xl text-center sm:mt-14">
        <p className="mb-5 inline-flex items-center gap-2 rounded-full border border-[var(--line)] bg-[var(--card)] px-4 py-1.5 font-mono text-[0.78rem] uppercase tracking-[0.14em] text-[var(--accent-2)]">
          Changelog
        </p>
        <h1 className="text-[clamp(2.2rem,5vw,3.4rem)] leading-[1.05] font-semibold tracking-[-0.03em] text-[var(--ink)]">
          What shipped, release by release.
        </h1>
        <p className="mt-5 text-[clamp(1rem,1.6vw,1.15rem)] leading-8 text-[var(--muted)]">
          allwright cuts a lot of small releases — most are packaging fixes
          or internal hardening nobody needs to read about. This page groups
          the ones that actually changed what you can build, newest first.
          For the full, unfiltered commit history, see the{" "}
          <a
            href={`${GITHUB_URL}/tags`}
            target="_blank"
            rel="noreferrer"
            className="font-medium text-[var(--accent-2)] hover:underline"
          >
            tags on GitHub
          </a>
          .
        </p>
        <p className="mt-4 text-sm leading-6 text-[var(--muted)]">
          Next up is <strong className="font-semibold text-[var(--ink)]">v0.1.0</strong>
          , allwright&apos;s first minor version — coming soon. Read{" "}
          <Link href="/blog/road-to-v0-1-0" className="font-medium text-[var(--accent-2)] hover:underline">
            what&apos;s tangible today on the way there
          </Link>
          .
        </p>
      </section>

      <section aria-label="release history" className="relative mx-auto mt-14 w-full sm:mt-16">
        <div
          aria-hidden="true"
          className="absolute left-[15px] top-2 bottom-2 hidden w-px bg-[var(--line)] sm:left-[27px] sm:block"
        />
        <ol className="flex flex-col gap-6">
          {changelog.map((entry) => (
            <li key={entry.version} className="relative flex gap-4 sm:gap-6">
              <span
                aria-hidden="true"
                className="relative z-10 mt-1.5 hidden h-3.5 w-3.5 shrink-0 rounded-full border-2 border-[var(--accent)] bg-[var(--background)] sm:block"
              />
              <article className="min-w-0 flex-1 rounded-[1.5rem] border border-[var(--line)] bg-[var(--card)] p-6 backdrop-blur-xl sm:p-7">
                <div className="flex flex-wrap items-center gap-x-3 gap-y-1">
                  <span className="rounded-full bg-[var(--accent-soft)] px-3 py-1 font-mono text-[0.7rem] font-medium text-[var(--accent)]">
                    {entry.version}
                  </span>
                  <span className="font-mono text-[0.72rem] uppercase tracking-[0.08em] text-[var(--muted)]">
                    {formatEntryDate(entry.date)}
                  </span>
                </div>
                <h2 className="mt-3 text-lg font-semibold text-[var(--ink)] sm:text-xl">
                  {entry.title}
                </h2>
                <ul className="mt-3 space-y-2">
                  {entry.highlights.map((item) => (
                    <li key={item} className="flex gap-2.5 text-sm leading-6 text-[var(--muted)]">
                      <span className="mt-1.5 h-1.5 w-1.5 shrink-0 rounded-full bg-[var(--accent)]" aria-hidden="true" />
                      <span>{item}</span>
                    </li>
                  ))}
                </ul>
              </article>
            </li>
          ))}
        </ol>
      </section>

      <section
        aria-label="get started"
        className="mx-auto mt-14 flex w-full flex-col items-center gap-4 rounded-[2rem] border border-[var(--line)] bg-[var(--card)] p-8 text-center backdrop-blur-xl sm:mt-16"
      >
        <h2 className="text-xl font-semibold text-[var(--ink)] sm:text-2xl">
          See what&apos;s real today
        </h2>
        <p className="max-w-[46ch] text-sm leading-6 text-[var(--muted)]">
          The changelog is history. For the current, continuously updated
          picture of what each surface can and can&apos;t do yet, see
          Availability.
        </p>
        <div className="flex flex-wrap items-center justify-center gap-3">
          <Link
            href="/availability"
            className="inline-flex items-center rounded-full bg-[linear-gradient(120deg,var(--accent),var(--accent-2))] px-6 py-3 text-sm font-semibold text-white shadow-[0_18px_40px_var(--accent-soft)] transition hover:-translate-y-0.5"
          >
            Full availability breakdown
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
