import type { Metadata } from "next";
import Link from "next/link";
import { notFound } from "next/navigation";

import { SITE_NAME } from "../../brand";
import { CodeBlock } from "../code-block";
import { getLanguageIds, getReference } from "../data";
import type { LanguageId } from "../types";

export function generateStaticParams() {
  return getLanguageIds().map((language) => ({ language }));
}

export async function generateMetadata({
  params,
}: {
  params: Promise<{ language: string }>;
}): Promise<Metadata> {
  const { language } = await params;
  const reference = getReference(language as LanguageId);
  if (!reference) return {};

  const description = `Every ${reference.label} type and method allwright ships today — signatures, descriptions, and the release each one shipped in, from ${reference.packageName}.`;
  const title = `${reference.label} API reference`;

  return {
    title,
    description,
    alternates: { canonical: `/reference/${reference.id}` },
    openGraph: {
      type: "website",
      url: `/reference/${reference.id}`,
      siteName: SITE_NAME,
      locale: "en_US",
      title,
      description,
    },
    twitter: {
      card: "summary_large_image",
      title,
      description,
    },
  };
}

// The category-index page a Docusaurus sidebar item points at before you
// pick a specific doc: this language's install step, then one card per
// module, each linking into its own /reference/[language]/[module] page.
export default async function ReferenceLanguagePage({
  params,
}: {
  params: Promise<{ language: string }>;
}) {
  const { language } = await params;
  const reference = getReference(language as LanguageId);
  if (!reference) notFound();

  return (
    <div>
      <h1 className="text-2xl font-semibold tracking-[-0.01em] text-[var(--ink)] sm:text-3xl">
        {reference.label} API reference
      </h1>
      <p className="mt-3 max-w-[65ch] text-sm leading-6 text-[var(--muted)] sm:text-base">
        Every type and method{" "}
        <code className="rounded-md border border-[var(--line)] bg-[var(--card)] px-1.5 py-0.5 font-mono text-[0.85em] text-[var(--ink)]">
          {reference.packageName}
        </code>{" "}
        ships today, with a <strong className="font-semibold text-[var(--ink)]">Since</strong> badge on
        every entry matching the{" "}
        <Link href="/changelog" className="font-medium text-[var(--accent-2)] hover:underline">
          changelog
        </Link>{" "}
        release it first shipped in.
      </p>

      <div className="mt-6 max-w-xl">
        <CodeBlock code={reference.installCommand} lang="bash" />
      </div>

      <div className="mt-10 grid gap-4 sm:grid-cols-2">
        {reference.modules.map((module) => (
          <Link
            key={module.slug}
            href={`/reference/${reference.id}/${module.slug}`}
            className="flex h-full flex-col rounded-2xl border border-[var(--line)] bg-[var(--card)] p-5 backdrop-blur-xl transition hover:-translate-y-1 hover:border-[var(--accent-2)]"
          >
            <div className="flex items-center justify-between gap-2">
              <h2 className="text-base font-semibold text-[var(--ink)]">{module.title}</h2>
              <span className="rounded-full border border-[var(--line)] px-2 py-0.5 font-mono text-[0.65rem] text-[var(--muted)]">
                {module.members.length}
              </span>
            </div>
            <p className="mt-2 text-sm leading-6 text-[var(--muted)]">{module.description}</p>
            <span className="mt-auto inline-flex items-center gap-1 pt-4 text-xs font-medium text-[var(--accent-2)]">
              View {module.title} →
            </span>
          </Link>
        ))}
      </div>
    </div>
  );
}
