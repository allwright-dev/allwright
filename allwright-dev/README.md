# allwright.dev

Standalone Next.js site for the `allwright.dev` domain.

This app uses Bun as the primary local workflow, Next.js 16 as the current stable app framework line, and Tailwind CSS v4 through the official PostCSS integration.

## Local development

```bash
cd allwright.dev
bun install
bun run dev
```

Local development stays Bun-first, but Vercel deployments use the standard Node/npm build path via `vercel.json` for stability.

## Deploy

Deploy the `allwright.dev/` folder as a Next.js project on Vercel.

The initial site is a coming-soon landing page that can grow into the public marketing and documentation site for the project.

## Analytics

Google Analytics (GA4) is wired up in `app/layout.tsx` via `@next/third-parties/google`, gated by a single env var:

```bash
NEXT_PUBLIC_GA_ID=G-XXXXXXXXXX
```

Copy `.env.example` to `.env.local` and set it to your GA4 measurement ID to enable tracking; leave it unset to run with analytics off (the default locally). In Vercel, set `NEXT_PUBLIC_GA_ID` as a project environment variable — scope it to Production only if you don't want preview/staging deploys reporting into the same GA property.

## Workflow

Bun is the primary local development stack for this site.

```bash
bun run build
bun run start
```
