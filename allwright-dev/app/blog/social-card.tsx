// Per-post variant of app/brand.tsx's SocialCard: same palette and mark, but
// titled with the post instead of the site tagline, so link unfurls for
// individual posts don't all look identical. When the post has a matching
// entry in og-hero.tsx, its diagram renders alongside the title so the
// preview actually reflects what the post is about, not just its headline;
// posts without one fall back to a text-only layout.

import { BRAND_FROM, BRAND_TO, LogoMark } from "../brand";

export function PostSocialCard({
  title,
  date,
  diagram,
}: {
  title: string;
  date: string;
  diagram?: React.ReactElement | null;
}) {
  return (
    <div
      style={{
        width: "100%",
        height: "100%",
        display: "flex",
        alignItems: "center",
        padding: "72px 88px",
        background: "linear-gradient(135deg, #06171a 0%, #0a2a2c 45%, #0f2f3f 100%)",
        fontFamily: "sans-serif",
      }}
    >
      <div style={{ display: "flex", flexDirection: "column", flex: "1 1 auto", ...(diagram ? { maxWidth: 650 } : {}) }}>
        <div style={{ display: "flex", alignItems: "center", gap: 18 }}>
          <LogoMark size={52} />
          <span
            style={{
              fontSize: 28,
              fontWeight: 600,
              letterSpacing: "-0.02em",
              color: "#eaf7f3",
            }}
          >
            allwright
          </span>
          <span
            style={{
              display: "flex",
              padding: "6px 16px",
              borderRadius: 999,
              fontSize: 18,
              color: "#eaf7f3",
              background: `linear-gradient(120deg, ${BRAND_FROM}33, ${BRAND_TO}33)`,
              border: "1px solid rgba(234,247,243,0.18)",
            }}
          >
            Blog
          </span>
        </div>

        <div
          style={{
            display: "flex",
            marginTop: 44,
            fontSize: 52,
            fontWeight: 600,
            lineHeight: 1.14,
            letterSpacing: "-0.03em",
            color: "#f4fbf9",
          }}
        >
          {title}
        </div>

        <div style={{ display: "flex", marginTop: 28, fontSize: 24, color: "#9db4b0" }}>{date}</div>
      </div>

      {diagram ? <div style={{ display: "flex", flex: "0 0 auto", marginLeft: 40 }}>{diagram}</div> : null}
    </div>
  );
}
