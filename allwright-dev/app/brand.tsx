// Shared brand constants + the allwright mark, reused by the favicon/app-icon/
// social-preview image routes (app/icon.tsx, app/apple-icon.tsx,
// app/opengraph-image.tsx, app/twitter-image.tsx) so every generated image
// stays in sync with the site palette defined in app/globals.css.

export const SITE_URL = "https://allwright.dev";
export const SITE_NAME = "allwright";
export const SITE_TITLE = "Allwright — one automation engine for everything you test";
export const SITE_DESCRIPTION =
  "Allwright brings web, mobile, desktop, and API testing into a single automation engine, so teams stop stitching tools together and start shipping with confidence.";
export const GITHUB_URL = "https://github.com/allwright-dev/allwright";

export const BRAND_FROM = "#0e9f8e";
export const BRAND_TO = "#1f6fb2";

// A simple, high-contrast cursor/pointer glyph — automation acting on
// whatever surface it is pointed at. Kept as a single flat polygon so it
// stays legible down to a 16px favicon.
const POINTER_POINTS =
  "10,6 10,23 13.8,19.6 16.6,25.6 19.4,24.3 16.6,18.1 22,18.1";

export function LogoMark({ size = 32 }: { size?: number }) {
  return (
    <div
      style={{
        width: size,
        height: size,
        borderRadius: size * 0.28,
        background: `linear-gradient(135deg, ${BRAND_FROM}, ${BRAND_TO})`,
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
      }}
    >
      <svg
        width={size * 0.56}
        height={size * 0.56}
        viewBox="0 0 32 32"
      >
        <polygon points={POINTER_POINTS} fill="#ffffff" />
      </svg>
    </div>
  );
}

// Shared 1200x630 social-preview card, used by both app/opengraph-image.tsx
// and app/twitter-image.tsx so link unfurls on every platform match.
export function SocialCard() {
  return (
    <div
      style={{
        width: "100%",
        height: "100%",
        display: "flex",
        flexDirection: "column",
        justifyContent: "center",
        padding: "80px 96px",
        background: "linear-gradient(135deg, #06171a 0%, #0a2a2c 45%, #0f2f3f 100%)",
        fontFamily: "sans-serif",
      }}
    >
      <div style={{ display: "flex", alignItems: "center", gap: 24 }}>
        <LogoMark size={84} />
        <span
          style={{
            fontSize: 46,
            fontWeight: 600,
            letterSpacing: "-0.02em",
            color: "#eaf7f3",
          }}
        >
          allwright
        </span>
      </div>

      <div
        style={{
          display: "flex",
          marginTop: 56,
          fontSize: 66,
          fontWeight: 600,
          lineHeight: 1.08,
          letterSpacing: "-0.03em",
          color: "#f4fbf9",
          maxWidth: 980,
        }}
      >
        One automation engine for everything you test.
      </div>

      <div
        style={{
          display: "flex",
          marginTop: 34,
          fontSize: 30,
          color: "#9db4b0",
          maxWidth: 880,
        }}
      >
        Web, mobile, desktop, and API — unified in a single engine.
      </div>

      <div style={{ display: "flex", gap: 16, marginTop: 48 }}>
        {["Web", "Mobile", "Desktop", "API"].map((label) => (
          <div
            key={label}
            style={{
              display: "flex",
              padding: "10px 24px",
              borderRadius: 999,
              fontSize: 24,
              color: "#eaf7f3",
              background: `linear-gradient(120deg, ${BRAND_FROM}33, ${BRAND_TO}33)`,
              border: "1px solid rgba(234,247,243,0.18)",
            }}
          >
            {label}
          </div>
        ))}
      </div>
    </div>
  );
}
