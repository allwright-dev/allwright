import { ImageResponse } from "next/og";

import { SocialCard } from "../../brand";
import { getLanguageIds, getReference } from "../data";
import type { LanguageId } from "../types";

export const size = { width: 1200, height: 630 };
export const contentType = "image/png";
export const alt = "allwright API reference social preview card";

export function generateStaticParams() {
  return getLanguageIds().map((language) => ({ language }));
}

export default async function TwitterImage({ params }: { params: Promise<{ language: string }> }) {
  const { language } = await params;
  const reference = getReference(language as LanguageId);
  const label = reference?.label ?? "API";
  const description = reference
    ? `Every type and method ${reference.packageName} ships today, with a version badge on every entry.`
    : "Every type and method allwright ships today, per client language.";

  return new ImageResponse(
    (
      <SocialCard
        eyebrow="API Reference"
        title={`${label} API reference.`}
        description={description}
        pills={reference?.modules.slice(0, 4).map((module) => module.title) ?? []}
      />
    ),
    { ...size }
  );
}
