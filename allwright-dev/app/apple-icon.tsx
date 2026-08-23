import { ImageResponse } from "next/og";

import { LogoMark } from "./brand";

export const size = { width: 180, height: 180 };
export const contentType = "image/png";

export default function AppleIcon() {
  return new ImageResponse(
    (
      <div
        style={{
          width: "100%",
          height: "100%",
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          background: "#eef8f4",
        }}
      >
        <LogoMark size={148} />
      </div>
    ),
    { ...size },
  );
}
