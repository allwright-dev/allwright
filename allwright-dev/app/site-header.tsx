"use client";

import Image from "next/image";
import Link from "next/link";
import { usePathname } from "next/navigation";

import { GITHUB_URL } from "./brand";
import { ThemeToggle } from "./theme-toggle";

const NAV_LINKS = [
  { href: "/", label: "Home" },
  { href: "/how-it-works", label: "How it works" },
  { href: "/availability", label: "Availability" },
  { href: "/changelog", label: "Changelog" },
  { href: "/blog", label: "Blog" },
];

export function SiteHeader() {
  const pathname = usePathname();

  return (
    <header className="relative mx-auto flex w-full max-w-6xl flex-col items-center gap-4 px-4 py-6 sm:grid sm:grid-cols-[1fr_auto_1fr] sm:items-center sm:px-6 sm:py-8">
      <Link href="/" className="flex items-center gap-2.5 sm:justify-self-start">
        <Image src="/logo.svg" alt="" width={28} height={28} priority className="rounded-[8px]" />
        <span className="font-mono text-[0.95rem] font-medium tracking-[-0.02em] text-[var(--ink)]">
          allwright
        </span>
      </Link>

      <nav
        aria-label="Primary"
        className="flex items-center gap-1 rounded-full border border-[var(--line)] bg-[var(--card)] p-1 backdrop-blur-xl sm:justify-self-center"
      >
        {NAV_LINKS.map((link) => {
          const active = pathname === link.href;
          return (
            <Link
              key={link.href}
              href={link.href}
              aria-current={active ? "page" : undefined}
              className={`rounded-full px-4 py-1.5 text-sm font-medium transition ${
                active
                  ? "bg-[var(--accent-soft)] text-[var(--accent-2)]"
                  : "text-[var(--muted)] hover:text-[var(--ink)]"
              }`}
            >
              {link.label}
            </Link>
          );
        })}
      </nav>

      <div className="flex items-center gap-3 sm:justify-self-end">
        <a
          href={GITHUB_URL}
          target="_blank"
          rel="noreferrer"
          className="hidden items-center rounded-full border border-[var(--line)] bg-[var(--card)] px-4 py-2 text-sm font-medium text-[var(--ink)] transition hover:-translate-y-0.5 hover:border-[var(--accent-2)] sm:inline-flex"
        >
          Star on GitHub
        </a>
        <ThemeToggle />
      </div>
    </header>
  );
}
