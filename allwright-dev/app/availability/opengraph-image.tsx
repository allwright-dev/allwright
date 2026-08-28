import { ImageResponse } from "next/og";

import { SocialCard } from "../brand";

export const size = { width: 1200, height: 630 };
export const contentType = "image/png";

export default function OpengraphImage() {
  return new ImageResponse(
    (
      <SocialCard
        eyebrow="Availability"
        title="What's real today, what isn't yet."
        description="The honest, current picture of allwright: what ships, what's still a reserved plugin slot, and which client languages are published."
        pills={["Rust", "Go", "Java", "Python", "TypeScript"]}
      />
    ),
    { ...size }
  );
}
