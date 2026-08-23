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
- `rust/allwright`: CLI crate and top-level operator entrypoint
- `rust/allwright-client`: reusable Rust client crate with a higher-level browser/tab API
- `rust/playground`: end-to-end client crate for exercising the engine server
- `go/`: Go module containing generated engine stubs, a reusable Go client, and a Go playground
- `java/`: Gradle-based Java client project that generates engine stubs from the shared proto and exposes a high-level browser/page API
- `python/`: Python client package that loads the shared proto dynamically at runtime and exposes a high-level browser/page API
- `allwright-dev/`: standalone Next.js site for the purchased `allwright.dev` domain, intended for Vercel deployment and public-facing marketing/docs entrypoints
- `typescript/`: TypeScript/JavaScript stack folder containing the JS client package and playground
- `proto/`: shared protobuf and gRPC contract root for all stacks
- `rust/engine-lib`: owner of all engine code, including the gRPC server
- `rust/engine-lib` further uses `rust/mobile-lib`, `rust/desktop-lib`, and `rust/web-lib`
- `rust/mobile-lib`: mobile support library used by `rust/engine-lib`
- `rust/desktop-lib`: desktop support library used by `rust/engine-lib`
- `rust/web-lib`: web support library used by `rust/engine-lib`

Supporting libraries:

- `rust/mobile-lib` may depend on:
  - `rust/ios-lib`
  - `rust/android-lib`
- `rust/desktop-lib` may depend on:
  - `rust/windows-lib`
  - `rust/macos-lib`
  - `rust/linux-lib`

## Runtime Model

- The entire project should remain Tokio async runtime driven.
- The CLI enters through `#[tokio::main]`.
- `rust/engine-lib` owns the gRPC server surface.
- `rust/engine-lib` should remain the only owner of engine behavior and engine-facing contracts.
- The engine direction is driverless browser automation.
- Do not introduce ChromeDriver into the primary browser control path.
- The current gRPC layout lives in top-level `proto/`.
- The current proto package is `allwright.engine.v1`.
- The current starter service is `EngineService`.
- Generated gRPC client code is enabled and currently consumed by `rust/allwright-client` and `rust/playground`.
- Generated Go proto/gRPC code is checked in under `go/gen/allwright/engine/v1` and consumed by the Go module.
- The Java stack generates protobuf and gRPC stubs from `proto/engine/v1/engine.proto` during the Gradle build in `java/`.
- The Python stack currently loads `proto/engine/v1/engine.proto` dynamically at runtime through `grpcio-tools`.
- The TypeScript stack currently loads `proto/engine/v1/engine.proto` dynamically at runtime rather than checking in generated TS stubs.
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
- Browser launch now enables CDP and surfaces browser-level connection metadata.
- Browser sessions should be tracked with engine session ids and associated CDP session information.
- Tabs now open through CDP `Target.createTarget`, not GUI scripting.
- The initial launch-created tab is discovered through CDP `Target.getTargets`.
- Tab navigation now runs through CDP `Target.attachToTarget` plus `Page.navigate`.
- The current navigation path waits for `Page.loadEventFired`, then injects a pinned `chromium-bidi` mapper artifact into a hidden mapper target in the same browser session.
- The injected mapper is sourced from the published `chromium-bidi@17.0.2` package but is checked into `rust/web-lib/third_party/chromium-bidi/17.0.2/` so build/runtime do not depend on `npm`.
- Browser sessions now persist a real `bidi_session_id` plus mapper target/session ids after mapper injection.
- The current implemented BiDi action surface includes selector-based click through `script.evaluate` on the tab browsing context.
- Chrome launching currently routes through `web-lib`, uses the browser binary directly, and discovers the CDP WebSocket endpoint via `DevToolsActivePort`.
- Platform crates are support layers, not owners of engine logic.
- `rust/playground` is the place for local end-to-end testing against the live engine server.
- `rust/allwright-client` now hides the engine transport behind a lazy singleton connection and exposes `launch_chrome`, `ping`, `Browser`, and `Tab` methods instead of public tonic setup.
- The Rust singleton client currently defaults to `http://127.0.0.1:50051`, also respects `ALLWRIGHT_SERVER_ADDR`, and can be redirected in-process with `allwright_client::set_server_addr(...)`.
- `rust/playground` now exercises the engine through the `allwright-client` crate in `rust/allwright-client`, supports the launch-created initial tab plus additional browser-session tabs, and closes them through the high-level browser/tab API as a minimal end-to-end test flow.
- `rust/playground` waits for keyboard confirmation before closing the browser session so browser state can be observed manually.
- `go/client` now uses package name `allwright`, hides the engine transport behind a lazy singleton connection, and exposes browser/tab methods instead of public gRPC dial APIs.
- The Go singleton client currently uses `ALLWRIGHT_SERVER_ADDR` or falls back to `127.0.0.1:50051`.
- `go/cmd/playground` is the Go-side playground and currently exercises a minimal browser-session test flow directly, without extra ping-style smoke commands.
- `java/src/main/java/dev/allwright/client/Allwright.java` now provides the Java client, hides the engine transport behind a lazy singleton connection, and exposes Playwright-style `chromium.launch(...)`, `Browser`, and `Page` methods instead of public gRPC setup.
- The Java singleton client currently uses `ALLWRIGHT_SERVER_ADDR` or falls back to `127.0.0.1:50051`, and supports in-process override with `Allwright.setServerAddr(...)`.
- `python/allwright/client.py` now provides the Python client, hides the engine transport behind a lazy singleton connection, and exposes Playwright-style `chromium.launch()`, `Browser`, and `Page` methods instead of public grpc setup.
- The Python singleton client currently uses `ALLWRIGHT_SERVER_ADDR` or falls back to `127.0.0.1:50051`, and supports in-process override with `set_server_addr(...)`.
- `allwright-dev/package.json` currently keeps Bun as the package manager declaration but uses standard `next dev`, `next build`, and `next start` scripts.
- `allwright-dev/vercel.json` currently pins Vercel site behavior to the Next.js framework with `npm install` and `npm run build` so deployments avoid the Bun build crash path.
- `typescript/src/index.ts` now provides the TypeScript/JavaScript client, hides the engine transport behind a lazy singleton connection, and exposes Playwright-style `chromium.launch(...)`, `Browser`, and `Page` methods instead of public grpc-js setup.
- New TypeScript work should prefer the `chromium` / `Browser` / `Page` surface; older compatibility helpers like `launchChrome`, `initialTab()`, `newTab()`, and `navigate()` should be treated as transitional.
- The TypeScript singleton client currently uses `ALLWRIGHT_SERVER_ADDR` or falls back to `127.0.0.1:50051`, and also supports in-process override with `setServerAddr(...)`.
- `typescript/src/playground.ts` is the TypeScript-side playground for a minimal browser-session test flow; use Bun for local development runs in this repo and keep it focused on real browser work rather than extra ping-style smoke commands.
- Regenerating Go stubs currently requires local installation of `protoc-gen-go@v1.36.10` and `protoc-gen-go-grpc@v1.5.1`, then running `protoc` against `proto/engine/v1/engine.proto` with output rooted at `go/`.

## Current Dependency Hierarchy

```text
proto/
└── engine/v1/engine.proto

rust/allwright
└── rust/engine-lib
    ├── uses rust/mobile-lib
    │   ├── rust/ios-lib
    │   └── rust/android-lib
    ├── uses rust/desktop-lib
    │   ├── rust/windows-lib
    │   ├── rust/macos-lib
    │   └── rust/linux-lib
    └── uses rust/web-lib
```

## Maintenance Rules

- Keep `README.md` and `Codex.md` aligned.
- Prefer documenting ownership changes here at the same time as code changes.
- Do not move engine logic out of `rust/engine-lib`.
- Do not collapse platform-specific responsibilities back into `allwright`.
- Keep async boundaries explicit and Tokio-native.
- Keep the gRPC contract minimal until requirements are explicitly defined.
- Preserve the driverless direction when evolving browser and tab session APIs.
