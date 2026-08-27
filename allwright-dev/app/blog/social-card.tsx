// Per-post variant of app/brand.tsx's SocialCard: same palette and mark, but
// titled with the post instead of the site tagline, so link unfurls for
// individual posts don't all look identical.

import { BRAND_FROM, BRAND_TO, LogoMark } from "../brand";

export function PostSocialCard({ title, date }: { title: string; date: string }) {
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
      <div style={{ display: "flex", alignItems: "center", gap: 20 }}>
        <LogoMark size={56} />
        <span
          style={{
            fontSize: 30,
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
            fontSize: 20,
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
          marginTop: 52,
          fontSize: 58,
          fontWeight: 600,
          lineHeight: 1.12,
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
          marginTop: 32,
          fontSize: 26,
          color: "#9db4b0",
        }}
      >
        {date}
      </div>
    </div>
  );
}
