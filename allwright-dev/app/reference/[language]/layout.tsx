import Link from "next/link";
import { notFound } from "next/navigation";

import { getLanguageIds, getReference } from "../data";
import { LanguageTabs } from "../language-tabs";
import { ReferenceSidebar } from "../reference-sidebar";
import type { LanguageId } from "../types";

export function generateStaticParams() {
  return getLanguageIds().map((language) => ({ language }));
}

// Shared chrome for every page under /reference/[language]/* — the language
// switcher and the module sidebar stay mounted across module-to-module
// navigation instead of re-rendering per page, the same "index, then drill
// down without losing your place" shape a Docusaurus sidebar gives you.
export default async function ReferenceLanguageLayout({
  children,
  params,
}: {
  children: React.ReactNode;
  params: Promise<{ language: string }>;
}) {
  const { language } = await params;
  const reference = getReference(language as LanguageId);
  if (!reference) notFound();

  return (
    <div className="relative mx-auto w-full max-w-6xl pb-6">
      <div className="mx-auto mt-10 flex flex-wrap items-center justify-between gap-4 sm:mt-14">
        <Link
          href="/reference"
          className="text-sm font-medium text-[var(--muted)] transition hover:text-[var(--ink)]"
        >
          ← All languages
        </Link>
        <LanguageTabs active={reference.id} />
      </div>

      <div className="mt-8 grid gap-6 sm:grid-cols-[220px_1fr] sm:gap-10">
        <aside className="sm:sticky sm:top-24 sm:h-fit">
          <ReferenceSidebar reference={reference} />
        </aside>
        <div className="min-w-0">{children}</div>
      </div>
    </div>
  );
}
