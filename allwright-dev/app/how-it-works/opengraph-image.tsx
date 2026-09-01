import { ImageResponse } from "next/og";

import { SocialCard } from "../brand";

const description =
  "A small core engine plus an installable plugin per surface: web and mobile today, desktop and API to come.";

export const size = { width: 1200, height: 630 };
export const contentType = "image/png";
export const alt = description;

export default function OpengraphImage() {
  return new ImageResponse(
    <SocialCard eyebrow="How it works" title="À la carte, not a buffet." description={description} />,
    { ...size }
  );
}
