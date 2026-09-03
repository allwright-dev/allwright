// Satori-safe (next/og) versions of the per-post hero diagrams in
// hero-image.tsx, for embedding in generated opengraph-image/twitter-image
// PNGs. Satori can't resolve the site's CSS custom properties or
// `currentColor`, and it doesn't support `<text>` inside `<svg>` at all, so
// every diagram here draws its shapes as plain SVG with literal hex colors
// and layers every label as a normal, absolutely-positioned HTML <div> on
// top instead. Keep this in sync with hero-image.tsx's registry when a new
// post gets a custom hero: same slug, same shape, two renderers.

import { BRAND_FROM, BRAND_TO } from "../brand";

const INK = "#eaf7f3";
const INK_SOFT = "#f4fbf9";
const MUTED = "#9db4b0";
const LINE = "rgba(234, 247, 243, 0.28)";

const DIAGRAM_SIZE = { width: 420, height: 260 };

function TypeScriptOgDiagram() {
  const { width, height } = DIAGRAM_SIZE;
  const midY = 132;

  return (
    <div style={{ position: "relative", width, height, display: "flex" }}>
      <svg width={width} height={height} viewBox={`0 0 ${width} ${height}`} style={{ position: "absolute", left: 0, top: 0 }}>
        <rect x={0} y={midY - 40} width={84} height={80} rx={16} fill="#3178C6" />
        <line x1={90} y1={midY} x2={154} y2={midY} stroke={MUTED} strokeWidth={2} />
        <circle cx={210} cy={midY} r={50} fill="rgba(14,159,142,0.16)" stroke={BRAND_TO} strokeWidth={2} />
        <line x1={266} y1={midY} x2={330} y2={midY} stroke={MUTED} strokeWidth={2} />
        <rect x={336} y={midY - 40} width={84} height={80} rx={14} fill="none" stroke={LINE} strokeWidth={2} />
      </svg>

      <div style={{ position: "absolute", left: 42, top: midY - 22, transform: "translate(-50%,-50%)", display: "flex", fontSize: 24, fontWeight: 700, color: "#ffffff" }}>
        TS
      </div>
      <div style={{ position: "absolute", left: 210, top: midY - 20, transform: "translateX(-50%)", display: "flex", flexDirection: "column", alignItems: "center" }}>
        <div style={{ display: "flex", fontSize: 15, fontWeight: 700, color: INK_SOFT }}>allwright</div>
        <div style={{ display: "flex", fontSize: 12, color: MUTED, marginTop: 2 }}>core</div>
      </div>
      <div style={{ position: "absolute", left: 378, top: midY - 22, transform: "translate(-50%,-50%)", display: "flex", fontSize: 13, fontWeight: 600, color: INK }}>
        Browser
      </div>
    </div>
  );
}

// Same fragmentation-vs-one-core story as hero-image.tsx's EngineHero,
// compressed to a single fan-out instead of two side-by-side panels so it
// still reads at social-preview thumbnail size.
function EngineOgDiagram() {
  const { width, height } = DIAGRAM_SIZE;
  const rowYs = [10, 78, 146, 214];
  const rowHeight = 46;
  const boxX = 240;
  const boxWidth = 160;
  const surfaces = [
    { label: "Web", ready: true },
    { label: "Mobile", ready: false },
    { label: "Desktop", ready: false },
    { label: "API", ready: false },
  ];
  const core = { cx: 70, cy: 138, r: 46 };

  return (
    <div style={{ position: "relative", width, height, display: "flex" }}>
      <svg width={width} height={height} viewBox={`0 0 ${width} ${height}`} style={{ position: "absolute", left: 0, top: 0 }}>
        <circle cx={core.cx} cy={core.cy} r={core.r} fill="rgba(14,159,142,0.16)" stroke={BRAND_TO} strokeWidth={2} />
        {surfaces.map((surface, i) => (
          <line
            key={`line-${surface.label}`}
            x1={core.cx + core.r}
            y1={core.cy}
            x2={boxX}
            y2={rowYs[i] + rowHeight / 2}
            stroke={surface.ready ? BRAND_TO : LINE}
            strokeWidth={2}
            strokeDasharray={surface.ready ? undefined : "5 5"}
          />
        ))}
        {surfaces.map((surface, i) => (
          <rect
            key={`box-${surface.label}`}
            x={boxX}
            y={rowYs[i]}
            width={boxWidth}
            height={rowHeight}
            rx={12}
            fill="rgba(16,41,45,0.55)"
            stroke={surface.ready ? BRAND_TO : LINE}
            strokeWidth={surface.ready ? 2 : 1.5}
            strokeDasharray={surface.ready ? undefined : "5 5"}
          />
        ))}
      </svg>

      <div style={{ position: "absolute", left: core.cx, top: core.cy - 20, transform: "translateX(-50%)", display: "flex", flexDirection: "column", alignItems: "center" }}>
        <div style={{ display: "flex", fontSize: 15, fontWeight: 700, color: INK_SOFT }}>allwright</div>
        <div style={{ display: "flex", fontSize: 12, color: MUTED, marginTop: 2 }}>core</div>
      </div>
      {surfaces.map((surface, i) => (
        <div
          key={`label-${surface.label}`}
          style={{
            position: "absolute",
            left: boxX + boxWidth / 2,
            top: rowYs[i] + rowHeight / 2 - 11,
            transform: "translateX(-50%)",
            display: "flex",
            fontSize: 16,
            fontWeight: 600,
            color: INK,
          }}
        >
          {surface.label}
        </div>
      ))}
    </div>
  );
}

// Same TS-client-into-core shape as TypeScriptOgDiagram, but fanned out to
// two right-hand targets instead of one: the browser plus a phone standing
// in for the androidApp fixture, so the thumbnail reads as "one client, two
// surfaces" at a glance.
function AndroidOgDiagram() {
  const { width, height } = DIAGRAM_SIZE;
  const midY = 132;
  const coreX = 172;

  return (
    <div style={{ position: "relative", width, height, display: "flex" }}>
      <svg width={width} height={height} viewBox={`0 0 ${width} ${height}`} style={{ position: "absolute", left: 0, top: 0 }}>
        <rect x={0} y={midY - 40} width={84} height={80} rx={16} fill="#3178C6" />
        <line x1={90} y1={midY} x2={coreX - 44} y2={midY} stroke={MUTED} strokeWidth={2} />

        <circle cx={coreX} cy={midY} r={44} fill="rgba(14,159,142,0.16)" stroke={BRAND_TO} strokeWidth={2} />

        <line x1={coreX + 44} y1={midY - 14} x2={306} y2={midY - 44} stroke={BRAND_TO} strokeWidth={2} />
        <line x1={coreX + 44} y1={midY + 14} x2={306} y2={midY + 62} stroke={BRAND_TO} strokeWidth={2} />

        {/* Browser node */}
        <rect x={312} y={midY - 74} width={100} height={64} rx={12} fill="none" stroke={LINE} strokeWidth={2} />
        {/* Phone node */}
        <rect x={330} y={midY + 26} width={64} height={86} rx={14} fill="none" stroke={LINE} strokeWidth={2} />
        <rect x={352} y={midY + 96} width={20} height={3} rx={1.5} fill={MUTED} />
      </svg>

      <div style={{ position: "absolute", left: 42, top: midY - 22, transform: "translate(-50%,-50%)", display: "flex", fontSize: 24, fontWeight: 700, color: "#ffffff" }}>
        TS
      </div>
      <div style={{ position: "absolute", left: coreX, top: midY - 20, transform: "translateX(-50%)", display: "flex", flexDirection: "column", alignItems: "center" }}>
        <div style={{ display: "flex", fontSize: 14, fontWeight: 700, color: INK_SOFT }}>allwright</div>
        <div style={{ display: "flex", fontSize: 11, color: MUTED, marginTop: 2 }}>core</div>
      </div>
      <div style={{ position: "absolute", left: 362, top: midY - 42, transform: "translateX(-50%)", display: "flex", fontSize: 13, fontWeight: 600, color: INK }}>
        Browser
      </div>
      <div style={{ position: "absolute", left: 362, top: midY + 69, transform: "translateX(-50%)", display: "flex", fontSize: 13, fontWeight: 600, color: INK }}>
        Android
      </div>
    </div>
  );
}

// Same "today, then dashed road ahead" story as RoadmapHero in
// hero-image.tsx, compressed to fit the thumbnail: a solid line from day
// one to today's filled milestone, a dashed line on to v0.1.0.
function RoadmapOgDiagram() {
  const { width, height } = DIAGRAM_SIZE;
  const midY = 132;

  return (
    <div style={{ position: "relative", width, height, display: "flex" }}>
      <svg width={width} height={height} viewBox={`0 0 ${width} ${height}`} style={{ position: "absolute", left: 0, top: 0 }}>
        <line x1={30} y1={midY} x2={210} y2={midY} stroke={MUTED} strokeWidth={2} />
        <line x1={210} y1={midY} x2={390} y2={midY} stroke={LINE} strokeWidth={2} strokeDasharray="6 6" />

        <circle cx={30} cy={midY} r={8} fill="rgba(16,41,45,0.55)" stroke={MUTED} strokeWidth={1.6} />
        <circle cx={210} cy={midY} r={30} fill="rgba(14,159,142,0.16)" stroke={BRAND_TO} strokeWidth={2} />
        <circle cx={390} cy={midY} r={20} fill="none" stroke={LINE} strokeWidth={1.6} strokeDasharray="4 3" />
      </svg>

      <div style={{ position: "absolute", left: 30, top: midY - 34, transform: "translateX(-50%)", display: "flex", fontSize: 13, fontWeight: 600, color: INK }}>
        v0.0.7
      </div>
      <div style={{ position: "absolute", left: 210, top: midY - 6, transform: "translateX(-50%)", display: "flex", flexDirection: "column", alignItems: "center" }}>
        <div style={{ display: "flex", fontSize: 16, fontWeight: 700, color: INK_SOFT }}>v0.0.60</div>
        <div style={{ display: "flex", fontSize: 10.5, color: MUTED, marginTop: 2 }}>you are here</div>
      </div>
      <div style={{ position: "absolute", left: 390, top: midY - 52, transform: "translateX(-50%)", display: "flex", flexDirection: "column", alignItems: "center" }}>
        <div style={{ display: "flex", fontSize: 15, fontWeight: 700, color: INK_SOFT }}>v0.1.0</div>
        <div style={{ display: "flex", fontSize: 10.5, color: MUTED, marginTop: 2 }}>coming soon</div>
      </div>
    </div>
  );
}

// Same terminal-into-scaffolded-project shape as InitHero in hero-image.tsx,
// compressed to fit the thumbnail: a terminal card on the left, an arrow,
// then a short file list standing in for the full project tree.
function InitOgDiagram() {
  const { width, height } = DIAGRAM_SIZE;
  const midY = 130;
  const files = ["package.json", "vitest.config.ts", "tests/web.spec.ts"];

  return (
    <div style={{ position: "relative", width, height, display: "flex" }}>
      <svg width={width} height={height} viewBox={`0 0 ${width} ${height}`} style={{ position: "absolute", left: 0, top: 0 }}>
        <rect x={0} y={midY - 54} width={196} height={108} rx={14} fill="rgba(16,41,45,0.55)" stroke={LINE} strokeWidth={2} />
        <line x1={202} y1={midY} x2={244} y2={midY} stroke={MUTED} strokeWidth={2} />
        <rect x={250} y={midY - 60} width={170} height={120} rx={14} fill="none" stroke={BRAND_TO} strokeWidth={2} />
      </svg>

      <div style={{ position: "absolute", left: 16, top: midY - 40, display: "flex", fontSize: 13, fontFamily: "monospace", color: BRAND_TO }}>
        $ npm init allwright
      </div>
      <div style={{ position: "absolute", left: 16, top: midY - 12, display: "flex", fontSize: 12, fontFamily: "monospace", color: MUTED }}>
        ✔ TypeScript ✔ Web
      </div>
      <div style={{ position: "absolute", left: 16, top: midY + 12, display: "flex", fontSize: 12, fontFamily: "monospace", color: MUTED }}>
        ✔ installed
      </div>

      {files.map((file, i) => (
        <div
          key={file}
          style={{
            position: "absolute",
            left: 266,
            top: midY - 40 + i * 34,
            display: "flex",
            fontSize: 12.5,
            fontFamily: "monospace",
            color: INK,
          }}
        >
          {file}
        </div>
      ))}
    </div>
  );
}

const ogHeroRegistry: Record<string, () => React.ReactElement> = {
  "get-started-with-typescript": TypeScriptOgDiagram,
  "why-allwright-if-playwright-exists": EngineOgDiagram,
  "android-testing-playwright-style": AndroidOgDiagram,
  "road-to-v0-1-0": RoadmapOgDiagram,
  "npm-init-allwright": InitOgDiagram,
};

/** Returns the post's diagram element for its social-preview card, or null for posts without one (their card falls back to a text-only layout). */
export function getOgHeroDiagram(slug: string): React.ReactElement | null {
  const Diagram = ogHeroRegistry[slug];
  return Diagram ? <Diagram /> : null;
}
