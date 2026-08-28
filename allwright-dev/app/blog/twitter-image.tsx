import { ImageResponse } from "next/og";

import { SocialCard } from "../brand";

export const size = { width: 1200, height: 630 };
export const contentType = "image/png";

export default function TwitterImage() {
  return new ImageResponse(
    (
      <SocialCard
        eyebrow="Blog"
        title="The allwright blog."
        description="Guides, release notes, and engineering deep-dives on test automation, across every client language allwright ships."
        pills={[]}
      />
    ),
    { ...size }
  );
}
