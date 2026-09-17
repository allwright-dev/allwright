import { CodeBlock } from "./code-block";
import type { ApiKind, ApiMember } from "./types";

const KIND_LABEL: Record<ApiKind, string> = {
  function: "Function",
  method: "Method",
  type: "Type",
  class: "Class",
  property: "Property",
};

export async function MemberCard({
  member,
  moduleSlug,
  codeLang,
}: {
  member: ApiMember;
  moduleSlug: string;
  codeLang: string;
}) {
  // Prefixed with the module slug: member names repeat across modules
  // within the same language (e.g. "click" exists conceptually in both
  // Page and Mobile), and anchors must be unique on the page.
  const anchorId = `${moduleSlug}-${member.name}`.toLowerCase().replace(/[^a-z0-9]+/g, "-");

  return (
    <div
      id={anchorId}
      className="scroll-mt-24 rounded-2xl border border-[var(--line)] bg-[var(--card)] p-5 backdrop-blur-xl sm:p-6"
    >
      <div className="flex flex-wrap items-center justify-between gap-2">
        <h4 className="font-mono text-base font-semibold text-[var(--ink)]">{member.name}</h4>
        <div className="flex flex-wrap items-center gap-2">
          <span className="rounded-full border border-[var(--line)] px-2.5 py-0.5 font-mono text-[0.62rem] uppercase tracking-[0.06em] text-[var(--muted)]">
            {KIND_LABEL[member.kind]}
          </span>
          <span className="rounded-full bg-[var(--accent-soft)] px-2.5 py-0.5 font-mono text-[0.62rem] uppercase tracking-[0.06em] text-[var(--accent)]">
            Since {member.since}
          </span>
        </div>
      </div>
      <p className="mt-3 text-sm leading-6 text-[var(--muted)]">{member.description}</p>
      <div className="mt-4">
        <CodeBlock code={member.signature} lang={codeLang} />
      </div>
      {member.example ? (
        <div className="mt-3">
          <p className="mb-2 text-xs font-semibold uppercase tracking-[0.08em] text-[var(--muted)]">Example</p>
          <CodeBlock code={member.example} lang={codeLang} />
        </div>
      ) : null}
    </div>
  );
}
