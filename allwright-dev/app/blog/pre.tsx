"use client";

import { useRef, useState } from "react";

// rehype-pretty-code renders a fenced code block as
// <figure><pre data-language=".." style="...">...</pre></figure>. This
// swaps in for that <pre>, adding a copy-to-clipboard button without having
// to know the raw source string ahead of time — it just reads back whatever
// text ended up in the DOM.
export function Pre({ children, className, ...props }: React.ComponentPropsWithoutRef<"pre">) {
  const preRef = useRef<HTMLPreElement>(null);
  const [copied, setCopied] = useState(false);

  async function handleCopy() {
    const text = preRef.current?.textContent ?? "";
    try {
      await navigator.clipboard.writeText(text);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1600);
    } catch {
      // Clipboard access can be blocked (insecure context, permissions);
      // there's nothing useful to do but leave the button as-is.
    }
  }

  return (
    <div className="group/code relative">
      <pre
        ref={preRef}
        {...props}
        className={`overflow-x-auto py-4 text-[0.82rem] leading-6 sm:py-5 sm:text-[0.85rem] ${className ?? ""}`}
      >
        {children}
      </pre>
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
