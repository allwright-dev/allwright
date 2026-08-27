import { ImageResponse } from "next/og";

import { getAllPosts, formatPostDate, getPostBySlug } from "../blog-data";
import { PostSocialCard } from "../social-card";

export const size = { width: 1200, height: 630 };
export const contentType = "image/png";

export function generateStaticParams() {
  return getAllPosts().map((post) => ({ slug: post.slug }));
}

export default async function TwitterImage({ params }: { params: Promise<{ slug: string }> }) {
  const { slug } = await params;
  const post = getPostBySlug(slug);
  const title = post?.frontmatter.title ?? "allwright blog";
  const date = post ? formatPostDate(post.frontmatter.date) : "";

  return new ImageResponse(<PostSocialCard title={title} date={date} />, { ...size });
}
