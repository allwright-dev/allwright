import type { MetadataRoute } from "next";

import { SITE_DESCRIPTION, SITE_NAME } from "./brand";

export default function manifest(): MetadataRoute.Manifest {
  return {
    name: "Allwright — one automation engine for everything you test",
    short_name: SITE_NAME,
    description: SITE_DESCRIPTION,
    start_url: "/",
    display: "standalone",
    background_color: "#eef8f4",
    theme_color: "#0e9f8e",
    icons: [
      { src: "/icon", sizes: "32x32", type: "image/png" },
      { src: "/apple-icon", sizes: "180x180", type: "image/png" },
    ],
  };
}
