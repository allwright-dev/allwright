// Blog posts are sourced from MDX files in content/blog, one file per post,
// with metadata stored as YAML frontmatter inside each file. This module is
// the only place that touches the filesystem — pages just call these
// functions and stay plain Server Components.

import fs from "node:fs";
import path from "node:path";

import matter from "gray-matter";
import readingTime from "reading-time";

const BLOG_DIR = path.join(process.cwd(), "content", "blog");

export type BlogFrontmatter = {
  title: string;
  description: string;
  /** ISO date, e.g. "2026-08-28". */
  date: string;
  author: string;
  authorRole?: string;
  tags: string[];
  /** Extra search-intent phrases for <meta name="keywords">; falls back to tags. */
  keywords?: string[];
  /**
   * Defaults to true when omitted, so existing posts don't need to opt in.
   * Set to `false` in frontmatter to keep a post out of the index, its own
   * route, the sitemap, and generateStaticParams — a work-in-progress post
   * stays a plain file in content/blog until it's flipped back on.
   */
  published?: boolean;
};

export type BlogPost = {
  slug: string;
  frontmatter: BlogFrontmatter;
  /** Raw MDX body, frontmatter block already stripped. */
  content: string;
  readingMinutes: number;
};

const REQUIRED_STRING_FIELDS = ["title", "description", "date", "author"] as const;

function toFrontmatter(data: Record<string, unknown>, slug: string): BlogFrontmatter {
  for (const field of REQUIRED_STRING_FIELDS) {
    if (typeof data[field] !== "string" || data[field] === "") {
      throw new Error(`Blog post "${slug}.mdx" is missing required frontmatter field "${field}"`);
    }
  }

  return {
    title: data.title as string,
    description: data.description as string,
    date: data.date as string,
    author: data.author as string,
    authorRole: typeof data.authorRole === "string" ? data.authorRole : undefined,
    tags: Array.isArray(data.tags) ? data.tags.map(String) : [],
    keywords: Array.isArray(data.keywords) ? data.keywords.map(String) : undefined,
    // Anything other than an explicit `false` counts as published.
    published: data.published !== false,
  };
}

function readPost(slug: string): BlogPost {
  const filePath = path.join(BLOG_DIR, `${slug}.mdx`);
  const raw = fs.readFileSync(filePath, "utf8");
  const { data, content } = matter(raw);

  return {
    slug,
    frontmatter: toFrontmatter(data, slug),
    content,
    readingMinutes: Math.max(1, Math.round(readingTime(content).minutes)),
  };
}

export function getPostSlugs(): string[] {
  if (!fs.existsSync(BLOG_DIR)) return [];
  return fs
    .readdirSync(BLOG_DIR)
    .filter((file) => file.endsWith(".mdx"))
    .map((file) => file.replace(/\.mdx$/, ""));
}

export function getAllPosts({ includeUnpublished = false } = {}): BlogPost[] {
  return getPostSlugs()
    .map(readPost)
    .filter((post) => includeUnpublished || post.frontmatter.published)
    .sort((a, b) => (a.frontmatter.date < b.frontmatter.date ? 1 : -1));
}

export function getPostBySlug(slug: string): BlogPost | null {
  try {
    const post = readPost(slug);
    return post.frontmatter.published ? post : null;
  } catch {
    return null;
  }
}

export function formatPostDate(date: string): string {
  return new Date(`${date}T00:00:00Z`).toLocaleDateString("en-US", {
    year: "numeric",
    month: "long",
    day: "numeric",
    timeZone: "UTC",
  });
}
