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

const INSPECTOR_SURFACES = [
  {
    name: "Android",
    needs: "Needs adb and an emulator or connected device.",
    commands: `allwright plugin install mobile-android
allwright inspector mobile-android ./sample.apk`,
    note: "Pass a local APK or a download URL. --device picks an emulator or serial.",
  },
  {
    name: "iOS",
    needs: "Needs a Mac with Xcode and a Simulator or registered iPhone.",
    commands: `allwright plugin install mobile-ios
allwright inspector mobile-ios ./Flights-simulator.ipa`,
    note: "Pass a .app, .zip, .ipa, or URL. --device picks a Simulator name or UDID.",
  },
  {
    name: "macOS",
    needs: "Needs macOS 14+, Xcode, and Accessibility permission.",
    commands: `allwright plugin install desktop-mac
allwright inspector desktop-mac com.apple.TextEdit`,
    note: "Pass the app's bundle identifier. --url opens a URL in the app.",
  },
  {
    name: "Windows",
    needs: "Needs Windows x64. Nothing else to install.",
    commands: `allwright plugin install desktop-windows
allwright inspector desktop-windows "C:\\Windows\\System32\\notepad.exe"`,
    note: "Pass an executable path or a command like notepad.exe.",
  },
];

const INIT_FLAGS = [
  { flag: "--yes / -y", does: "Accept defaults (TypeScript, Web) without prompting" },
  { flag: "--typescript / --javascript", does: "Skip the language prompt" },
  { flag: "--web / --mobile / --ios", does: "Pick a surface without the interactive prompt" },
  { flag: "--surface <web|mobile-android|mobile-ios>", does: "Repeat to scaffold any surface combination" },
  { flag: "--both", does: "Preserve the Web + Mobile Android shortcut" },
  { flag: "--all", does: "Scaffold Web, Mobile Android, and Mobile iOS together" },
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
            <p className="text-[var(--muted)]">✔ Web, Mobile Android, Mobile iOS — select any?</p>
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
            invoked it), and installs for you. The generated Web test runs
            immediately; mobile tests run once the selected emulator,
            simulator, or device is available.
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
|   |-- mobile.spec.ts
|   \`-- ios.spec.ts
|-- allwright.config.yaml
|-- vitest.config.ts
|-- tsconfig.json
|-- package.json
\`-- README.md`}
          </pre>
          <p className="mt-5 text-xs leading-6 text-[var(--muted)] sm:text-sm">
            <code className="rounded-md border border-[var(--line)] bg-[var(--card)] px-1.5 py-0.5 font-mono text-[0.85em] text-[var(--ink)]">web.spec.ts</code>,{" "}
            <code className="rounded-md border border-[var(--line)] bg-[var(--card)] px-1.5 py-0.5 font-mono text-[0.85em] text-[var(--ink)]">mobile.spec.ts</code>, and{" "}
            <code className="rounded-md border border-[var(--line)] bg-[var(--card)] px-1.5 py-0.5 font-mono text-[0.85em] text-[var(--ink)]">ios.spec.ts</code> are
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
            Example: <code className="rounded-md border border-[var(--line)] bg-[var(--card)] px-1.5 py-0.5 font-mono text-[0.85em] text-[var(--ink)]">npm init allwright@latest my-app -- --typescript --ios --no-install</code>.
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
            once you have a project scaffolded. For iOS, select Mobile iOS in
            the initializer; it writes the public Flights IPA configuration and
            a retrying <code className="font-mono text-[var(--ink)]">iosApp</code> test.
            Working in another language,
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

      <section aria-label="find locators with the inspector" className="mx-auto mt-14 w-full sm:mt-16">
        <div className="mx-auto max-w-2xl text-center">
          <h2 className="text-xl font-semibold text-[var(--ink)] sm:text-2xl">
            Not sure what to click? Start the inspector
          </h2>
          <p className="mt-3 text-sm leading-6 text-[var(--muted)] sm:text-base">
            With the <code className="font-mono">allwright</code> CLI installed,
            add the plugin for your surface, then start the inspector. It opens
            at <code className="font-mono">http://127.0.0.1:9227</code>; click an
            element in the screenshot and copy its locator in your language.
            Web pages aren&apos;t covered yet.
          </p>
        </div>
        <div className="mt-8 grid gap-4 sm:grid-cols-2">
          {INSPECTOR_SURFACES.map((surface) => (
            <div
              key={surface.name}
              className="rounded-[1.5rem] border border-[var(--line)] bg-[var(--card)] p-6 backdrop-blur-xl"
            >
              <h3 className="text-base font-semibold text-[var(--ink)]">{surface.name}</h3>
              <p className="mt-2 text-sm leading-6 text-[var(--muted)]">{surface.needs}</p>
              <pre className="mt-4 overflow-x-auto rounded-xl border border-[var(--line)] bg-[var(--background)]/60 p-3 font-mono text-xs leading-6 text-[var(--ink)]">
                {surface.commands}
              </pre>
              <p className="mt-3 text-xs leading-5 text-[var(--muted)]">{surface.note}</p>
            </div>
          ))}
        </div>
        <p className="mx-auto mt-6 max-w-[56ch] text-center text-sm leading-6 text-[var(--muted)]">
          Add <code className="font-mono">--no-open</code> to print the address
          instead of opening a browser. Use Interact mode to navigate to the
          screen you want, then switch back to Inspect to pick an element.{" "}
          <Link href="/blog/allwright-inspector-find-locators" className="font-medium text-[var(--accent-2)] hover:underline">
            Full walkthrough →
          </Link>
        </p>
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
