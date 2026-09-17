import Link from "next/link";

import { references } from "./data";
import type { LanguageId } from "./types";

// Route-based, not client state — switching language is a real navigation
// to /reference/<id>, same pattern as the site-wide nav pill in
// app/site-header.tsx, so each language keeps its own shareable URL.
export function LanguageTabs({ active }: { active: LanguageId }) {
  return (
    <nav
      aria-label="Language"
      className="flex flex-wrap items-center justify-center gap-1 rounded-full border border-[var(--line)] bg-[var(--card)] p-1 backdrop-blur-xl"
    >
      {references.map((reference) => {
        const isActive = reference.id === active;
        return (
          <Link
            key={reference.id}
            href={`/reference/${reference.id}`}
            aria-current={isActive ? "page" : undefined}
            className={`rounded-full px-4 py-1.5 text-sm font-medium transition ${
              isActive
                ? "bg-[var(--accent-soft)] text-[var(--accent-2)]"
                : "text-[var(--muted)] hover:text-[var(--ink)]"
            }`}
          >
            {reference.label}
          </Link>
        );
      })}
    </nav>
  );
}
