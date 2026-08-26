import type { Metadata } from "next";
import Link from "next/link";

import { languages, surfaceStatus } from "../availability-data";
import { GITHUB_URL } from "../brand";
import { StatusPill } from "../status-pill";

export const metadata: Metadata = {
  title: "How it works",
  description:
    "allwright is a small core engine plus installable plugins for web, mobile, desktop, and API testing — à la carte, not a buffet. See what's installable today and which client languages are published.",
};

// The real plugin catalog: one entry per installable surface plugin, laid
// out around the core in a hexagon. Web is the only one installable today —
// everything else already has a reserved slot but ships nothing yet.
const pluginCatalog = [
  { label: "Web", center: { x: 240, y: 60 }, status: "Available now" as const },
  { label: "Mobile — Android", center: { x: 370, y: 135 }, status: "Not yet available" as const },
  { label: "Mobile — iOS", center: { x: 370, y: 285 }, status: "Not yet available" as const },
  { label: "Desktop — Windows", center: { x: 240, y: 360 }, status: "Not yet available" as const },
  { label: "Desktop — Linux", center: { x: 110, y: 285 }, status: "Not yet available" as const },
  { label: "Desktop — macOS", center: { x: 110, y: 135 }, status: "Not yet available" as const },
];

const CORE = { x: 240, y: 210, r: 48 };
const BOX = { w: 130, h: 56 };

function ArrowMarker({ id }: { id: string }) {
  return (
    <marker id={id} viewBox="0 0 8 8" refX="6.5" refY="4" markerWidth="6" markerHeight="6" orient="auto-start-reverse">
      <path d="M0,0 L8,4 L0,8 Z" fill="currentColor" />
    </marker>
  );
}

function WithoutEngineDiagram() {
  const boxes = ["Web tool", "Mobile tool", "Desktop tool", "API tool"];
  const ys = [10, 70, 130, 190];

  return (
    <figure className="flex flex-col items-center">
      <svg
        viewBox="0 0 260 240"
        role="img"
        aria-label="Four separate, disconnected tools for web, mobile, desktop, and API tooling, each learned and maintained on its own"
        className="h-auto w-full max-w-[280px] text-[var(--line)]"
      >
        {boxes.map((label, i) => (
          <g key={label}>
            <rect
              x={40}
              y={ys[i]}
              width={180}
              height={40}
              rx={10}
              fill="var(--card)"
              stroke="currentColor"
              strokeWidth="1"
              strokeDasharray="4 3"
            />
            <text x={130} y={ys[i] + 25} textAnchor="middle" fontSize="12" fill="var(--muted)">
              {label}
            </text>
          </g>
        ))}
      </svg>
      <figcaption className="mt-3 max-w-[26ch] text-center text-sm leading-6 text-[var(--muted)]">
        <span className="font-medium text-[var(--ink)]">Without one engine:</span>{" "}
        four unrelated tools, four things to learn, four ways to maintain.
      </figcaption>
    </figure>
  );
}

function WithEngineDiagram() {
  const ys = [10, 70, 130, 190];

  return (
    <figure className="flex flex-col items-center">
      <svg
        viewBox="0 0 260 240"
        role="img"
        aria-label="One small allwright core in the center, reaching web with a solid line for an installed plugin, and mobile, desktop, and API with dashed lines for not-yet-available plugins"
        className="h-auto w-full max-w-[280px] text-[var(--line)]"
      >
        <defs>
          <ArrowMarker id="hiw-with-arrow" />
        </defs>
        {surfaceStatus.map((surface, i) => {
          const boxCenterY = ys[i] + 20;
          const ready = surface.status === "Available now";
          return (
            <line
              key={surface.label}
              x1={52 + 30}
              y1={120}
              x2={150}
              y2={boxCenterY}
              stroke="currentColor"
              strokeWidth="1"
              strokeDasharray={ready ? undefined : "3 3"}
              markerEnd="url(#hiw-with-arrow)"
            />
          );
        })}
        <circle cx={52} cy={120} r={30} fill="var(--accent-soft)" stroke="var(--accent-2)" strokeWidth="1.4" />
        <text x={52} y={124} textAnchor="middle" fontSize="10.5" fontWeight="600" fill="var(--accent-2)">
          core
        </text>
        {surfaceStatus.map((surface, i) => {
          const ready = surface.status === "Available now";
          return (
            <g key={surface.label}>
              <rect
                x={152}
                y={ys[i]}
                width={90}
                height={40}
                rx={10}
                fill="var(--card)"
                stroke="currentColor"
                strokeWidth="1"
                strokeDasharray={ready ? undefined : "3 3"}
              />
              <text x={197} y={ys[i] + 25} textAnchor="middle" fontSize="12" fill="var(--ink)">
                {surface.label}
              </text>
            </g>
          );
        })}
      </svg>
      <figcaption className="mt-3 max-w-[30ch] text-center text-sm leading-6 text-[var(--muted)]">
        <span className="font-medium text-[var(--ink)]">With allwright:</span>{" "}
        one core, one set of commands — solid for the plugin that&apos;s
        installed, dashed for the ones still to come.
      </figcaption>
    </figure>
  );
}

function PluginCatalogDiagram() {
  return (
    <figure className="flex flex-col items-center">
      <svg
        viewBox="0 0 480 420"
        role="img"
        aria-label="The allwright core in the center with today's plugin slots around it: Web is filled in and installed, while Mobile Android, Mobile iOS, Desktop Windows, Desktop Linux, and Desktop macOS are outlined as reserved but not yet installable."
        className="h-auto w-full max-w-md text-[var(--line)]"
      >
        <defs>
          <ArrowMarker id="hiw-catalog-arrow" />
        </defs>

        {pluginCatalog.map((plugin) => {
          const ready = plugin.status === "Available now";
          const dx = plugin.center.x - CORE.x;
          const dy = plugin.center.y - CORE.y;
          const dist = Math.sqrt(dx * dx + dy * dy);
          const ux = dx / dist;
          const uy = dy / dist;
          const start = { x: CORE.x + ux * CORE.r, y: CORE.y + uy * CORE.r };
          const end = { x: plugin.center.x - ux * 35, y: plugin.center.y - uy * 35 };
          return (
            <line
              key={plugin.label}
              x1={start.x}
              y1={start.y}
              x2={end.x}
              y2={end.y}
              stroke="currentColor"
              strokeWidth="1.2"
              strokeDasharray={ready ? undefined : "3 3"}
              markerEnd="url(#hiw-catalog-arrow)"
            />
          );
        })}

        <circle cx={CORE.x} cy={CORE.y} r={CORE.r} fill="var(--accent-soft)" stroke="var(--accent-2)" strokeWidth="1.6" />
        <text x={CORE.x} y={CORE.y - 3} textAnchor="middle" fontSize="12.5" fontWeight="600" fill="var(--accent-2)">
          allwright
        </text>
        <text x={CORE.x} y={CORE.y + 13} textAnchor="middle" fontSize="10" fill="var(--accent-2)">
          core
        </text>

        {pluginCatalog.map((plugin) => {
          const ready = plugin.status === "Available now";
          const x = plugin.center.x - BOX.w / 2;
          const y = plugin.center.y - BOX.h / 2;
          return (
            <g key={plugin.label}>
              <rect
                x={x}
                y={y}
                width={BOX.w}
                height={BOX.h}
                rx={12}
                fill="var(--card)"
                stroke="currentColor"
                strokeWidth={ready ? 1.4 : 1}
                strokeDasharray={ready ? undefined : "3 3"}
              />
              <text x={plugin.center.x} y={plugin.center.y - 4} textAnchor="middle" fontSize="12" fontWeight="600" fill="var(--ink)">
                {plugin.label}
              </text>
              <text
                x={plugin.center.x}
                y={plugin.center.y + 14}
                textAnchor="middle"
                fontSize="9.5"
                fill={ready ? "var(--accent)" : "var(--muted)"}
              >
                {ready ? "● installed" : "○ in the catalog"}
              </text>
            </g>
          );
        })}
      </svg>
      <figcaption className="mt-4 max-w-[46ch] text-center text-sm leading-6 text-[var(--muted)]">
        Web is installed and ready; every other slot is already reserved,
        waiting on a real runtime build.
      </figcaption>
    </figure>
  );
}

function ClientFlowDiagram() {
  const langYs = [35, 73, 111, 149, 187];
  const surfaceYs = [15, 75, 135, 195];
  const hub = { cx: 380, cy: 130, r: 42 };

  return (
    <figure className="flex flex-col items-center">
      <svg
        viewBox="0 0 760 270"
        role="img"
        aria-label="Rust, Go, Java, Python, and TypeScript client libraries all converging into one small allwright core, which fans back out to Web with a solid line, and Mobile, Desktop, and API with dashed lines for plugins that aren't installable yet"
        className="h-auto w-full max-w-3xl text-[var(--line)]"
      >
        <defs>
          <ArrowMarker id="hiw-flow-arrow" />
        </defs>

        {languages.map((lang, i) => (
          <line
            key={lang.name}
            x1={160}
            y1={langYs[i] + 15}
            x2={hub.cx - hub.r}
            y2={hub.cy}
            stroke="currentColor"
            strokeWidth="1"
            markerEnd="url(#hiw-flow-arrow)"
          />
        ))}

        {surfaceStatus.map((surface, i) => {
          const ready = surface.status === "Available now";
          return (
            <line
              key={surface.label}
              x1={hub.cx + hub.r}
              y1={hub.cy}
              x2={600}
              y2={surfaceYs[i] + 25}
              stroke="currentColor"
              strokeWidth="1"
              strokeDasharray={ready ? undefined : "3 3"}
              markerEnd="url(#hiw-flow-arrow)"
            />
          );
        })}

        <text x={110} y={14} textAnchor="middle" fontSize="11.5" fill="var(--muted)">
          any client library
        </text>
        <text x={620} y={14} textAnchor="middle" fontSize="11.5" fill="var(--muted)">
          one plugin per surface
        </text>

        {languages.map((lang, i) => (
          <g key={lang.name}>
            <rect
              x={20}
              y={langYs[i]}
              width={140}
              height={30}
              rx={15}
              fill="var(--card)"
              stroke="currentColor"
              strokeWidth="1"
            />
            <text x={90} y={langYs[i] + 20} textAnchor="middle" fontSize="12" fill="var(--ink)">
              {lang.name}
            </text>
          </g>
        ))}

        <circle cx={hub.cx} cy={hub.cy} r={hub.r} fill="var(--accent-soft)" stroke="var(--accent-2)" strokeWidth="1.6" />
        <text x={hub.cx} y={hub.cy - 2} textAnchor="middle" fontSize="13" fontWeight="600" fill="var(--accent-2)">
          allwright
        </text>
        <text x={hub.cx} y={hub.cy + 14} textAnchor="middle" fontSize="10" fill="var(--accent-2)">
          core
        </text>

        {surfaceStatus.map((surface, i) => {
          const ready = surface.status === "Available now";
          return (
            <g key={surface.label}>
              <rect
                x={600}
                y={surfaceYs[i]}
                width={140}
                height={50}
                rx={12}
                fill="var(--card)"
                stroke="currentColor"
                strokeWidth={ready ? 1.4 : 1}
                strokeDasharray={ready ? undefined : "3 3"}
              />
              <text x={670} y={surfaceYs[i] + 21} textAnchor="middle" fontSize="12.5" fontWeight="600" fill="var(--ink)">
                {surface.label}
              </text>
              <text
                x={670}
                y={surfaceYs[i] + 38}
                textAnchor="middle"
                fontSize="9.5"
                fill={ready ? "var(--accent)" : "var(--muted)"}
              >
                {surface.status}
              </text>
            </g>
          );
        })}
      </svg>
      <figcaption className="mt-4 max-w-[56ch] text-center text-sm leading-6 text-[var(--muted)]">
        Pick the client library for your stack, describe automation once,
        and the same core carries it to whichever plugin is installed.
      </figcaption>
    </figure>
  );
}

export default function HowItWorks() {
  return (
    <div className="relative mx-auto w-full max-w-6xl pb-6">
      <section className="mx-auto mt-10 max-w-3xl text-center sm:mt-14">
        <p className="mb-5 inline-flex items-center gap-2 rounded-full border border-[var(--line)] bg-[var(--card)] px-4 py-1.5 font-mono text-[0.78rem] uppercase tracking-[0.14em] text-[var(--accent-2)]">
          How it works
        </p>
        <h1 className="text-[clamp(2.2rem,5vw,3.4rem)] leading-[1.05] font-semibold tracking-[-0.03em] text-[var(--ink)]">
          À la carte, not a buffet.
        </h1>
        <p className="mt-5 text-[clamp(1rem,1.6vw,1.15rem)] leading-8 text-[var(--muted)]">
          Allwright isn&apos;t a separate tool for every surface wearing one
          brand — and it isn&apos;t one giant program trying to do everything
          either. It&apos;s a small core plus a plugin per surface: install
          the plugin for what you&apos;re testing, and the core knows how to
          drive it. Leave a plugin uninstalled, and the core simply
          doesn&apos;t attempt that surface yet.
        </p>
      </section>

      <section aria-label="the plugin catalog" className="mx-auto mt-14 w-full sm:mt-16">
        <div className="mx-auto max-w-2xl text-center">
          <h2 className="text-xl font-semibold text-[var(--ink)] sm:text-2xl">
            One core, as many plugins as it needs
          </h2>
          <p className="mt-3 text-sm leading-6 text-[var(--muted)] sm:text-base">
            Every plugin is built from the ground up on allwright&apos;s own
            engine — not a wrapper around someone else&apos;s driver. Here&apos;s
            today&apos;s catalog; it grows as new surfaces ship.
          </p>
        </div>
        <div className="mt-8 rounded-[2rem] border border-[var(--line)] bg-[var(--card)] p-6 backdrop-blur-xl sm:p-10">
          <PluginCatalogDiagram />
          <div className="mx-auto mt-8 max-w-xl rounded-2xl border border-[var(--line)] bg-[var(--background)]/60 p-4 font-mono text-xs leading-6 text-[var(--muted)] sm:text-sm">
            <p className="text-[var(--ink)]">$ allwright plugin list</p>
            <p className="text-[var(--ink)]">$ allwright plugin install web</p>
          </div>
        </div>
      </section>

      <section
        aria-label="one engine versus many tools"
        className="mx-auto mt-14 grid w-full grid-cols-1 items-start gap-10 rounded-[2rem] border border-[var(--line)] bg-[var(--card)] p-8 backdrop-blur-xl sm:mt-16 sm:grid-cols-2 sm:gap-6 sm:p-10"
      >
        <WithoutEngineDiagram />
        <WithEngineDiagram />
      </section>

      <section aria-label="how requests flow" className="mx-auto mt-14 w-full sm:mt-16">
        <p className="mb-6 text-center font-mono text-[0.8rem] uppercase tracking-[0.14em] text-[var(--accent-2)]">
          From your test code to the core
        </p>
        <div className="rounded-[2rem] border border-[var(--line)] bg-[var(--card)] p-6 backdrop-blur-xl sm:p-10">
          <ClientFlowDiagram />
        </div>
      </section>

      <section aria-label="supported languages" className="mx-auto mt-14 w-full sm:mt-16">
        <div className="mx-auto max-w-2xl text-center">
          <h2 className="text-xl font-semibold text-[var(--ink)] sm:text-2xl">
            Bring your own language
          </h2>
          <p className="mt-3 text-sm leading-6 text-[var(--muted)] sm:text-base">
            Rust, Go, and TypeScript clients are published and ready to
            install. Java and Python clients are complete but not yet on a
            package registry — build them from source using the examples
            below.
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
                <h3 className="font-mono text-sm font-semibold text-[var(--ink)]">
                  {lang.name}
                </h3>
                <StatusPill status={lang.status} />
              </div>
              <p className="mt-2 text-xs leading-5 text-[var(--muted)]">
                {lang.note}
              </p>
              <span className="mt-auto inline-flex items-center gap-1 pt-3 text-xs font-medium text-[var(--accent-2)]">
                View example →
              </span>
            </a>
          ))}
        </div>
        <p className="mx-auto mt-6 max-w-[52ch] text-center text-sm leading-6 text-[var(--muted)]">
          Every client above speaks the same command set — nothing is
          language-exclusive.{" "}
          <Link href="/availability" className="font-medium text-[var(--accent-2)] hover:underline">
            See exactly what that command set covers today →
          </Link>
        </p>
      </section>

      <section aria-label="what the engine actually does" className="mx-auto mt-14 w-full max-w-3xl sm:mt-16">
        <h2 className="text-xl font-semibold text-[var(--ink)] sm:text-2xl">
          No drivers to babysit
        </h2>
        <p className="mt-4 text-sm leading-7 text-[var(--muted)] sm:text-base">
          Once you install the web plugin, allwright drives a real, current
          Chromium or Firefox browser directly instead of going through a
          separate driver binary you have to download and version-match by
          hand. That means fewer &ldquo;works on my machine&rdquo; surprises,
          and tests that behave the way an actual person clicking through
          your app would.
        </p>
        <p className="mt-4 text-sm leading-7 text-[var(--muted)] sm:text-base">
          Mobile, desktop, and API testing will work the same way once their
          plugins ship — no new tool to learn, just one more{" "}
          <code className="font-mono text-[var(--ink)]">plugin install</code>{" "}
          for whichever surface you need next.
        </p>
      </section>

      <section
        aria-label="get started"
        className="mx-auto mt-14 flex w-full flex-col items-center gap-4 rounded-[2rem] border border-[var(--line)] bg-[var(--card)] p-8 text-center backdrop-blur-xl sm:mt-16"
      >
        <h2 className="text-xl font-semibold text-[var(--ink)] sm:text-2xl">
          Follow along as it ships
        </h2>
        <p className="max-w-[46ch] text-sm leading-6 text-[var(--muted)]">
          allwright is building in public. Star the repo to track progress,
          or head back home for the quick pitch.
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
            href="/"
            className="inline-flex items-center rounded-full border border-[var(--line)] bg-[var(--card)] px-6 py-3 text-sm font-medium text-[var(--ink)] transition hover:-translate-y-0.5 hover:border-[var(--accent-2)]"
          >
            Back to home
          </Link>
        </div>
      </section>
    </div>
  );
}
