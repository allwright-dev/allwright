import Link from "next/link";

import { GITHUB_URL, LogoMark, SITE_DESCRIPTION, SITE_NAME, SITE_URL } from "./brand";
import { StatusPill } from "./status-pill";

const surfaces = [
  {
    label: "Web",
    description: "A small, working set of core browser actions today — not yet the full coverage real test suites need.",
    status: "Available now",
    position: { left: "6%", top: "10%" },
    icon: (
      <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6">
        <circle cx="12" cy="12" r="9" />
        <path d="M3 12h18M12 3c2.4 2.6 3.6 5.7 3.6 9s-1.2 6.4-3.6 9c-2.4-2.6-3.6-5.7-3.6-9S9.6 5.6 12 3Z" />
      </svg>
    ),
  },
  {
    label: "Mobile",
    description: "Android is real today over adb — tap, fill, and read a live app. iOS isn't wired up yet.",
    status: "Android available",
    position: { left: "94%", top: "10%" },
    icon: (
      <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6">
        <rect x="7" y="2.5" width="10" height="19" rx="2.2" />
        <path d="M11 18.2h2" strokeLinecap="round" />
      </svg>
    ),
  },
  {
    label: "Desktop",
    description: "Full application automation for the tools your business runs on.",
    status: "Not yet available",
    position: { left: "6%", top: "90%" },
    icon: (
      <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6">
        <rect x="2.5" y="4" width="19" height="13" rx="1.8" />
        <path d="M8 21h8M12 17v4" strokeLinecap="round" />
      </svg>
    ),
  },
  {
    label: "API",
    description: "Backend checks written in the same test, alongside the same browser flow.",
    status: "Not yet available",
    position: { left: "94%", top: "90%" },
    icon: (
      <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6">
        <path d="M8 9 4.5 12.5 8 16M16 9l3.5 3.5L16 16M13.5 6.5l-3 11" strokeLinecap="round" strokeLinejoin="round" />
      </svg>
    ),
  },
];

const benefits = [
  {
    title: "À la carte, not a buffet",
    body: "The core stays small on its own. Each surface is a plugin you install on purpose, so you never carry the weight of automation you don't use.",
  },
  {
    title: "Built from the ground up",
    body: "No wrapped drivers, no glued-together tools. Every plugin runs on allwright's own engine, engineered to behave the way a real user does — so results stay dependable, not flaky.",
  },
  {
    title: "One workflow, every team",
    body: "Web, mobile, desktop, and API testers describe automation the same way, so knowledge and coverage carry across the whole product as each plugin ships.",
  },
];

const jsonLd = {
  "@context": "https://schema.org",
  "@type": "SoftwareApplication",
  name: SITE_NAME,
  alternateName: "Allwright",
  url: SITE_URL,
  description: SITE_DESCRIPTION,
  applicationCategory: "DeveloperApplication",
  operatingSystem: "Web, iOS, Android, Windows, macOS, Linux",
  image: `${SITE_URL}/opengraph-image`,
  sameAs: [GITHUB_URL],
  publisher: {
    "@type": "Organization",
    name: SITE_NAME,
    url: SITE_URL,
    logo: `${SITE_URL}/logo.svg`,
  },
};

export default function Home() {
  return (
    <div className="relative mx-auto w-full max-w-6xl pb-6">
      <script
        type="application/ld+json"
        // eslint-disable-next-line react/no-danger
        dangerouslySetInnerHTML={{ __html: JSON.stringify(jsonLd) }}
      />

      <section className="relative mx-auto mt-10 grid w-full place-items-center text-center sm:mt-16">
        <p className="animate-rise mb-5 inline-flex items-center gap-2 rounded-full border border-[var(--line)] bg-[var(--card)] px-4 py-1.5 font-mono text-[0.78rem] uppercase tracking-[0.14em] text-[var(--accent-2)]">
          <span className="h-1.5 w-1.5 rounded-full bg-[var(--accent)]" />
          Building in public — coming soon
        </p>

        <h1 className="animate-rise max-w-[18ch] text-[clamp(2.6rem,7vw,5.6rem)] leading-[1.02] font-semibold tracking-[-0.04em] text-[var(--ink)]">
          One automation engine{" "}
          <span className="bg-[linear-gradient(120deg,var(--accent),var(--accent-2))] bg-clip-text text-transparent">
            for everything you test.
          </span>
        </h1>

        <p className="animate-rise-delay mt-6 max-w-[52ch] text-[clamp(1.02rem,2vw,1.25rem)] leading-8 text-[var(--muted)]">
          Web, mobile, desktop, and API — allwright is one small core engine
          with an installable plugin for each surface. It&apos;s à la carte,
          not a buffet: your team learns one system, and installs only the
          plugin for what it&apos;s actually testing.
        </p>

        <p className="animate-rise-delay mt-4 font-mono text-[0.8rem] uppercase tracking-[0.1em] text-[var(--accent-2)]">
          Built from the ground up — no wrapped drivers, no glued-together tools.
        </p>

        <div className="animate-rise-delay mt-9 flex flex-wrap items-center justify-center gap-3">
          <a
            href={GITHUB_URL}
            target="_blank"
            rel="noreferrer"
            className="inline-flex items-center rounded-full bg-[linear-gradient(120deg,var(--accent),var(--accent-2))] px-6 py-3 text-sm font-semibold text-white shadow-[0_18px_40px_var(--accent-soft)] transition hover:-translate-y-0.5"
          >
            Follow the project
          </a>
          <Link
            href="/how-it-works"
            className="inline-flex items-center rounded-full border border-[var(--line)] bg-[var(--card)] px-6 py-3 text-sm font-medium text-[var(--ink)] transition hover:-translate-y-0.5 hover:border-[var(--accent-2)]"
          >
            See how it works
          </Link>
        </div>

        <div
          aria-hidden="true"
          className="animate-rise-delay relative mt-14 h-[220px] w-full max-w-2xl sm:mt-16 sm:h-[280px]"
        >
          <svg
            viewBox="0 0 100 100"
            preserveAspectRatio="none"
            role="img"
            aria-label="A small allwright core at the center, with plugin slots reaching out to web, mobile, desktop, and API automation"
            className="absolute inset-0 h-full w-full text-[var(--line)]"
          >
            <defs>
              <marker
                id="hero-arrow"
                viewBox="0 0 8 8"
                refX="6.5"
                refY="4"
                markerWidth="5"
                markerHeight="5"
                orient="auto-start-reverse"
              >
                <path d="M0,0 L8,4 L0,8 Z" fill="currentColor" />
              </marker>
            </defs>
            {surfaces.map((surface) => (
              <line
                key={surface.label}
                x1="50%"
                y1="50%"
                x2={surface.position.left}
                y2={surface.position.top}
                stroke="currentColor"
                strokeWidth="0.5"
                strokeDasharray={surface.status !== "Not yet available" ? undefined : "2 2"}
                markerEnd="url(#hero-arrow)"
              />
            ))}
          </svg>

          <div
            className="absolute flex h-16 w-16 -translate-x-1/2 -translate-y-1/2 items-center justify-center sm:h-20 sm:w-20"
            style={{ left: "50%", top: "50%" }}
          >
            <LogoMark size={64} />
          </div>

          {surfaces.map((surface) => (
            <div
              key={surface.label}
              className="absolute flex -translate-x-1/2 -translate-y-1/2 flex-col items-center gap-1.5"
              style={surface.position}
            >
              <span
                className={`flex h-12 w-12 items-center justify-center rounded-full border bg-[var(--card)] text-[var(--accent-2)] shadow-sm backdrop-blur-xl ${
                  surface.status !== "Not yet available" ? "border-[var(--line)]" : "border-dashed border-[var(--line)]"
                }`}
              >
                {surface.icon}
              </span>
              <span className="font-mono text-[0.7rem] uppercase tracking-[0.08em] text-[var(--muted)]">
                {surface.label}
              </span>
            </div>
          ))}
        </div>
      </section>

      <section
        id="surfaces"
        aria-label="what allwright automates"
        className="relative mx-auto mt-14 w-full sm:mt-16"
      >
        <div className="mx-auto max-w-2xl text-center">
          <p className="font-mono text-[0.8rem] uppercase tracking-[0.14em] text-[var(--accent-2)]">
            À la carte, not a buffet
          </p>
          <h2 className="mt-3 text-xl font-semibold text-[var(--ink)] sm:text-2xl">
            The core stays small. Every surface below is an optional plugin.
          </h2>
        </div>

        <div className="mt-8 grid w-full gap-4 sm:grid-cols-2 lg:grid-cols-4">
          {surfaces.map((surface) => (
            <article
              key={surface.label}
              className="animate-rise-delay rounded-[1.5rem] border border-[var(--line)] bg-[var(--card)] p-6 backdrop-blur-xl transition hover:-translate-y-1 hover:border-[var(--accent-2)]"
            >
              <div className="flex flex-wrap items-start justify-between gap-2">
                <span className="inline-flex h-11 w-11 items-center justify-center rounded-full bg-[var(--accent-soft)] text-[var(--accent-2)]">
                  {surface.icon}
                </span>
                <StatusPill status={surface.status} />
              </div>
              <h3 className="mt-4 text-lg font-semibold text-[var(--ink)]">
                {surface.label}
              </h3>
              <p className="mt-2 text-sm leading-6 text-[var(--muted)]">
                {surface.description}
              </p>
            </article>
          ))}
        </div>
        <p className="mx-auto mt-6 max-w-[52ch] text-center text-sm leading-6 text-[var(--muted)]">
          &ldquo;Available now&rdquo; means real and installable today, not
          feature-complete.{" "}
          <Link href="/availability" className="font-medium text-[var(--accent-2)] hover:underline">
            See the detailed breakdown →
          </Link>
        </p>
      </section>

      <section
        aria-label="why allwright"
        className="relative mx-auto mt-14 w-full rounded-[2rem] border border-[var(--line)] bg-[var(--card)] p-6 backdrop-blur-xl sm:mt-16 sm:p-10"
      >
        <p className="mb-8 text-center font-mono text-[0.8rem] uppercase tracking-[0.14em] text-[var(--accent-2)]">
          Why allwright
        </p>
        <div className="grid gap-8 text-center sm:grid-cols-3">
          {benefits.map((benefit) => (
            <div key={benefit.title}>
              <h3 className="text-base font-semibold text-[var(--ink)]">
                {benefit.title}
              </h3>
              <p className="mt-3 text-sm leading-6 text-[var(--muted)]">
                {benefit.body}
              </p>
            </div>
          ))}
        </div>
      </section>

      <section
        aria-label="learn more"
        className="relative mx-auto mt-14 flex w-full flex-col items-center gap-4 rounded-[2rem] border border-[var(--line)] bg-[var(--card)] p-8 text-center backdrop-blur-xl sm:mt-16"
      >
        <h2 className="text-xl font-semibold text-[var(--ink)] sm:text-2xl">
          Curious how the plugin model actually works?
        </h2>
        <p className="max-w-[46ch] text-sm leading-6 text-[var(--muted)]">
          See the client languages allwright speaks today and exactly which
          plugins are installable now versus still on the way.
        </p>
        <div className="flex flex-wrap items-center justify-center gap-3">
          <Link
            href="/how-it-works"
            className="inline-flex items-center rounded-full bg-[linear-gradient(120deg,var(--accent),var(--accent-2))] px-6 py-3 text-sm font-semibold text-white shadow-[0_18px_40px_var(--accent-soft)] transition hover:-translate-y-0.5"
          >
            See how it works
          </Link>
          <Link
            href="/availability"
            className="inline-flex items-center rounded-full border border-[var(--line)] bg-[var(--card)] px-6 py-3 text-sm font-medium text-[var(--ink)] transition hover:-translate-y-0.5 hover:border-[var(--accent-2)]"
          >
            Full availability breakdown
          </Link>
        </div>
      </section>
    </div>
  );
}
