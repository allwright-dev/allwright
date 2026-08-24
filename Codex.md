# Codex Handoff

This document is the handoff reference for AI-assisted work in this repository.

## Always Keep This Updated

Any AI agent working in this repo must update this file when architecture, ownership, workspace layout, runtime model, or important conventions change.

This instruction should be treated as ongoing project policy for all future AI conversations in this repository.

## Project Structure

- Top-level repo folders are now split by technology:
- `rust/`
  - `go/`
  - `java/`
  - `python/`
  - `allwright-dev/`
  - `typescript/`
  - `proto/`
- `rust/allwright`: `allwright-core`, the lightweight Rust engine crate containing the high-level Rust client API, the lightweight gRPC server fallback, and the shared plugin catalog
- `rust/allwright-cli`: installable `allwright` CLI package that depends on `allwright-core`, installs supported plugins, and delegates runtime startup to installed plugin binaries
- `.github/workflows/release-surface-plugins.yml`: tag-triggered GitHub Actions workflow that builds platform plugin archives and attaches them to GitHub Releases
- `scripts/install.sh`: Linux/macOS installer script for downloading the published `allwright` CLI release asset
- `scripts/install.ps1`: Windows PowerShell installer script for downloading the published `allwright` CLI release asset
- `rust/allwright-plugin-sdk`: shared plugin traits and surface metadata for publishable Rust surface crates
- `rust/allwright-surface-web`: publishable `web` surface crate that also ships the standalone `allwright-surface-web` runtime binary
- `rust/allwright-surface-mobile`: shared mobile surface abstractions consumed by `mobile-android` and `mobile-ios`
- `rust/allwright-surface-mobile-android`: publishable `mobile-android` surface crate
- `rust/allwright-surface-mobile-ios`: publishable `mobile-ios` surface crate
- `rust/allwright-surface-desktop`: shared desktop surface abstractions consumed by `desktop-mac`, `desktop-windows`, and `desktop-linux`
- `rust/allwright-surface-desktop-mac`: publishable `desktop-mac` surface crate
- `rust/allwright-surface-desktop-windows`: publishable `desktop-windows` surface crate
- `rust/allwright-surface-desktop-linux`: publishable `desktop-linux` surface crate
- `rust/allwright/examples/`: Rust example programs for exercising the lightweight engine core
- `go/`: Go module `allwright.dev` containing generated engine stubs, the public Go client package at the module root, and Go examples
- `java/`: Gradle-based Java client project that generates engine stubs from the shared proto and exposes a high-level browser/page API
- `python/`: Python client package that loads the shared proto dynamically at runtime and exposes a high-level browser/page API
- `allwright-dev/`: standalone Next.js site for the purchased `allwright.dev` domain, intended for Vercel deployment and public-facing marketing/docs entrypoints
- `typescript/`: TypeScript/JavaScript source folder containing the npm package source and examples
- `proto/`: shared protobuf and gRPC contract root for all stacks
- `proto/engine/v1/engine.proto`: umbrella engine service contract that imports the split proto ownership layers
- `proto/core/v1/`: core-owned shared engine/session/browser/tab message contracts
- `proto/surfaces/web/v1/`: web surface-owned command and event contracts
- `rust/allwright` now contains the lightweight Rust engine/client implementation and compiles against the shared top-level `proto/` contract during builds

## Runtime Model

- The entire project should remain Tokio async runtime driven.
- The CLI enters through `#[tokio::main]`.
- The Rust workspace publishable crates are currently aligned on version `0.0.7`.
- Installing the `allwright` package should provide the CLI plus the lightweight core together.
- `rust/allwright` now owns the lightweight Rust fallback gRPC server surface and public Rust client surface in one package.
- Engine behavior and engine-facing Rust contracts should stay inside `rust/allwright`.
- allwright should remain a single engine even as the product grows broader capabilities.
- Surface capabilities should be designed as separately installable plugins such as `web`, `mobile-android`, `mobile-ios`, `desktop-mac`, `desktop-windows`, and `desktop-linux`, all extending the core engine instead of creating separate engines or unrelated runtimes.
- The CLI currently installs the `web` plugin by downloading the matching archive from GitHub Releases into the local allwright plugin directory, records it in the local plugin manifest, and delegates `allwright serve` to the installed `allwright-surface-web` binary when present.
- The non-web surface crates should remain visible in the plugin catalog but should stay non-installable until they ship real runtime artifacts.
- GitHub tag pushes such as `v0.0.7` should trigger release automation that builds the current web plugin runtime archives for the supported OS matrix and uploads them to the matching GitHub Release.
- GitHub tag pushes such as `v0.0.7` should also build and upload the main `allwright` CLI archives so installer scripts can bootstrap core plus CLI without Cargo.
- The engine direction is driverless browser automation.
- Do not introduce ChromeDriver into the primary browser control path.
- The current gRPC layout lives in top-level `proto/`.
- The proto contract is now split by ownership: `engine/v1` is the umbrella service surface, `core/v1` owns shared engine/session messages, and `surfaces/web/v1` owns the current web-specific messages.
- The current proto package is `allwright.engine.v1`.
- The current starter service is `EngineService`.
- Generated gRPC client code is enabled and currently consumed by `rust/allwright` and its examples.
- Generated Go proto/gRPC code is checked in under `go/gen/allwright/engine/v1` and consumed by the Go module.
- The Java stack generates protobuf and gRPC stubs from `proto/engine/v1/engine.proto` during the Gradle build in `java/`.
- The Python stack currently loads `proto/engine/v1/engine.proto` dynamically at runtime through `grpcio-tools`.
- The TypeScript stack currently loads the shared top-level `proto/engine/v1/engine.proto` dynamically at runtime rather than checking in generated TS stubs.
- The public Rust client surface should stay high-level and should not expose raw gRPC connection setup.
- The public Go client surface should stay high-level and should not expose raw gRPC connection setup.
- The public Java client surface should stay high-level and should not expose raw gRPC connection setup.
- The public Python client surface should stay high-level and should not expose raw gRPC connection setup.
- The public TypeScript client surface should stay high-level and should not expose raw grpc-js client setup.
- Use Bun for local development workflows in the TypeScript stack, but keep the client surface generic for TypeScript/JavaScript consumers.
- Use Bun as the primary local development workflow for `allwright-dev/`.
- `allwright-dev/` should stay on the stable Next.js 16 release line and use Tailwind CSS v4 through the official PostCSS integration unless there is a deliberate migration decision.
- `allwright-dev/` is user-facing marketing/product surface, not developer-facing repo messaging.
- The core product message for `allwright-dev/` is that allwright is one automation engine for all automation needs.
- The GitHub repository was transferred; the canonical remote and all in-repo/site links now point at `https://github.com/allwright-dev/allwright` (previously `qalens/allwright`).
- All copy on `allwright-dev/` must read as user-facing product messaging (audience, benefits, surfaces covered); it must never describe internal implementation, architecture, or repo/dev workflow details.
- `allwright-dev/` uses `next-themes` (class strategy on `<html>`, `defaultTheme="system"`) for light/dark mode via `app/theme-provider.tsx` and exposes a toggle through `app/theme-toggle.tsx`.
- `allwright-dev/app/globals.css` defines the site palette as a green/blue (teal + blue accent) theme with light tokens on `:root` and dark tokens under `.dark`, matching the `next-themes` class strategy.
- `allwright-dev/app/brand.tsx` is the single source of truth for site/brand constants (`SITE_URL`, `SITE_NAME`, `SITE_TITLE`, `SITE_DESCRIPTION`, `GITHUB_URL`) and the shared logo/social-card JSX; update links (like the GitHub URL) there rather than inlining them per file.
- The allwright mark is a gradient (teal-to-blue) squircle with a white pointer/cursor glyph, defined once in `allwright-dev/app/brand.tsx` (`LogoMark`) for generated images and standalone as `allwright-dev/public/logo.svg` for static use.
- Favicon/app-icon/social-preview images are generated at build/request time via Next.js file conventions (`app/icon.tsx`, `app/apple-icon.tsx`, `app/opengraph-image.tsx`, `app/twitter-image.tsx`) using `next/og`'s `ImageResponse`, not static binary assets.
- `allwright-dev/` also ships `app/manifest.ts`, `app/robots.ts`, and `app/sitemap.ts` (Next.js metadata route conventions) plus full `openGraph`/`twitter`/`robots`/canonical metadata in `app/layout.tsx` and JSON-LD (`SoftwareApplication`) structured data in `app/page.tsx` for SEO and link-preview support.
- Vercel must use `allwright-dev/` as the project Root Directory for the site deployment.
- Local Bun development is preferred for `allwright-dev/`, but Vercel builds should use the standard Node/npm Next.js path for stability.
- The current Vercel workaround exists because Bun 1.3.14 has a known crash path with Next.js 16.3.0 builds on Linux, producing `SIGILL`/segfault failures during or after `next build`.
- The current RPCs are:
  - `Ping`
  - `BrowserSession` as a bidirectional stream
  - `TabSession` as a bidirectional stream
- The first browser command is `LaunchChromeCommand`.
- `LaunchChromeCommand` is only for opening a plain browser window.
- `LaunchChromeCommand` also returns `initial_tab_session_id` for the startup tab that Chrome already opened.
- Browser launch commands must not carry URL/navigation input.
- `OpenTabCommand` opens a new tab from `BrowserSession` and returns `tab_session_id`.
- `OpenTabCommand` is only valid after the parent browser session has launched.
- `TabSession` is the follow-up tab-scoped stream keyed by `browser_session_id` plus `tab_session_id`.
- `TabSession` must validate that the tab belongs to the specified browser session.
- The startup Chrome tab must be tracked like any later tab and accept the same tab-stream commands.
- Future tab launch commands must also not carry URL/navigation input.
- Navigation is a separate `NavigateTabCommand` on `TabSession`.
- Click is a separate `ClickElementCommand` on `TabSession`.
- Future tab creation and tab-level communication should be modeled as separate commands inside the browser session stream.
- Browser launch will likely need CDP enabled so the engine can attach to the browser session.
- The implemented web plugin runtime path enables CDP and surfaces browser-level connection metadata.
- The implemented web plugin runtime path tracks browser sessions with engine session ids and associated CDP session information.
- The implemented web plugin runtime path opens tabs through CDP `Target.createTarget`, not GUI scripting.
- The implemented web plugin runtime path discovers the initial launch-created tab through CDP `Target.getTargets`.
- The implemented web plugin runtime path runs tab navigation through CDP `Target.attachToTarget` plus `Page.navigate`.
- The implemented web plugin runtime path waits for `Page.loadEventFired`, then injects a pinned `chromium-bidi` mapper artifact into a hidden mapper target in the same browser session.
- The injected mapper is sourced from the published `chromium-bidi@17.0.2` package but is checked into `rust/allwright-surface-web/third_party/chromium-bidi/17.0.2/` so the publishable web surface crate does not depend on `npm`.
- The intended web plugin path persists a real `bidi_session_id` plus mapper target/session ids after mapper injection.
- The implemented web surface crate includes selector-based click through `script.evaluate` on the tab browsing context.
- Chrome launching logic currently lives in `allwright-surface-web`, uses the browser binary directly, and discovers the CDP WebSocket endpoint via `DevToolsActivePort`.
- Shared surface crates support plugin families, while leaf surface crates provide the publishable install targets.
- `rust/allwright` now hides the engine transport behind a lazy singleton connection and exposes `launch_chrome`, `ping`, `Browser`, `Tab`, and plugin catalog metadata from the lightweight core package.
- The Rust singleton client currently defaults to `http://127.0.0.1:50051`, also respects `ALLWRIGHT_SERVER_ADDR`, and can be redirected in-process with `allwright::set_server_addr(...)`.
- `rust/allwright/examples/playground.rs` now exercises the engine through the `allwright` crate, supports the launch-created initial tab plus additional browser-session tabs, and closes them through the high-level browser/tab API as a minimal end-to-end test flow.
- The Rust playground example waits for keyboard confirmation before closing the browser session so browser state can be observed manually.
- `go/` now uses module path `allwright.dev`, exposes package name `allwright` from the module root, and relies on `allwright.dev` vanity import metadata that points Go tooling at the GitHub repository subdirectory `go/`.
- The Go singleton client currently uses `ALLWRIGHT_SERVER_ADDR` or falls back to `127.0.0.1:50051`.
- `go/examples/playground` is the Go-side example and currently exercises a minimal browser-session test flow directly, without extra ping-style smoke commands.
- `java/src/main/java/dev/allwright/client/Allwright.java` now provides the Java client, hides the engine transport behind a lazy singleton connection, and exposes Playwright-style `chromium.launch(...)`, `Browser`, and `Page` methods instead of public gRPC setup.
- The Java singleton client currently uses `ALLWRIGHT_SERVER_ADDR` or falls back to `127.0.0.1:50051`, and supports in-process override with `Allwright.setServerAddr(...)`.
- `python/allwright/client.py` now provides the Python client, hides the engine transport behind a lazy singleton connection, and exposes Playwright-style `chromium.launch()`, `Browser`, and `Page` methods instead of public grpc setup.
- The Python singleton client currently uses `ALLWRIGHT_SERVER_ADDR` or falls back to `127.0.0.1:50051`, and supports in-process override with `set_server_addr(...)`.
- `allwright-dev/package.json` currently keeps Bun as the package manager declaration but uses standard `next dev`, `next build`, and `next start` scripts.
- `allwright-dev/vercel.json` currently pins Vercel site behavior to the Next.js framework with `npm install` and `npm run build` so deployments avoid the Bun build crash path.
- `typescript/src/index.ts` now provides the TypeScript/JavaScript client, hides the engine transport behind a lazy singleton connection, and exposes Playwright-style `chromium.launch(...)`, `Browser`, and `Page` methods instead of public grpc-js setup.
- New TypeScript work should prefer the `chromium` / `Browser` / `Page` surface; older compatibility helpers like `launchChrome`, `initialTab()`, `newTab()`, and `navigate()` should be treated as transitional.
- The TypeScript singleton client currently uses `ALLWRIGHT_SERVER_ADDR` or falls back to `127.0.0.1:50051`, and also supports in-process override with `setServerAddr(...)`.
- The repo root `package.json` now targets npm publication as `@allwright/core`, packages `typescript/dist/` plus the shared top-level `proto/`, and keeps the playground flow under `typescript/examples/playground.ts`.
- `typescript/examples/playground.ts` is the TypeScript-side example for a minimal browser-session test flow; use Bun for local development runs in this repo and keep it focused on real browser work rather than extra ping-style smoke commands.
- Regenerating Go stubs currently requires local installation of `protoc-gen-go@v1.36.10` and `protoc-gen-go-grpc@v1.5.1`, then running `protoc` against `proto/engine/v1/engine.proto` with output rooted at `go/`.

## Current Dependency Hierarchy

```text
proto/
└── engine/v1/engine.proto

rust/allwright-plugin-sdk
rust/allwright-surface-web
├── library surface helpers
└── standalone runtime binary
rust/allwright-surface-mobile
├── rust/allwright-surface-mobile-android
└── rust/allwright-surface-mobile-ios
rust/allwright-surface-desktop
├── rust/allwright-surface-desktop-mac
├── rust/allwright-surface-desktop-windows
└── rust/allwright-surface-desktop-linux

rust/allwright
├── high-level Rust client API
├── shared proto bindings
├── lightweight fallback server
└── plugin catalog metadata

rust/allwright-cli
├── installable `allwright` binary
├── plugin installation
└── runtime delegation

.github/workflows/release-surface-plugins.yml
└── build and upload release archives for installable plugin runtimes

scripts/install.sh
└── download and install the released `allwright` CLI on Linux/macOS

scripts/install.ps1
└── download and install the released `allwright` CLI on Windows
```

## Maintenance Rules

- Keep `README.md` and `Codex.md` aligned.
- Prefer documenting ownership changes here at the same time as code changes.
- Keep shared engine contracts and client-facing APIs in `rust/allwright`.
- Keep platform-specific runtime logic out of `rust/allwright`; it belongs in the surface crates.
- Prefer adding explicit extension points for surface plugins like `web`, `mobile-android`, `mobile-ios`, `desktop-mac`, `desktop-windows`, and `desktop-linux` over introducing parallel engine implementations.
- Keep async boundaries explicit and Tokio-native.
- Keep the gRPC contract minimal until requirements are explicitly defined.
- Preserve the driverless direction when evolving browser and tab session APIs.
