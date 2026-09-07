import type { Metadata } from "next";
import Link from "next/link";

import { GITHUB_URL, SITE_NAME } from "../brand";

const description =
  "Go from an empty folder to a passing allwright test in one command: npm init allwright@latest. Config, a starter test, and installed dependencies, or wire allwright into an existing project by hand.";

export const metadata: Metadata = {
  title: "Quickstart",
  description,
  alternates: { canonical: "/quickstart" },
  openGraph: {
    type: "website",
    url: "/quickstart",
    siteName: SITE_NAME,
    locale: "en_US",
    title: "Quickstart: one command to a passing test",
    description,
  },
  twitter: {
    card: "summary_large_image",
    title: "Quickstart: one command to a passing test",
    description,
  },
};

const INIT_FLAGS = [
  { flag: "--yes / -y", does: "Accept defaults (TypeScript, Web) without prompting" },
  { flag: "--typescript / --javascript", does: "Skip the language prompt" },
  { flag: "--web / --mobile / --both", does: "Pick which surfaces to scaffold tests for" },
  { flag: "--package-manager <bun|npm|pnpm|yarn>", does: "Override auto-detection" },
  { flag: "--no-install", does: "Scaffold without installing dependencies" },
];

const HAND_WRITTEN_LANGUAGES = [
  { name: "Rust", href: `${GITHUB_URL}/blob/main/rust/allwright/examples/playground.rs` },
  { name: "Go", href: `${GITHUB_URL}/tree/main/go/examples/playground` },
  { name: "Java", href: `${GITHUB_URL}/tree/main/java` },
  { name: "Python", href: `${GITHUB_URL}/tree/main/python` },
];

export default function Quickstart() {
  return (
    <div className="relative mx-auto w-full max-w-5xl pb-6">
      <section className="mx-auto mt-10 max-w-3xl text-center sm:mt-14">
        <p className="mb-5 inline-flex items-center gap-2 rounded-full border border-[var(--line)] bg-[var(--card)] px-4 py-1.5 font-mono text-[0.78rem] uppercase tracking-[0.14em] text-[var(--accent-2)]">
          Quickstart
        </p>
        <h1 className="text-[clamp(2.2rem,5vw,3.4rem)] leading-[1.05] font-semibold tracking-[-0.03em] text-[var(--ink)]">
          One command to a passing test.
        </h1>
        <p className="mt-5 text-[clamp(1rem,1.6vw,1.15rem)] leading-8 text-[var(--muted)]">
          <code className="rounded-md border border-[var(--line)] bg-[var(--card)] px-1.5 py-0.5 font-mono text-[0.85em] text-[var(--ink)]">
            npm init allwright@latest
          </code>{" "}
          scaffolds a working TypeScript or JavaScript project — config, a
          starter test for the surfaces you pick, dependencies installed —
          instead of assembling one by hand.
        </p>
      </section>

      <section aria-label="the fast path" className="mx-auto mt-14 w-full sm:mt-16">
        <div className="rounded-[2rem] border border-[var(--line)] bg-[var(--card)] p-6 backdrop-blur-xl sm:p-10">
          <div className="rounded-2xl border border-[var(--line)] bg-[var(--background)]/60 p-5 font-mono text-xs leading-6 text-[var(--muted)] sm:text-sm">
            <p className="text-[var(--accent-2)]">$ npm init allwright@latest my-app</p>
            <p className="text-[var(--muted)]">✔ TypeScript or JavaScript?</p>
            <p className="text-[var(--muted)]">✔ Web, Mobile Android, or both?</p>
            <p className="text-[var(--ink)]">✔ scaffolded, dependencies installed</p>
            <p className="mt-3 text-[var(--accent-2)]">$ cd my-app</p>
            <p className="text-[var(--accent-2)]">$ npm test</p>
            <p className="text-[var(--ink)]">✔ 1 passed</p>
          </div>
          <p className="mt-6 text-sm leading-6 text-[var(--muted)] sm:text-base">
            It asks two questions — language, and which surfaces to test —
            then writes a real, runnable project against the same public demo
            targets used throughout our guides, detects your package manager
            from an existing lockfile (or falls back to whichever manager
            invoked it), and installs for you. <code className="rounded-md border border-[var(--line)] bg-[var(--card)] px-1.5 py-0.5 font-mono text-[0.85em] text-[var(--ink)]">npm test</code> passes
            immediately, before you change a line.
          </p>
        </div>
      </section>

      <section aria-label="what it scaffolds" className="mx-auto mt-14 w-full sm:mt-16">
        <div className="mx-auto max-w-2xl text-center">
          <h2 className="text-xl font-semibold text-[var(--ink)] sm:text-2xl">
            What lands in the folder
          </h2>
        </div>
        <div className="mt-8 rounded-[2rem] border border-[var(--line)] bg-[var(--card)] p-6 backdrop-blur-xl sm:p-10">
          <pre className="overflow-x-auto rounded-2xl border border-[var(--line)] bg-[var(--background)]/60 p-5 font-mono text-xs leading-6 text-[var(--muted)] sm:text-sm">
{`.
|-- tests/
|   |-- web.spec.ts
|   \`-- mobile.spec.ts
|-- allwright.config.yaml
|-- vitest.config.ts
|-- tsconfig.json
|-- package.json
\`-- README.md`}
          </pre>
          <p className="mt-5 text-xs leading-6 text-[var(--muted)] sm:text-sm">
            <code className="rounded-md border border-[var(--line)] bg-[var(--card)] px-1.5 py-0.5 font-mono text-[0.85em] text-[var(--ink)]">web.spec.ts</code> and{" "}
            <code className="rounded-md border border-[var(--line)] bg-[var(--card)] px-1.5 py-0.5 font-mono text-[0.85em] text-[var(--ink)]">mobile.spec.ts</code> are
            included based on which surfaces you pick;{" "}
            <code className="rounded-md border border-[var(--line)] bg-[var(--card)] px-1.5 py-0.5 font-mono text-[0.85em] text-[var(--ink)]">tsconfig.json</code> only
            for TypeScript projects.
          </p>
        </div>
      </section>

      <section aria-label="non-interactive use" className="mx-auto mt-14 w-full sm:mt-16">
        <div className="mx-auto max-w-2xl text-center">
          <h2 className="text-xl font-semibold text-[var(--ink)] sm:text-2xl">
            Scripting it, or wiring it into CI
          </h2>
          <p className="mt-3 text-sm leading-6 text-[var(--muted)] sm:text-base">
            Every prompt has a matching flag, so CI and scripts can skip the
            interactive session entirely.
          </p>
        </div>
        <div className="mt-8 overflow-x-auto rounded-[2rem] border border-[var(--line)] bg-[var(--card)] p-6 backdrop-blur-xl sm:p-10">
          <div className="overflow-x-auto rounded-xl border border-[var(--line)]">
            <table className="w-full border-collapse text-sm">
              <thead className="bg-[var(--background)]/60">
                <tr>
                  <th className="border-b border-[var(--line)] px-4 py-2 text-left font-semibold text-[var(--ink)]">
                    Flag
                  </th>
                  <th className="border-b border-[var(--line)] px-4 py-2 text-left font-semibold text-[var(--ink)]">
                    Does
                  </th>
                </tr>
              </thead>
              <tbody>
                {INIT_FLAGS.map((row) => (
                  <tr key={row.flag}>
                    <td className="border-b border-[var(--line)] px-4 py-2 align-top font-mono text-xs text-[var(--ink)] sm:text-sm">
                      {row.flag}
                    </td>
                    <td className="border-b border-[var(--line)] px-4 py-2 align-top text-[var(--muted)]">
                      {row.does}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          <p className="mt-5 text-sm leading-6 text-[var(--muted)]">
            Example: <code className="rounded-md border border-[var(--line)] bg-[var(--card)] px-1.5 py-0.5 font-mono text-[0.85em] text-[var(--ink)]">npm init allwright@latest my-app -- --typescript --web --no-install</code>.
            See the{" "}
            <a
              href={`${GITHUB_URL}/tree/main/typescript/create`}
              target="_blank"
              rel="noreferrer"
              className="font-medium text-[var(--accent-2)] hover:underline"
            >
              create-allwright README
            </a>{" "}
            for the full reference.
          </p>
        </div>
      </section>

      <section aria-label="building it by hand" className="mx-auto mt-14 w-full sm:mt-16">
        <div className="mx-auto max-w-2xl text-center">
          <h2 className="text-xl font-semibold text-[var(--ink)] sm:text-2xl">
            Prefer to build it up yourself?
          </h2>
        </div>
        <div className="mt-8 rounded-[2rem] border border-[var(--line)] bg-[var(--card)] p-6 backdrop-blur-xl sm:p-10">
          <p className="text-sm leading-7 text-[var(--muted)] sm:text-base">
            The initializer is for the blank-folder case. If you&apos;re wiring
            allwright into an existing TypeScript or JavaScript project, our{" "}
            <Link href="/blog/get-started-with-typescript" className="font-medium text-[var(--accent-2)] hover:underline">
              getting-started guide
            </Link>{" "}
            walks through the same project by hand — install{" "}
            <code className="rounded-md border border-[var(--line)] bg-[var(--card)] px-1.5 py-0.5 font-mono text-[0.85em] text-[var(--ink)]">@allwright.dev/vitest</code>,
            write a config, write a test, run it — so you understand what each
            generated file is actually doing.
          </p>
          <p className="mt-4 text-sm leading-7 text-[var(--muted)] sm:text-base">
            Testing Android? Jump to the{" "}
            <Link href="/blog/android-testing-playwright-style" className="font-medium text-[var(--accent-2)] hover:underline">
              Android testing walkthrough
            </Link>{" "}
            once you have a project scaffolded. Working in another language,
            allwright&apos;s client libraries are published and ready to install
            directly:
          </p>
          <div className="mt-5 flex flex-wrap items-center justify-center gap-3">
            {HAND_WRITTEN_LANGUAGES.map((lang) => (
              <a
                key={lang.name}
                href={lang.href}
                target="_blank"
                rel="noreferrer"
                className="inline-flex items-center rounded-full border border-[var(--line)] bg-[var(--card)] px-4 py-2 font-mono text-sm font-medium text-[var(--ink)] transition hover:-translate-y-0.5 hover:border-[var(--accent-2)]"
              >
                {lang.name}
              </a>
            ))}
          </div>
        </div>
      </section>

      <section
        aria-label="get started"
        className="mx-auto mt-14 flex w-full flex-col items-center gap-4 rounded-[2rem] border border-[var(--line)] bg-[var(--card)] p-8 text-center backdrop-blur-xl sm:mt-16"
      >
        <h2 className="text-xl font-semibold text-[var(--ink)] sm:text-2xl">
          Try it now
        </h2>
        <p className="max-w-[46ch] text-sm leading-6 text-[var(--muted)]">
          Run the command, see what lands in a real folder, then check{" "}
          <Link href="/availability" className="font-medium text-[var(--accent-2)] hover:underline">
            Availability
          </Link>{" "}
          for exactly what today&apos;s command set covers.
        </p>
        <div className="flex flex-wrap items-center justify-center gap-3">
          <a
            href={`${GITHUB_URL}/tree/main/typescript/create`}
            target="_blank"
            rel="noreferrer"
            className="inline-flex items-center rounded-full bg-[linear-gradient(120deg,var(--accent),var(--accent-2))] px-6 py-3 text-sm font-semibold text-white shadow-[0_18px_40px_var(--accent-soft)] transition hover:-translate-y-0.5"
          >
            View create-allwright on GitHub
          </a>
          <Link
            href="/blog/npm-init-allwright"
            className="inline-flex items-center rounded-full border border-[var(--line)] bg-[var(--card)] px-6 py-3 text-sm font-medium text-[var(--ink)] transition hover:-translate-y-0.5 hover:border-[var(--accent-2)]"
          >
            Read the full walkthrough
          </Link>
        </div>
      </section>
    </div>
  );
}
