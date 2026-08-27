import type { Metadata } from "next";
import Link from "next/link";
import { notFound } from "next/navigation";
import { compileMDX } from "next-mdx-remote/rsc";

import { GITHUB_URL, SITE_URL } from "../../brand";
import { formatPostDate, getAllPosts, getPostBySlug } from "../blog-data";
import { HeroImage } from "../hero-image";
import { blogMdxComponents } from "../mdx-components";
import { blogMdxOptions } from "../mdx-options";

export function generateStaticParams() {
  return getAllPosts().map((post) => ({ slug: post.slug }));
}

export async function generateMetadata({
  params,
}: {
  params: Promise<{ slug: string }>;
}): Promise<Metadata> {
  const { slug } = await params;
  const post = getPostBySlug(slug);
  if (!post) return {};

  const { title, description, date, author, tags, keywords } = post.frontmatter;
  const url = `/blog/${slug}`;

  return {
    title,
    description,
    keywords: keywords ?? tags,
    alternates: { canonical: url },
    openGraph: {
      type: "article",
      url,
      title,
      description,
      publishedTime: date,
      authors: [author],
      tags,
    },
    twitter: {
      card: "summary_large_image",
      title,
      description,
    },
  };
}

export default async function BlogPostPage({
  params,
}: {
  params: Promise<{ slug: string }>;
}) {
  const { slug } = await params;
  const post = getPostBySlug(slug);
  if (!post) notFound();

  const { content } = await compileMDX({
    source: post.content,
    components: blogMdxComponents,
    options: { mdxOptions: blogMdxOptions },
  });

  const jsonLd = {
    "@context": "https://schema.org",
    "@type": "BlogPosting",
    headline: post.frontmatter.title,
    description: post.frontmatter.description,
    datePublished: post.frontmatter.date,
    dateModified: post.frontmatter.date,
    author: { "@type": "Organization", name: post.frontmatter.author },
    publisher: { "@type": "Organization", name: "allwright", url: SITE_URL },
    keywords: (post.frontmatter.keywords ?? post.frontmatter.tags).join(", "),
    mainEntityOfPage: `${SITE_URL}/blog/${slug}`,
    url: `${SITE_URL}/blog/${slug}`,
    image: `${SITE_URL}/blog/${slug}/opengraph-image`,
  };

  return (
    <article className="relative mx-auto w-full max-w-3xl pb-6">
      <script
        type="application/ld+json"
        // eslint-disable-next-line react/no-danger
        dangerouslySetInnerHTML={{ __html: JSON.stringify(jsonLd) }}
      />
      <div className="mx-auto mt-10 sm:mt-14">
        <Link
          href="/blog"
          className="inline-flex items-center gap-1.5 text-sm font-medium text-[var(--accent-2)] hover:underline"
        >
          ← All posts
        </Link>

        <div className="mt-6 flex flex-wrap items-center gap-2 font-mono text-[0.7rem] uppercase tracking-[0.08em] text-[var(--muted)]">
          <time dateTime={post.frontmatter.date}>{formatPostDate(post.frontmatter.date)}</time>
          <span aria-hidden="true">·</span>
          <span>{post.readingMinutes} min read</span>
          <span aria-hidden="true">·</span>
          <span>{post.frontmatter.author}</span>
        </div>

        <h1 className="mt-4 text-[clamp(1.9rem,4.2vw,2.9rem)] leading-[1.08] font-semibold tracking-[-0.02em] text-[var(--ink)]">
          {post.frontmatter.title}
        </h1>
        <p className="mt-4 text-[1.05rem] leading-8 text-[var(--muted)]">
          {post.frontmatter.description}
        </p>

        {post.frontmatter.tags.length > 0 && (
          <div className="mt-5 flex flex-wrap gap-2">
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

        <div className="mt-8 sm:mt-10">
          <HeroImage slug={slug} />
        </div>
      </div>

      <div className="mt-10 border-t border-[var(--line)] pt-10 sm:mt-12">{content}</div>

      <section
        aria-label="get started"
        className="mx-auto mt-14 flex w-full flex-col items-center gap-4 rounded-[2rem] border border-[var(--line)] bg-[var(--card)] p-8 text-center backdrop-blur-xl sm:mt-16"
      >
        <h2 className="text-xl font-semibold text-[var(--ink)] sm:text-2xl">
          Try it in your own project
        </h2>
        <p className="max-w-[46ch] text-sm leading-6 text-[var(--muted)]">
          allwright is building in public. Star the repo to track progress,
          or keep reading the rest of the blog.
        </p>
        <div className="flex flex-wrap items-center justify-center gap-3">
          <a
            href={GITHUB_URL}
            target="_blank"
            rel="noreferrer"
            className="inline-flex items-center rounded-full bg-[linear-gradient(120deg,var(--accent),var(--accent-2))] px-6 py-3 text-sm font-semibold text-white shadow-[0_18px_40px_var(--accent-soft)] transition hover:-translate-y-0.5"
          >
            Star on GitHub
          </a>
          <Link
            href="/blog"
            className="inline-flex items-center rounded-full border border-[var(--line)] bg-[var(--card)] px-6 py-3 text-sm font-medium text-[var(--ink)] transition hover:-translate-y-0.5 hover:border-[var(--accent-2)]"
          >
            More posts
          </Link>
        </div>
      </section>
    </article>
  );
}
