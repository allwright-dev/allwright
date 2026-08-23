import Image from "next/image";

import { GITHUB_URL, LogoMark, SITE_DESCRIPTION, SITE_NAME, SITE_URL } from "./brand";
import { ThemeToggle } from "./theme-toggle";

const surfaces = [
  {
    label: "Web",
    description: "Real browser flows that click, type, and navigate like a person would.",
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
    description: "The same test logic driving native and hybrid apps on real devices.",
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
    description: "Backend checks that stay in sync with the same flows and data.",
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
    title: "One engine, not a pile of tools",
    body: "Every surface you test runs on the same engine, so your team learns one system instead of juggling a different framework for every platform.",
  },
  {
    title: "Built for steady, trustworthy runs",
    body: "Allwright is engineered to behave the way a real user does, so results stay dependable instead of failing on things that were never actually broken.",
  },
  {
    title: "One workflow, every team",
    body: "Web, mobile, desktop, and API testers describe automation the same way, so knowledge and coverage carry across the whole product.",
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
    <main className="relative min-h-screen overflow-hidden bg-[radial-gradient(circle_at_top_left,var(--accent-soft),transparent_38%),radial-gradient(circle_at_85%_15%,var(--accent-2-soft),transparent_32%),linear-gradient(180deg,var(--background)_0%,var(--background-deep)_100%)] px-4 py-6 sm:px-6 sm:py-8">
      <script
        type="application/ld+json"
        // eslint-disable-next-line react/no-danger
        dangerouslySetInnerHTML={{ __html: JSON.stringify(jsonLd) }}
      />
      <div className="grid-overlay pointer-events-none absolute inset-0 opacity-40" />
      <div className="ambient-left absolute left-[-10%] top-[6%] h-[24rem] w-[24rem] rounded-full bg-[radial-gradient(circle,var(--accent-soft),transparent_68%)] blur-md" />
      <div className="ambient-right absolute bottom-[-4%] right-[-10%] h-[26rem] w-[26rem] rounded-full bg-[radial-gradient(circle,var(--accent-2-soft),transparent_68%)] blur-md" />

      <div className="relative mx-auto flex w-full max-w-6xl items-center justify-between">
        <a href="#" className="flex items-center gap-2.5">
          <Image src="/logo.svg" alt="" width={28} height={28} priority className="rounded-[8px]" />
          <span className="font-mono text-[0.95rem] font-medium tracking-[-0.02em] text-[var(--ink)]">
            allwright
          </span>
        </a>
        <div className="flex items-center gap-3">
          <a
            href={GITHUB_URL}
            target="_blank"
            rel="noreferrer"
            className="hidden items-center rounded-full border border-[var(--line)] bg-[var(--card)] px-4 py-2 text-sm font-medium text-[var(--ink)] transition hover:-translate-y-0.5 hover:border-[var(--accent-2)] sm:inline-flex"
          >
            Star on GitHub
          </a>
          <ThemeToggle />
        </div>
      </div>

      <section className="relative mx-auto mt-10 grid w-full max-w-6xl place-items-center text-center sm:mt-16">
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
          Web, mobile, desktop, and API — allwright brings every kind of test
          automation under one roof, so your team ships with one reliable
          engine instead of stitching together a different tool for each
          surface.
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
          <a
            href="#surfaces"
            className="inline-flex items-center rounded-full border border-[var(--line)] bg-[var(--card)] px-6 py-3 text-sm font-medium text-[var(--ink)] transition hover:-translate-y-0.5 hover:border-[var(--accent-2)]"
          >
            See what it covers
          </a>
        </div>

        <div
          aria-hidden="true"
          className="animate-rise-delay relative mt-14 h-[220px] w-full max-w-2xl sm:mt-16 sm:h-[280px]"
        >
          <svg
            viewBox="0 0 100 100"
            preserveAspectRatio="none"
            className="absolute inset-0 h-full w-full text-[var(--line)]"
          >
            {surfaces.map((surface) => (
              <line
                key={surface.label}
                x1={surface.position.left}
                y1={surface.position.top}
                x2="50%"
                y2="50%"
                stroke="currentColor"
                strokeWidth="0.5"
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
              <span className="flex h-12 w-12 items-center justify-center rounded-full border border-[var(--line)] bg-[var(--card)] text-[var(--accent-2)] shadow-sm backdrop-blur-xl">
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
        className="relative mx-auto mt-10 grid w-full max-w-6xl gap-4 sm:mt-14 sm:grid-cols-2 lg:grid-cols-4"
      >
        {surfaces.map((surface) => (
          <article
            key={surface.label}
            className="animate-rise-delay rounded-[1.5rem] border border-[var(--line)] bg-[var(--card)] p-6 backdrop-blur-xl transition hover:-translate-y-1 hover:border-[var(--accent-2)]"
          >
            <span className="mb-4 inline-flex h-11 w-11 items-center justify-center rounded-full bg-[var(--accent-soft)] text-[var(--accent-2)]">
              {surface.icon}
            </span>
            <h2 className="text-lg font-semibold text-[var(--ink)]">
              {surface.label}
            </h2>
            <p className="mt-2 text-sm leading-6 text-[var(--muted)]">
              {surface.description}
            </p>
          </article>
        ))}
      </section>

      <section
        aria-label="why allwright"
        className="relative mx-auto mt-14 w-full max-w-6xl rounded-[2rem] border border-[var(--line)] bg-[var(--card)] p-6 backdrop-blur-xl sm:mt-16 sm:p-10"
      >
        <p className="mb-8 max-w-[46ch] font-mono text-[0.8rem] uppercase tracking-[0.14em] text-[var(--accent-2)]">
          Why allwright
        </p>
        <div className="grid gap-8 sm:grid-cols-3">
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

      <footer className="relative mx-auto mt-14 flex w-full max-w-6xl flex-col items-center gap-3 pb-4 text-center sm:mt-16 sm:flex-row sm:justify-between sm:text-left">
        <p className="text-sm text-[var(--muted)]">
          allwright — one engine, all right.
        </p>
        <a
          href={GITHUB_URL}
          target="_blank"
          rel="noreferrer"
          className="text-sm font-medium text-[var(--accent-2)] underline-offset-4 hover:underline"
        >
          Watch our progress on GitHub
        </a>
      </footer>
    </main>
  );
}
