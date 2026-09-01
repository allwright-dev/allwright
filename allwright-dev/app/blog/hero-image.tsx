// Per-post hero banners. Drawn as inline SVG in the same hand-built style as
// the diagrams on /how-it-works (accent-gradient nodes, arrow markers, CSS
// variables for theming) instead of a raster image — it stays crisp at any
// size, adapts to light/dark automatically, and needs no image asset
// pipeline. Add a new function + registry entry per post; posts without one
// fall back to DefaultHero.

function ArrowMarker({ id }: { id: string }) {
  return (
    <marker id={id} viewBox="0 0 8 8" refX="6.5" refY="4" markerWidth="6" markerHeight="6" orient="auto-start-reverse">
      <path d="M0,0 L8,4 L0,8 Z" fill="currentColor" />
    </marker>
  );
}

type HeroVariant = "standalone" | "card";

// "standalone" is the full frame used at the top of a post page (its own
// border + radius on all corners). "card" is meant to sit flush against the
// top edge of an already-bordered, already-clipped container (the index
// page's post cards) — no border of its own, and only the top corners
// rounded, so it doesn't create a visible seam against the card below it.
const FRAME_VARIANT_CLASSES: Record<HeroVariant, string> = {
  standalone: "rounded-[1.5rem] border border-[var(--line)]",
  card: "rounded-none border-0",
};

function HeroFrame({
  children,
  label,
  variant = "standalone",
}: {
  children: React.ReactNode;
  label: string;
  variant?: HeroVariant;
}) {
  return (
    <div
      className={`relative aspect-[2.35/1] w-full overflow-hidden bg-[var(--card)] backdrop-blur-xl ${FRAME_VARIANT_CLASSES[variant]}`}
    >
      <div className="grid-overlay pointer-events-none absolute inset-0 opacity-30" />
      <svg viewBox="0 0 1200 460" role="img" aria-label={label} className="relative h-full w-full text-[var(--line)]">
        {children}
      </svg>
    </div>
  );
}

function TypeScriptHero({ variant }: { variant?: HeroVariant }) {
  return (
    <HeroFrame
      variant={variant}
      label="A TypeScript client node with an arrow into the allwright core, and a second arrow from the core into a real browser window, labelled with the @allwright.dev/vitest package and the page.goto, page.click, and expect calls that connect them"
    >
      <defs>
        <ArrowMarker id="hero-ts-arrow" />
      </defs>

      <line x1={230} y1={230} x2={528} y2={230} stroke="currentColor" strokeWidth="1.4" markerEnd="url(#hero-ts-arrow)" />
      <line x1={672} y1={230} x2={968} y2={230} stroke="currentColor" strokeWidth="1.4" markerEnd="url(#hero-ts-arrow)" />

      <text x={380} y={196} textAnchor="middle" fontSize="14" fontFamily="var(--font-mono)" fill="var(--muted)">
        @allwright.dev/vitest
      </text>
      <text x={820} y={196} textAnchor="middle" fontSize="14" fontFamily="var(--font-mono)" fill="var(--muted)">
        page.goto · page.click · expect
      </text>

      {/* TypeScript node */}
      <rect x={90} y={170} width={140} height={120} rx={24} fill="#3178C6" />
      <text x={160} y={246} textAnchor="middle" fontSize="52" fontWeight="700" fill="#ffffff" fontFamily="var(--font-mono)">
        TS
      </text>
      <text x={160} y={140} textAnchor="middle" fontSize="15" fontWeight="600" fill="var(--ink)">
        TypeScript client
      </text>

      {/* allwright core */}
      <circle cx={600} cy={230} r={72} fill="var(--accent-soft)" stroke="var(--accent-2)" strokeWidth="1.6" />
      <text x={600} y={224} textAnchor="middle" fontSize="16" fontWeight="600" fill="var(--accent-2)">
        allwright
      </text>
      <text x={600} y={246} textAnchor="middle" fontSize="13" fill="var(--accent-2)">
        core
      </text>

      {/* Browser node */}
      <rect x={968} y={170} width={172} height={120} rx={16} fill="var(--background)" stroke="currentColor" strokeWidth="1" />
      <rect x={968} y={170} width={172} height={30} rx={16} fill="var(--card)" stroke="currentColor" strokeWidth="1" />
      <circle cx={984} cy={185} r={4} fill="var(--muted)" opacity="0.5" />
      <circle cx={998} cy={185} r={4} fill="var(--muted)" opacity="0.5" />
      <circle cx={1012} cy={185} r={4} fill="var(--muted)" opacity="0.5" />
      <path
        d="M1054 234c0-16.6 13.4-30 30-30s30 13.4 30 30-13.4 30-30 30-30-13.4-30-30Z"
        fill="none"
        stroke="var(--accent)"
        strokeWidth="1.6"
      />
      <path d="M1054 234h60M1084 204v60" stroke="var(--accent)" strokeWidth="1.2" opacity="0.6" />
      <text x={1054} y={140} textAnchor="middle" fontSize="15" fontWeight="600" fill="var(--ink)">
        Real browser
      </text>
      <text x={1054} y={158} textAnchor="middle" fontSize="12" fill="var(--muted)">
        Chromium · Firefox
      </text>
    </HeroFrame>
  );
}

// Left panel: four disconnected, dashed tool boxes (the fragmented status
// quo). Right panel: the same four surfaces, but fanned out from one core:
// solid for the plugin that's actually installed today (web), dashed for
// the rest, matching the real install status from /how-it-works.
function EngineHero({ variant }: { variant?: HeroVariant }) {
  const rowYs = [96, 178, 260, 342];
  const rowHeight = 64;
  const surfaces = [
    { label: "Web", ready: true },
    { label: "Mobile", ready: false },
    { label: "Desktop", ready: false },
    { label: "API", ready: false },
  ];
  const core = { cx: 860, cy: 230, r: 70 };

  return (
    <HeroFrame
      variant={variant}
      label="Left: four separate, disconnected automation tools for web, mobile, desktop, and API, each learned and maintained on its own. Right: the same four surfaces reached from one allwright core, with a solid line to web for the plugin that's installed today and dashed lines to mobile, desktop, and API for plugins still to come."
    >
      <defs>
        <ArrowMarker id="hero-engine-arrow" />
      </defs>

      <text x={290} y={54} textAnchor="middle" fontSize="15" fontWeight="600" fill="var(--muted)">
        Without one engine
      </text>
      {rowYs.map((y, i) => (
        <g key={`left-${surfaces[i].label}`}>
          <rect
            x={60}
            y={y}
            width={460}
            height={rowHeight}
            rx={14}
            fill="var(--card)"
            stroke="currentColor"
            strokeWidth="1"
            strokeDasharray="4 3"
          />
          <text x={290} y={y + rowHeight / 2 + 5} textAnchor="middle" fontSize="15" fill="var(--muted)">
            {surfaces[i].label} tool
          </text>
        </g>
      ))}

      <line x1={600} y1={40} x2={600} y2={420} stroke="currentColor" strokeWidth="1" opacity="0.3" strokeDasharray="2 6" />

      <text x={890} y={54} textAnchor="middle" fontSize="15" fontWeight="600" fill="var(--muted)">
        One core, one plugin per surface
      </text>
      {surfaces.map((surface, i) => {
        const boxCenterY = rowYs[i] + rowHeight / 2;
        return (
          <line
            key={`line-${surface.label}`}
            x1={core.cx + core.r}
            y1={core.cy}
            x2={990}
            y2={boxCenterY}
            stroke="currentColor"
            strokeWidth="1.4"
            strokeDasharray={surface.ready ? undefined : "4 3"}
            markerEnd="url(#hero-engine-arrow)"
          />
        );
      })}
      <circle cx={core.cx} cy={core.cy} r={core.r} fill="var(--accent-soft)" stroke="var(--accent-2)" strokeWidth="1.6" />
      <text x={core.cx} y={core.cy - 4} textAnchor="middle" fontSize="16" fontWeight="600" fill="var(--accent-2)">
        allwright
      </text>
      <text x={core.cx} y={core.cy + 18} textAnchor="middle" fontSize="13" fill="var(--accent-2)">
        core
      </text>

      {surfaces.map((surface, i) => (
        <g key={`right-${surface.label}`}>
          <rect
            x={990}
            y={rowYs[i]}
            width={170}
            height={rowHeight}
            rx={14}
            fill="var(--card)"
            stroke="currentColor"
            strokeWidth={surface.ready ? 1.4 : 1}
            strokeDasharray={surface.ready ? undefined : "4 3"}
          />
          <text x={1075} y={rowYs[i] + rowHeight / 2 - 4} textAnchor="middle" fontSize="15" fontWeight="600" fill="var(--ink)">
            {surface.label}
          </text>
          <text
            x={1075}
            y={rowYs[i] + rowHeight / 2 + 15}
            textAnchor="middle"
            fontSize="10.5"
            fill={surface.ready ? "var(--accent)" : "var(--muted)"}
          >
            {surface.ready ? "● installed" : "○ not yet"}
          </text>
        </g>
      ))}
    </HeroFrame>
  );
}

// Same TS-client-into-core language as TypeScriptHero, but the core now fans
// out to two targets instead of one: the existing browser node, plus a new
// phone node for the androidApp fixture. That fan-out is the actual claim of
// the post — one client, one core, both surfaces in the same test.
function AndroidHero({ variant }: { variant?: HeroVariant }) {
  return (
    <HeroFrame
      variant={variant}
      label="A TypeScript client node with an arrow into the allwright core, and two arrows out of the core: one into a real browser window labelled page.goto and page.click, and one into an Android phone labelled androidApp.click and androidApp.fill, showing one test driving both surfaces"
    >
      <defs>
        <ArrowMarker id="hero-android-arrow" />
      </defs>

      <text x={380} y={196} textAnchor="middle" fontSize="14" fontFamily="var(--font-mono)" fill="var(--muted)">
        @allwright.dev/vitest
      </text>

      <line x1={230} y1={230} x2={494} y2={230} stroke="currentColor" strokeWidth="1.4" markerEnd="url(#hero-android-arrow)" />

      {/* TypeScript node */}
      <rect x={90} y={170} width={140} height={120} rx={24} fill="#3178C6" />
      <text x={160} y={246} textAnchor="middle" fontSize="52" fontWeight="700" fill="#ffffff" fontFamily="var(--font-mono)">
        TS
      </text>
      <text x={160} y={140} textAnchor="middle" fontSize="15" fontWeight="600" fill="var(--ink)">
        TypeScript client
      </text>

      {/* allwright core */}
      <circle cx={560} cy={230} r={66} fill="var(--accent-soft)" stroke="var(--accent-2)" strokeWidth="1.6" />
      <text x={560} y={224} textAnchor="middle" fontSize="16" fontWeight="600" fill="var(--accent-2)">
        allwright
      </text>
      <text x={560} y={246} textAnchor="middle" fontSize="13" fill="var(--accent-2)">
        core
      </text>

      {/* Fan-out to browser (top) and Android device (bottom) */}
      <line x1={624} y1={205} x2={860} y2={140} stroke="currentColor" strokeWidth="1.4" markerEnd="url(#hero-android-arrow)" />
      <line x1={624} y1={255} x2={860} y2={335} stroke="currentColor" strokeWidth="1.4" markerEnd="url(#hero-android-arrow)" />

      <text x={745} y={122} textAnchor="middle" fontSize="12.5" fontFamily="var(--font-mono)" fill="var(--muted)">
        page.goto · page.click
      </text>
      <text x={745} y={358} textAnchor="middle" fontSize="12.5" fontFamily="var(--font-mono)" fill="var(--muted)">
        androidApp.click · fill
      </text>

      {/* Browser node */}
      <rect x={860} y={85} width={172} height={110} rx={16} fill="var(--background)" stroke="currentColor" strokeWidth="1" />
      <rect x={860} y={85} width={172} height={28} rx={16} fill="var(--card)" stroke="currentColor" strokeWidth="1" />
      <circle cx={876} cy={99} r={4} fill="var(--muted)" opacity="0.5" />
      <circle cx={890} cy={99} r={4} fill="var(--muted)" opacity="0.5" />
      <circle cx={904} cy={99} r={4} fill="var(--muted)" opacity="0.5" />
      <path
        d="M930 150c0-14.4 11.6-26 26-26s26 11.6 26 26-11.6 26-26 26-26-11.6-26-26Z"
        fill="none"
        stroke="var(--accent)"
        strokeWidth="1.6"
      />
      <path d="M930 150h52M956 124v52" stroke="var(--accent)" strokeWidth="1.2" opacity="0.6" />
      <text x={946} y={68} textAnchor="middle" fontSize="14" fontWeight="600" fill="var(--ink)">
        Real browser
      </text>

      {/* Android device node: phone outline with a tap-ripple glyph */}
      <rect x={900} y={270} width={92} height={150} rx={18} fill="var(--background)" stroke="currentColor" strokeWidth="1" />
      <rect x={936} y={282} width={20} height={4} rx={2} fill="var(--muted)" opacity="0.6" />
      <rect x={928} y={392} width={36} height={4} rx={2} fill="var(--muted)" opacity="0.6" />
      <circle cx={946} cy={345} r={20} fill="none" stroke="var(--accent)" strokeWidth="1.4" opacity="0.55" />
      <circle cx={946} cy={345} r={10} fill="none" stroke="var(--accent)" strokeWidth="1.4" />
      <circle cx={946} cy={345} r={3} fill="var(--accent)" />
      <text x={946} y={253} textAnchor="middle" fontSize="14" fontWeight="600" fill="var(--ink)">
        Android device
      </text>
      <text x={946} y={438} textAnchor="middle" fontSize="11.5" fill="var(--muted)">
        experimental · via adb
      </text>
    </HeroFrame>
  );
}

// A release-timeline hero instead of the usual client/core/surface diagram:
// the traveled road from day one to today is a solid line, today's "you are
// here" milestone is the filled node, and the road ahead to v0.1.0 is
// dashed — still coming, not yet arrived. The two chips hanging off today's
// node are the small, real, tangible feature set the post is actually
// about, not a promise about what v0.1.0 itself will contain.
function RoadmapHero({ variant }: { variant?: HeroVariant }) {
  return (
    <HeroFrame
      variant={variant}
      label="A release timeline: v0.0.7 at the start, a solid line traveled to today's v0.0.57 milestone marked 'you are here', then a dashed line ahead to v0.1.0 marked 'coming soon'. Two small chips labelled Web and Android hang off today's milestone, showing the small set of tangible features already real today."
    >
      <defs>
        <ArrowMarker id="hero-roadmap-arrow" />
      </defs>

      <line x1={130} y1={210} x2={600} y2={210} stroke="currentColor" strokeWidth="2" />
      <line
        x1={600}
        y1={210}
        x2={1050}
        y2={210}
        stroke="currentColor"
        strokeWidth="2"
        strokeDasharray="6 6"
        markerEnd="url(#hero-roadmap-arrow)"
      />

      {/* Day one */}
      <circle cx={130} cy={210} r={10} fill="var(--card)" stroke="currentColor" strokeWidth="1.6" />
      <text x={130} y={172} textAnchor="middle" fontSize="14" fontWeight="600" fill="var(--ink)">
        v0.0.7
      </text>
      <text x={130} y={252} textAnchor="middle" fontSize="12" fill="var(--muted)">
        Day one
      </text>

      {/* Today */}
      <circle cx={600} cy={210} r={30} fill="var(--accent-soft)" stroke="var(--accent-2)" strokeWidth="2" />
      <text x={600} y={204} textAnchor="middle" fontSize="16" fontWeight="600" fill="var(--accent-2)">
        v0.0.57
      </text>
      <text x={600} y={224} textAnchor="middle" fontSize="11.5" fill="var(--accent-2)">
        you are here
      </text>

      {/* Two tangible-today chips hanging off "today" */}
      <line x1={560} y1={236} x2={470} y2={310} stroke="currentColor" strokeWidth="1.2" opacity="0.6" />
      <line x1={640} y1={236} x2={730} y2={310} stroke="currentColor" strokeWidth="1.2" opacity="0.6" />
      <rect x={390} y={310} width={160} height={56} rx={14} fill="var(--card)" stroke="var(--accent)" strokeWidth="1.4" />
      <text x={470} y={334} textAnchor="middle" fontSize="14" fontWeight="600" fill="var(--ink)">
        Web
      </text>
      <text x={470} y={352} textAnchor="middle" fontSize="10.5" fill="var(--accent)">
        ● real today
      </text>
      <rect x={650} y={310} width={160} height={56} rx={14} fill="var(--card)" stroke="var(--accent)" strokeWidth="1.4" />
      <text x={730} y={334} textAnchor="middle" fontSize="14" fontWeight="600" fill="var(--ink)">
        Android
      </text>
      <text x={730} y={352} textAnchor="middle" fontSize="10.5" fill="var(--accent)">
        ● real today
      </text>

      {/* Coming soon */}
      <circle cx={1050} cy={210} r={22} fill="var(--card)" stroke="currentColor" strokeWidth="1.6" strokeDasharray="4 3" />
      <text x={1050} y={160} textAnchor="middle" fontSize="16" fontWeight="600" fill="var(--ink)">
        v0.1.0
      </text>
      <text x={1050} y={180} textAnchor="middle" fontSize="12" fill="var(--muted)">
        coming soon
      </text>
    </HeroFrame>
  );
}

function DefaultHero({ variant }: { variant?: HeroVariant }) {
  return (
    <HeroFrame variant={variant} label="The allwright logo mark on a gradient card">
      <defs>
        <radialGradient id="hero-default-glow" cx="50%" cy="50%" r="60%">
          <stop offset="0%" stopColor="var(--accent-soft)" />
          <stop offset="100%" stopColor="transparent" />
        </radialGradient>
      </defs>
      <rect x="0" y="0" width="1200" height="460" fill="url(#hero-default-glow)" />
      <circle cx={600} cy={230} r={64} fill="var(--accent-soft)" stroke="var(--accent-2)" strokeWidth="1.6" />
      <text x={600} y={224} textAnchor="middle" fontSize="16" fontWeight="600" fill="var(--accent-2)">
        allwright
      </text>
      <text x={600} y={246} textAnchor="middle" fontSize="13" fill="var(--accent-2)">
        blog
      </text>
    </HeroFrame>
  );
}

const heroRegistry: Record<string, (props: { variant?: HeroVariant }) => React.ReactElement> = {
  "get-started-with-typescript": TypeScriptHero,
  "why-allwright-if-playwright-exists": EngineHero,
  "android-testing-playwright-style": AndroidHero,
  "road-to-v0-1-0": RoadmapHero,
};

export function HeroImage({ slug, variant }: { slug: string; variant?: HeroVariant }) {
  const Hero = heroRegistry[slug] ?? DefaultHero;
  return <Hero variant={variant} />;
}
