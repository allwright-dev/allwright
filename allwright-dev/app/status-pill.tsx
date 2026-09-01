// Shared status vocabulary so "ready to use" vs "not yet" always reads the
// same way, whether it's a surface plugin's install status or a client
// library's publish status.

const READY_STATUSES = new Set(["Available now", "Android available", "Published"]);

export function StatusPill({ status }: { status: string }) {
  const ready = READY_STATUSES.has(status);

  return (
    <span
      className={`shrink-0 whitespace-nowrap rounded-full px-2.5 py-1 font-mono text-[0.62rem] uppercase tracking-[0.06em] ${
        ready
          ? "bg-[var(--accent-soft)] text-[var(--accent)]"
          : "border border-dashed border-[var(--line)] text-[var(--muted)]"
      }`}
    >
      {status}
    </span>
  );
}
