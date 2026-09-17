import { ImageResponse } from "next/og";

import { SocialCard } from "../brand";

const description =
  "Every type and method allwright ships today, one page per client language, with a version badge on every entry.";

export const size = { width: 1200, height: 630 };
export const contentType = "image/png";
export const alt = description;

export default function OpengraphImage() {
  return new ImageResponse(
    (
      <SocialCard
        eyebrow="API Reference"
        title="One page per client, every method."
        description={description}
        pills={["TypeScript", "Python", "Java", "Rust", "Go"]}
      />
    ),
    { ...size }
  );
}
