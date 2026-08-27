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
};

export function HeroImage({ slug, variant }: { slug: string; variant?: HeroVariant }) {
  const Hero = heroRegistry[slug] ?? DefaultHero;
  return <Hero variant={variant} />;
}
