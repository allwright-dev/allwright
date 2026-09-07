import { ImageResponse } from "next/og";

import { SocialCard } from "../brand";

const description =
  "npm init allwright@latest scaffolds a working TypeScript or JavaScript project — config, a starter test, dependencies installed — in one command.";

export const size = { width: 1200, height: 630 };
export const contentType = "image/png";
export const alt = description;

export default function TwitterImage() {
  return new ImageResponse(
    (
      <SocialCard
        eyebrow="Quickstart"
        title="One command to a passing test."
        description={description}
        pills={["Web", "Mobile Android", "TypeScript", "JavaScript"]}
      />
    ),
    { ...size }
  );
}
