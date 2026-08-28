import { ImageResponse } from "next/og";

import { SocialCard } from "../brand";

export const size = { width: 1200, height: 630 };
export const contentType = "image/png";

export default function TwitterImage() {
  return new ImageResponse(
    (
      <SocialCard
        eyebrow="How it works"
        title="À la carte, not a buffet."
        description="A small core engine plus an installable plugin per surface: web today, mobile, desktop, and API to come."
      />
    ),
    { ...size }
  );
}
