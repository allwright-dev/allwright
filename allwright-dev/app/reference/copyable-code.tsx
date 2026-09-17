"use client";

import { useState } from "react";

// Shiki renders server-side to a raw HTML string (see code-block.tsx); this
// just wraps that string with the same copy-to-clipboard affordance the blog
// post code blocks use (app/blog/pre.tsx), reading the raw source back from
// props instead of the DOM since there's no single <pre> ref to read here.
export function CopyableCode({ html, code }: { html: string; code: string }) {
  const [copied, setCopied] = useState(false);

  async function handleCopy() {
    try {
      await navigator.clipboard.writeText(code);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1600);
    } catch {
      // Clipboard access can be blocked (insecure context, permissions);
      // there's nothing useful to do but leave the button as-is.
    }
  }

  return (
    <div className="group/code relative">
      <div
        className="overflow-x-auto rounded-xl border border-[var(--line)] [&>pre]:overflow-x-auto [&>pre]:p-4 [&>pre]:text-[0.8rem] [&>pre]:leading-6 sm:[&>pre]:text-[0.85rem]"
        dangerouslySetInnerHTML={{ __html: html }}
      />
      <button
        type="button"
        onClick={handleCopy}
        className="absolute right-3 top-3 rounded-full border border-white/15 bg-black/40 px-2.5 py-1 font-mono text-[0.65rem] uppercase tracking-[0.06em] text-white/70 opacity-0 backdrop-blur transition group-hover/code:opacity-100 hover:text-white focus-visible:opacity-100"
      >
        {copied ? "Copied" : "Copy"}
      </button>
    </div>
  );
}
