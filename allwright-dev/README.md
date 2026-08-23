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

## Workflow

Bun is the primary local development stack for this site.

```bash
bun run build
bun run start
```
