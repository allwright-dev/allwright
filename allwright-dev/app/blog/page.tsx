import type { Metadata } from "next";
import Link from "next/link";

import { SITE_NAME, SITE_URL } from "../brand";
import { formatPostDate, getAllPosts } from "./blog-data";
import { HeroImage } from "./hero-image";

const description =
  "The allwright blog: guides, release notes, and engineering deep-dives on test automation for web, mobile, desktop, and API, across every client language allwright ships.";

export const metadata: Metadata = {
  title: "Blog",
  description,
  keywords: [
    "allwright blog",
    "test automation blog",
    "web automation tutorials",
    "end-to-end testing guides",
    "QA engineering",
    "TypeScript testing",
    "browser automation",
  ],
  alternates: { canonical: "/blog" },
  openGraph: {
    type: "website",
    url: "/blog",
    siteName: SITE_NAME,
    locale: "en_US",
    title: "The allwright blog",
    description,
  },
  twitter: {
    card: "summary_large_image",
    title: "The allwright blog",
    description,
  },
};

export default function BlogIndex() {
  const posts = getAllPosts();

  const jsonLd = {
    "@context": "https://schema.org",
    "@type": "Blog",
    name: "allwright blog",
    url: `${SITE_URL}/blog`,
    blogPost: posts.map((post) => ({
      "@type": "BlogPosting",
      headline: post.frontmatter.title,
      description: post.frontmatter.description,
      datePublished: post.frontmatter.date,
      url: `${SITE_URL}/blog/${post.slug}`,
    })),
  };

  return (
    <div className="relative mx-auto w-full max-w-5xl pb-6">
      <script
        type="application/ld+json"
        // eslint-disable-next-line react/no-danger
        dangerouslySetInnerHTML={{ __html: JSON.stringify(jsonLd) }}
      />
      <section className="mx-auto mt-10 max-w-3xl text-center sm:mt-14">
        <p className="mb-5 inline-flex items-center gap-2 rounded-full border border-[var(--line)] bg-[var(--card)] px-4 py-1.5 font-mono text-[0.78rem] uppercase tracking-[0.14em] text-[var(--accent-2)]">
          Blog
        </p>
        <h1 className="text-[clamp(2.2rem,5vw,3.4rem)] leading-[1.05] font-semibold tracking-[-0.03em] text-[var(--ink)]">
          The allwright blog.
        </h1>
        <p className="mt-5 text-[clamp(1rem,1.6vw,1.15rem)] leading-8 text-[var(--muted)]">
          Guides, release notes, and engineering deep-dives on test
          automation — getting started in your language, what shipped and
          why, and how the core and its plugins are actually built.
        </p>
      </section>

      {posts.length === 0 ? (
        <section className="mx-auto mt-14 max-w-lg rounded-[2rem] border border-dashed border-[var(--line)] bg-[var(--card)] p-8 text-center sm:mt-16">
          <p className="text-sm text-[var(--muted)]">
            Nothing published yet — check back soon.
          </p>
        </section>
      ) : (
        <section aria-label="posts" className="mx-auto mt-14 grid w-full gap-6 sm:mt-16 sm:grid-cols-2">
          {posts.map((post) => (
            <Link
              key={post.slug}
              href={`/blog/${post.slug}`}
              className="group flex h-full flex-col overflow-hidden rounded-[1.5rem] border border-[var(--line)] bg-[var(--card)] backdrop-blur-xl transition hover:-translate-y-1 hover:border-[var(--accent-2)]"
            >
              <HeroImage slug={post.slug} variant="card" />
              <div className="flex flex-1 flex-col p-6">
                <div className="flex flex-wrap items-center gap-2 font-mono text-[0.7rem] uppercase tracking-[0.08em] text-[var(--muted)]">
                  <time dateTime={post.frontmatter.date}>{formatPostDate(post.frontmatter.date)}</time>
                  <span aria-hidden="true">·</span>
                  <span>{post.readingMinutes} min read</span>
                </div>
                <h2 className="mt-3 text-lg font-semibold text-[var(--ink)] sm:text-xl">
                  {post.frontmatter.title}
                </h2>
                <p className="mt-2 text-sm leading-6 text-[var(--muted)]">
                  {post.frontmatter.description}
                </p>
                {post.frontmatter.tags.length > 0 && (
                  <div className="mt-4 flex flex-wrap gap-2">
                    {post.frontmatter.tags.map((tag) => (
                      <span
                        key={tag}
                        className="rounded-full bg-[var(--accent-soft)] px-2.5 py-1 font-mono text-[0.65rem] text-[var(--accent-2)]"
                      >
                        {tag}
                      </span>
                    ))}
                  </div>
                )}
                <span className="mt-auto inline-flex items-center gap-1 pt-4 text-xs font-medium text-[var(--accent-2)]">
                  Read the guide →
                </span>
              </div>
            </Link>
          ))}
        </section>
      )}
    </div>
  );
}
