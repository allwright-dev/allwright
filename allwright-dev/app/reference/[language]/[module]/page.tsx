import type { Metadata } from "next";
import Link from "next/link";
import { notFound } from "next/navigation";

import { SITE_NAME } from "../../../brand";
import { getLanguageIds, getReference } from "../../data";
import { MemberCard } from "../../member-card";
import type { ApiModule, LanguageId } from "../../types";

function findModule(modules: ApiModule[], slug: string) {
  return modules.find((module) => module.slug === slug);
}

export function generateStaticParams() {
  return getLanguageIds().flatMap((language) => {
    const reference = getReference(language);
    return (reference?.modules ?? []).map((module) => ({ language, module: module.slug }));
  });
}

export async function generateMetadata({
  params,
}: {
  params: Promise<{ language: string; module: string }>;
}): Promise<Metadata> {
  const { language, module: moduleSlug } = await params;
  const reference = getReference(language as LanguageId);
  const module = reference && findModule(reference.modules, moduleSlug);
  if (!reference || !module) return {};

  const title = `${module.title} — ${reference.label} API reference`;
  const description = `${module.description} ${module.members.length} documented ${
    module.members.length === 1 ? "member" : "members"
  } in ${reference.packageName}.`;

  return {
    title,
    description,
    alternates: { canonical: `/reference/${reference.id}/${module.slug}` },
    openGraph: {
      type: "website",
      url: `/reference/${reference.id}/${module.slug}`,
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

export default async function ReferenceModulePage({
  params,
}: {
  params: Promise<{ language: string; module: string }>;
}) {
  const { language, module: moduleSlug } = await params;
  const reference = getReference(language as LanguageId);
  const module = reference && findModule(reference.modules, moduleSlug);
  if (!reference || !module) notFound();

  const index = reference.modules.findIndex((m) => m.slug === module.slug);
  const previous = reference.modules[index - 1];
  const next = reference.modules[index + 1];

  return (
    <div>
      <p className="flex flex-wrap items-center gap-1.5 text-xs font-medium text-[var(--muted)]">
        <Link href="/reference" className="hover:text-[var(--ink)]">
          API Reference
        </Link>
        <span aria-hidden="true">/</span>
        <Link href={`/reference/${reference.id}`} className="hover:text-[var(--ink)]">
          {reference.label}
        </Link>
        <span aria-hidden="true">/</span>
        <span className="text-[var(--ink)]">{module.title}</span>
      </p>

      <h1 className="mt-3 text-2xl font-semibold tracking-[-0.01em] text-[var(--ink)] sm:text-3xl">
        {module.title}
      </h1>
      <p className="mt-3 max-w-[65ch] text-sm leading-6 text-[var(--muted)] sm:text-base">
        {module.description}
      </p>

      <div className="mt-8 grid gap-4">
        {module.members.map((member) => (
          <MemberCard key={member.name} member={member} moduleSlug={module.slug} codeLang={reference.codeLang} />
        ))}
      </div>

      {(previous || next) && (
        <div className="mt-10 flex flex-wrap items-stretch justify-between gap-3 border-t border-[var(--line)] pt-6">
          {previous ? (
            <Link
              href={`/reference/${reference.id}/${previous.slug}`}
              className="flex flex-col rounded-xl border border-[var(--line)] bg-[var(--card)] px-4 py-2.5 text-left transition hover:border-[var(--accent-2)]"
            >
              <span className="text-xs text-[var(--muted)]">← Previous</span>
              <span className="text-sm font-medium text-[var(--ink)]">{previous.title}</span>
            </Link>
          ) : (
            <span />
          )}
          {next ? (
            <Link
              href={`/reference/${reference.id}/${next.slug}`}
              className="flex flex-col rounded-xl border border-[var(--line)] bg-[var(--card)] px-4 py-2.5 text-right transition hover:border-[var(--accent-2)]"
            >
              <span className="text-xs text-[var(--muted)]">Next →</span>
              <span className="text-sm font-medium text-[var(--ink)]">{next.title}</span>
            </Link>
          ) : (
            <span />
          )}
        </div>
      )}
    </div>
  );
}
