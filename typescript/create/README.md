# create-allwright

Create a new allwright test project with:

```bash
npm init allwright@latest
```

Or:

```bash
npm create allwright@latest
```

The initializer prompts for:

- `TypeScript` or `JavaScript`
- one or more surfaces such as `Web` and `Mobile Android`
- an optional target directory

It scaffolds a Node project with `package.json`, Vitest config, shared allwright config, starter tests, and a short README for the generated app.

Dependencies are installed automatically with the package manager that ran the initializer. Existing lockfiles take precedence; use `--package-manager npm|yarn|pnpm|bun` to override detection, or `--no-install` to scaffold without installing.
