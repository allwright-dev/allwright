"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";

import type { LanguageReference } from "./types";

// Persistent module tree for one language, the left rail of the
// index-then-drill-down layout (app/reference/[language]/layout.tsx) — stays
// mounted across module-to-module navigation instead of re-rendering per
// page, the same role Docusaurus's sidebar plays, just sized to our
// six-to-seven-item module list instead of a full page tree. Reads the
// active module from the URL (like site-header.tsx's nav) rather than a
// prop, since the layout that renders this doesn't itself receive the
// [module] segment.
export function ReferenceSidebar({ reference }: { reference: LanguageReference }) {
  const pathname = usePathname();

  return (
    <nav aria-label={`${reference.label} API modules`} className="flex gap-1 overflow-x-auto sm:flex-col sm:overflow-visible">
      {reference.modules.map((module) => {
        const active = pathname === `/reference/${reference.id}/${module.slug}`;
        return (
          <Link
            key={module.slug}
            href={`/reference/${reference.id}/${module.slug}`}
            aria-current={active ? "page" : undefined}
            className={`flex shrink-0 items-center justify-between gap-3 rounded-xl px-3 py-2 text-sm font-medium transition sm:shrink ${
              active
                ? "bg-[var(--accent-soft)] text-[var(--accent-2)]"
                : "text-[var(--muted)] hover:bg-[var(--card)] hover:text-[var(--ink)]"
            }`}
          >
            <span>{module.title}</span>
            <span className="font-mono text-xs text-[var(--muted)]">{module.members.length}</span>
          </Link>
        );
      })}
    </nav>
  );
}
