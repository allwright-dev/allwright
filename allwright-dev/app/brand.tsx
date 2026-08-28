// Shared brand constants + the allwright mark, reused by the favicon/app-icon/
// social-preview image routes (app/icon.tsx, app/apple-icon.tsx,
// app/opengraph-image.tsx, app/twitter-image.tsx) so every generated image
// stays in sync with the site palette defined in app/globals.css.

export const SITE_URL = "https://allwright.dev";
export const SITE_NAME = "allwright";
export const SITE_TITLE = "Allwright — one automation engine for everything you test";
export const SITE_DESCRIPTION =
  "Allwright is a small core engine, built from the ground up, with an installable plugin for every surface you test — web, mobile, desktop, and API. À la carte, not a buffet.";
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

// Shared 1200x630 social-preview card. Defaults to the homepage pitch (used
// by app/opengraph-image.tsx and app/twitter-image.tsx as-is), but every
// other marketing page (how-it-works, availability, the blog index) passes
// its own eyebrow/title/description/pills so its link unfurl isn't just a
// copy of the homepage's.
export function SocialCard({
  eyebrow,
  title = "One automation engine for everything you test.",
  description = "Web, mobile, desktop, and API. One small core, one plugin per surface.",
  pills = ["Web", "Mobile", "Desktop", "API"],
}: {
  eyebrow?: string;
  title?: string;
  description?: string;
  pills?: string[];
}) {
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
        {eyebrow ? (
          <span
            style={{
              display: "flex",
              padding: "8px 22px",
              borderRadius: 999,
              fontSize: 26,
              color: "#eaf7f3",
              background: `linear-gradient(120deg, ${BRAND_FROM}33, ${BRAND_TO}33)`,
              border: "1px solid rgba(234,247,243,0.18)",
            }}
          >
            {eyebrow}
          </span>
        ) : null}
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
        {title}
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
        {description}
      </div>

      {pills.length > 0 && (
        <div style={{ display: "flex", gap: 16, marginTop: 48 }}>
          {pills.map((label) => (
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
      )}
    </div>
  );
}
