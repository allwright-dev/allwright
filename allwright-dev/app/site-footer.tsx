import { GITHUB_URL } from "./brand";

export function SiteFooter() {
  return (
    <footer className="relative mx-auto mt-14 flex w-full max-w-6xl flex-col items-center gap-3 px-4 pb-10 text-center sm:mt-16 sm:flex-row sm:justify-between sm:px-6 sm:text-left">
      <p className="text-sm text-[var(--muted)]">
        allwright — one engine, all right.
      </p>
      <a
        href={GITHUB_URL}
        target="_blank"
        rel="noreferrer"
        className="text-sm font-medium text-[var(--accent-2)] underline-offset-4 hover:underline"
      >
        Watch our progress on GitHub
      </a>
    </footer>
  );
}
