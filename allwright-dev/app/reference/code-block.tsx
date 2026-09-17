import { codeToHtml } from "shiki";

import { CopyableCode } from "./copyable-code";

// Server-rendered at build time (every reference page is static), so calling
// Shiki directly here costs nothing at request time — same theme as the blog
// post code blocks (app/blog/mdx-options.ts) for one consistent code look
// across the whole site.
export async function CodeBlock({ code, lang }: { code: string; lang: string }) {
  const html = await codeToHtml(code, { lang, theme: "github-dark-default" });
  return <CopyableCode html={html} code={code} />;
}
