@AGENTS.md

# Blog conventions (hard rule)

- Every new blog post MUST get a custom hero image — never leave it on the
  generic/default hero. Add a slug-keyed entry to `heroRegistry` in
  `app/blog/hero-image.tsx` (the in-page hero) and a matching entry to
  `ogHeroRegistry` in `app/blog/og-hero.tsx` (the social-preview card
  diagram), both illustrating what that specific post is actually about.
- Every new blog post MUST be optimized for link previews before it's
  considered done: a short, specific `title`, a single tight-sentence
  `description` (it becomes the OG/Twitter card copy), and a real check that
  `/blog/<slug>/opengraph-image` and `/blog/<slug>/twitter-image` render
  correctly — not just that the post page itself builds.
