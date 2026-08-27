// Element overrides passed straight to compileMDX() for a post body. Hand-
// styled with the same CSS variables as the rest of the site (see
// app/globals.css) rather than a typography plugin, to match how every
// other page here is built.

import type { MDXComponents } from "mdx/types";

import { Pre } from "./pre";

export const blogMdxComponents: MDXComponents = {
  h2: ({ children, ...props }) => (
    <h2 {...props} className="mt-12 text-2xl font-semibold tracking-[-0.01em] text-[var(--ink)] first:mt-0 sm:text-[1.7rem]">
      {children}
    </h2>
  ),
  h3: ({ children, ...props }) => (
    <h3 {...props} className="mt-9 text-xl font-semibold text-[var(--ink)]">
      {children}
    </h3>
  ),
  h4: ({ children, ...props }) => (
    <h4 {...props} className="mt-7 text-base font-semibold text-[var(--ink)]">
      {children}
    </h4>
  ),
  p: ({ children, ...props }) => (
    <p {...props} className="mt-5 text-[0.98rem] leading-7 text-[var(--muted)] first:mt-0">
      {children}
    </p>
  ),
  a: ({ children, href = "", ...props }) => {
    const external = /^https?:\/\//.test(href);
    return (
      <a
        {...props}
        href={href}
        target={external ? "_blank" : undefined}
        rel={external ? "noreferrer" : undefined}
        className="font-medium text-[var(--accent-2)] underline decoration-[var(--line)] underline-offset-4 transition hover:decoration-[var(--accent-2)]"
      >
        {children}
      </a>
    );
  },
  ul: ({ children, ...props }) => (
    <ul {...props} className="mt-5 list-disc space-y-2 pl-6 text-[0.98rem] leading-7 text-[var(--muted)] marker:text-[var(--accent-2)]">
      {children}
    </ul>
  ),
  ol: ({ children, ...props }) => (
    <ol {...props} className="mt-5 list-decimal space-y-2 pl-6 text-[0.98rem] leading-7 text-[var(--muted)] marker:text-[var(--accent-2)] marker:font-medium">
      {children}
    </ol>
  ),
  li: ({ children, ...props }) => (
    <li {...props} className="pl-1">
      {children}
    </li>
  ),
  blockquote: ({ children, ...props }) => (
    <blockquote
      {...props}
      className="mt-6 rounded-r-xl border-l-2 border-[var(--accent-2)] bg-[var(--card)] py-3 pl-5 pr-4 text-[0.95rem] leading-7 text-[var(--muted)]"
    >
      {children}
    </blockquote>
  ),
  strong: ({ children, ...props }) => (
    <strong {...props} className="font-semibold text-[var(--ink)]">
      {children}
    </strong>
  ),
  hr: (props) => <hr {...props} className="my-10 border-[var(--line)]" />,
  table: ({ children, ...props }) => (
    <div className="mt-6 overflow-x-auto rounded-xl border border-[var(--line)]">
      <table {...props} className="w-full border-collapse text-sm">
        {children}
      </table>
    </div>
  ),
  thead: ({ children, ...props }) => (
    <thead {...props} className="bg-[var(--card)]">
      {children}
    </thead>
  ),
  th: ({ children, ...props }) => (
    <th {...props} className="border-b border-[var(--line)] px-4 py-2 text-left font-semibold text-[var(--ink)]">
      {children}
    </th>
  ),
  td: ({ children, ...props }) => (
    <td {...props} className="border-b border-[var(--line)] px-4 py-2 align-top text-[var(--muted)]">
      {children}
    </td>
  ),
  code: ({ children, className, ...props }) => {
    // Fenced code blocks are already handled by rehype-pretty-code and
    // carry a data-language attribute; leave those untouched and only style
    // genuine inline `code` spans.
    if ("data-language" in props) {
      return (
        <code className={className} {...props}>
          {children}
        </code>
      );
    }
    return (
      <code className="rounded-md border border-[var(--line)] bg-[var(--card)] px-1.5 py-0.5 font-mono text-[0.85em] text-[var(--ink)]">
        {children}
      </code>
    );
  },
  pre: Pre,
};
