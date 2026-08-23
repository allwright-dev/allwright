import { ImageResponse } from "next/og";

import { SITE_DESCRIPTION, SocialCard } from "./brand";

export const size = { width: 1200, height: 630 };
export const contentType = "image/png";
export const alt = SITE_DESCRIPTION;

export default function OpengraphImage() {
  return new ImageResponse(<SocialCard />, { ...size });
}
