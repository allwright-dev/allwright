// Shared remark/rehype pipeline for rendering a post's MDX body. Kept in one
// place so the slug page's compileMDX() call and any future consumer (an RSS
// feed, a preview endpoint) stay in sync.

import type { Element } from "hast";
import remarkGfm from "remark-gfm";
import type { Options as PrettyCodeOptions } from "rehype-pretty-code";
import rehypePrettyCode from "rehype-pretty-code";
import type { PluggableList } from "unified";

const prettyCodeOptions: PrettyCodeOptions = {
  theme: "github-dark-default",
  keepBackground: true,
  // A plain string here applies to inline `code` too, which would send every
  // single-backtick span through Shiki and paint it with the block theme's
  // dark background. Scope the fallback language to fenced blocks only, so
  // inline code stays plain markup for mdx-components.tsx to style.
  defaultLang: { block: "text" },
  // Shiki collapses empty lines to zero height; give them a single space so
  // blank lines inside a snippet still take up a full line.
  onVisitLine(element: Element) {
    if (element.children.length === 0) {
      element.children = [{ type: "text", value: " " }];
    }
  },
};

export const blogMdxOptions: { remarkPlugins: PluggableList; rehypePlugins: PluggableList } = {
  remarkPlugins: [remarkGfm],
  rehypePlugins: [[rehypePrettyCode, prettyCodeOptions]],
};
