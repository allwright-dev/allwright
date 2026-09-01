import { ImageResponse } from "next/og";

import { SocialCard } from "../brand";

const description =
  "Browser and Android automation, new client languages, and the plugin architecture underneath all of it.";

export const size = { width: 1200, height: 630 };
export const contentType = "image/png";
export const alt = description;

export default function TwitterImage() {
  return new ImageResponse(
    <SocialCard eyebrow="Changelog" title="What shipped, release by release." description={description} />,
    { ...size }
  );
}
