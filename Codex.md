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
- `packages/`
- `proto/`
- `scripts/`
- `xtask/`
- `rust/allwright`: `allwright-core`, the Rust engine crate containing the high-level Rust client API, the single gRPC server host, the runtime plugin loader, and the shared plugin catalog
- `rust/allwright/src/plugin_runtime.rs`: shared core-owned plugin installation, version resolution, manifest management, release-asset download, and dynamic library invocation logic used by both the engine and the CLI
- `rust/allwright-cli`: installable `allwright` CLI package that depends on `allwright-core`; plugin install/list/invoke should delegate to core-owned plugin runtime logic instead of reimplementing install/version rules in the CLI
- `.github/workflows/release-surface-plugins.yml`: tag-triggered GitHub Actions workflow for root `vX.Y.Z` releases; it syncs versions from the tag, publishes Go/Python/npm/Rust packages, builds platform CLI/plugin archives, and attaches them to GitHub Releases
- `.github/workflows/release-surface-plugins.yml` now also publishes the Java client to Maven Central via the checked-in Gradle wrapper using Sonatype Central Portal token credentials in `OSSRH_USERNAME` / `OSSRH_PASSWORD`, plus `SIGNING_KEY` and `SIGNING_PASSWORD`
- `.github/workflows/ci.yml`: CI workflow that regenerates Rust proto bindings and fails if committed generated files are out of date
- `.github/workflows/ci.yml` now also runs the Java client tests through the checked-in Gradle wrapper on Java 21
- `scripts/install.sh`: Linux/macOS installer script for downloading the published `allwright` CLI release asset directly from GitHub without cloning the repo
- `scripts/install.ps1`: Windows PowerShell installer script for downloading the published `allwright` CLI release asset directly from GitHub without cloning the repo
- `scripts/generate-go-proto.sh`: repo-level helper that installs pinned Go protobuf generators under `go/.bin/` and regenerates the checked-in Go bindings from the canonical top-level `proto/` tree
- `scripts/generate-rust-proto.sh`: repo-level helper that regenerates Rust bindings from the canonical top-level `proto/` tree
- `scripts/sync-version.sh`: helper script that rewrites the workspace and internal crate versions from a provided release version
- `scripts/sync-npm-version.sh`: helper script that rewrites the npm workspace package versions from a provided release version and keeps `@allwright.dev/vitest` aligned to `@allwright.dev/core`
- `scripts/sync-python-version.sh`: helper script that rewrites the Python package version from a provided release version
- `xtask/`: internal workspace utility crate for repo maintenance commands such as Rust proto regeneration
- `rust/allwright-plugin-sdk`: shared plugin ABI, metadata, and request/response types for runtime-loaded Rust surface plugins
- `rust/allwright-surface-web`: publishable `web` surface crate that ships a runtime-loadable shared library (`cdylib`)
- `rust/allwright-surface-mobile`: shared mobile surface abstractions consumed by `mobile-android` and `mobile-ios`
- `rust/allwright-surface-mobile-android`: publishable `mobile-android` surface crate; the current Android runtime is native Rust plus `adb` and must not depend on Python bridge scripts or a separate `uiautomator2` Python environment
- `rust/allwright-surface-mobile-ios`: publishable `mobile-ios` surface crate
- `rust/allwright-surface-desktop`: shared desktop surface abstractions consumed by `desktop-mac`, `desktop-windows`, and `desktop-linux`
- `rust/allwright-surface-desktop-mac`: publishable `desktop-mac` surface crate
- `rust/allwright-surface-desktop-windows`: publishable `desktop-windows` surface crate
- `rust/allwright-surface-desktop-linux`: publishable `desktop-linux` surface crate
- `rust/allwright/examples/`: Rust example programs for exercising the lightweight engine core
- `go/`: Go module `allwright.dev` containing generated engine stubs, the public Go client package at the module root, and Go examples
- `java/`: Gradle-based Java client project that generates engine stubs from the shared proto and exposes a high-level browser/page API
- `java/build.gradle.kts`: Java build now also owns Maven Central publication metadata for `dev.allwright:allwright`, conditional OSSRH publishing credentials, in-memory signing support, and JUnit 5 test wiring
- `python/`: Python client package `allwright` that exposes a high-level browser/page API and bundles the shared proto files inside `python/allwright/proto/` for publishable runtime loading
- `allwright-dev/`: standalone Next.js site for the purchased `allwright.dev` domain, intended for Vercel deployment and public-facing marketing/docs entrypoints
- `typescript/core/`: published npm package `@allwright.dev/core`, containing the TypeScript/JavaScript client, bundled shared proto files, and examples
- `typescript/vitest/`: published npm package `@allwright.dev/vitest`, containing Vitest fixtures and retrying browser assertions on top of `@allwright.dev/core`
- `typescript/vitest/` now also owns mobile-friendly fixtures such as `android` and `androidApp`, and should support hybrid tests that use both web and Android in the same test run
- `proto/`: shared protobuf and gRPC contract root for all stacks
- `proto/engine/v1/engine.proto`: umbrella engine service contract that imports the split proto ownership layers
- `proto/core/v1/`: core-owned shared engine/session/browser/tab message contracts
- `proto/surfaces/web/v1/`: web surface-owned command and event contracts
- `rust/allwright` now contains the lightweight Rust engine/client implementation and compiles against the shared top-level `proto/` contract during builds
- `rust/allwright/src/proto_generated.rs` is a checked-in shim module for publishability, and `rust/allwright/src/allwright.engine.v1.rs` is the checked-in generated Rust output; both must be regenerated from the top-level `proto/` tree via `./scripts/generate-rust-proto.sh` rather than edited by hand

## Runtime Model

- The entire project should remain Tokio async runtime driven.
- The CLI enters through `#[tokio::main]`.
- The Rust workspace publishable crates should be synced from the release tag when GitHub Actions builds a tagged release.
- The npm workspace packages should also be synced from the release tag when GitHub Actions builds a tagged release.
- The Python package should also be synced from the release tag when GitHub Actions builds a tagged release.
- Installing `allwright` should mean installing the released CLI binary plus the lightweight core behavior it owns; it should not require Cargo on end-user machines.
- `rust/allwright` owns the single gRPC server host and public Rust client surface in one package.
- Engine behavior and engine-facing Rust contracts should stay inside `rust/allwright`.
- allwright should remain a single engine even as the product grows broader capabilities.
- Surface capabilities should be designed as separately installable plugins such as `web`, `mobile-android`, `mobile-ios`, `desktop-mac`, `desktop-windows`, and `desktop-linux`, all extending the core engine instead of creating separate engines or unrelated runtimes.
- Plugin discovery, install, version matching, and dynamic loading should be lazy and usage-triggered. Core server startup must stay surface-agnostic and must not eagerly discover, install, register, or load `web`, `mobile-android`, or any other surface plugin.
- Surface usage should trigger plugin resolution: web commands such as `launchBrowser(...)` should resolve `web`; Android commands such as `mobile.android.connect(...)` or plugin invocation should resolve `mobile-android`.
- The engine should own plugin resolution. Clients should only ensure the engine/CLI is available and then send commands; they should not duplicate plugin install/version logic per language.
- Hard rule: no client may invoke a plugin directly. Clients must talk only to the allwright server over the shared engine transport.
- Hard rule: plugin ids, plugin request payloads, plugin ABI loading, and plugin process/library invocation are core concerns only. Those details must not appear in Go, Java, Python, TypeScript, or future client libraries.
- The engine owning a session does not mean the engine owns surface behavior. Core should be glue only: accept client commands, resolve the target plugin, delegate execution, and relay results/events back through shared sessions.
- Surface semantics and execution must stay in plugins. Selector interpretation, locator behavior, browser automation, ADB/device work, CDP/BiDi interactions, screenshots, input synthesis, and other surface-specific logic must not migrate into core.
- Core-owned plugin resolution must preserve version matching: use `ALLWRIGHT_VERSION` when set, resolve `latest` through GitHub Releases, otherwise fall back to the shipped package version for the current core build, and only reuse an installed plugin when the manifest version matches the resolved target version.
- If a required plugin is absent and auto-install is enabled, core should download the matching release asset into the local allwright plugin directory, update the local plugin manifest, and then load the plugin. If auto-install is disabled, the usage path should fail with a clear plugin-required error.
- The CLI install path should be release-asset based, not `cargo install` based.
- Runtime plugins should be true shared libraries loaded by the core at runtime, not subprocess executables and not compile-time linked surface implementations.
- The `web` plugin should be packaged under `lib/` in the release archive, and the core plugin loader should resolve platform-specific library names such as `.dylib`, `.so`, and `.dll`.
- `mobile-android` is now also an installable runtime surface alongside `web`; other non-web surface crates should remain visible in the plugin catalog but stay non-installable until they ship real runtime artifacts.
- GitHub tag pushes such as `vX.Y.Z` should trigger release automation that syncs crate/package versions from the tag, publishes registries, builds the current web plugin runtime archives for the supported OS matrix, and uploads them to the matching GitHub Release.
- GitHub tag pushes such as `vX.Y.Z` should also build and upload the main `allwright` CLI archives so installer scripts can bootstrap core plus CLI without Cargo.
- The release workflow currently publishes:
- Go by creating a secondary `go/vX.Y.Z` tag from the root release tag and warming the Go proxy
- Python by syncing `python/` to the release version, building from `python/`, and using PyPI trusted publishing with the GitHub `pypi` environment
- npm by syncing the workspace package versions, running the root workspace build, and publishing `@allwright.dev/core` plus `@allwright.dev/vitest` through npm trusted publishing with the GitHub `Prod` environment
- Rust crates by syncing workspace Cargo versions and running `scripts/publish-crates.sh publish web` with `CARGO_REGISTRY_TOKEN`; the workflow currently sets `CARGO_PUBLISH_ALLOW_DIRTY=1` because version sync edits manifests during the job
- The release tag is the source of truth for release versioning; committed Cargo manifest versions may be rewritten during release automation.
- Unix and Windows installer scripts should prefer common human-owned install directories first and should avoid tool-managed bins such as `pnpm`, `npm`, `yarn`, `cargo`, `volta`, `bun`, and similar package-manager-owned paths.
- Installer examples in docs should default to direct GitHub script execution (`curl`, `wget`, or PowerShell `irm`) rather than assuming the repo has already been cloned.
- The engine direction is driverless browser automation.
- Do not introduce ChromeDriver into the primary browser control path.
- The current gRPC layout lives in top-level `proto/`.
- The proto contract is now split by ownership: `engine/v1` is the umbrella service surface, `core/v1` owns shared engine/session messages, and `surfaces/web/v1` owns the current web-specific messages.
- The current proto package is `allwright.engine.v1`.
- The current starter service is `EngineService`.
- Generated gRPC client code is enabled and currently consumed by `rust/allwright` and its examples.
- Generated Go proto/gRPC code is checked in under `go/gen/allwright/engine/v1` and consumed by the Go module.
- The Java stack generates protobuf and gRPC stubs from `proto/engine/v1/engine.proto` during the Gradle build in `java/`.
- The Java build is now publication-ready for Maven Central under `dev.allwright`, with `publishAllPublicationsToCentralPortalRepository` / `publishToMavenLocal` using `ALLWRIGHT_VERSION` or Gradle property `allwrightVersion`, and signing sourced from `SIGNING_KEY` / `SIGNING_PASSWORD` or matching Gradle properties.
- The tagged release workflow now uses `java/gradlew -p java publishAllPublicationsToCentralPortalRepository` on Java 21, then `POST`s to Sonatype's `/manual/upload/defaultRepository/dev.allwright?publishing_type=automatic` endpoint from the same job so the deployment is transferred from the Central Portal OSSRH Staging API compatibility service into the Central Publisher Portal.
- The Python stack currently loads the bundled package-local proto files through `grpcio-tools`; published consumers must not depend on `../proto` existing outside the wheel.
- The TypeScript/npm stack ships package-local copies of the shared top-level proto tree inside `typescript/core/proto/` rather than depending on repo-relative paths after publish.
- The public Rust client surface should stay high-level and should not expose raw gRPC connection setup.
- The public Go client surface should stay high-level and should not expose raw gRPC connection setup.
- The public Java client surface should stay high-level and should not expose raw gRPC connection setup.
- The public Python client surface should stay high-level and should not expose raw gRPC connection setup.
- The public TypeScript client surface should stay high-level and should not expose raw grpc-js client setup.
- Client libraries should prefer small physical source files with focused responsibilities as they grow; keep public `index` / barrel / aggregator files thin, and move concrete implementations into dedicated modules or per-class files where the language conventions allow it.
- Use Bun for local development workflows in the TypeScript stack, but keep the client surface generic for TypeScript/JavaScript consumers.
- Use Bun as the primary local development workflow for `allwright-dev/`.
- `allwright-dev/` should stay on the stable Next.js 16 release line and use Tailwind CSS v4 through the official PostCSS integration unless there is a deliberate migration decision.
- `allwright-dev/` is user-facing marketing/product surface, not developer-facing repo messaging.
- The core product message for `allwright-dev/` is that allwright is one automation engine for all automation needs.
- The GitHub repository was transferred; the canonical remote and all in-repo/site links now point at `https://github.com/allwright-dev/allwright` (previously `qalens/allwright`).
- All copy on `allwright-dev/` must read as user-facing product messaging (audience, benefits, surfaces covered); it must never describe internal implementation, architecture, or repo/dev workflow details.
- `allwright-dev/` is now multi-page: `app/page.tsx` is the marketing home page (hero, honest per-surface status pills, benefits), `app/how-it-works/page.tsx` is a dedicated "how it works" page that explains the one-engine model in user-facing terms (before/after comparison diagram, a client-languages → engine → surfaces flow diagram, and a "bring your own language" section linking out to the real example path per language in the GitHub repo), and `app/availability/page.tsx` is a dedicated "availability" page that spells out, capability by capability, what the web surface can and can't do yet (a plain "available now" / "not yet available" list, deliberately not naming any protocol/transport), alongside the same per-surface and per-language status shown elsewhere.
- `allwright-dev/app/availability-data.ts` is the single source of truth for `surfaceStatus` and `languages` data, imported by both `app/how-it-works/page.tsx` and `app/availability/page.tsx` so per-surface and per-language status can't drift between pages; update it, not the page files, when a surface or language status changes.
- Shared page chrome (background/ambient decoration, `SiteHeader`, `SiteFooter`) now lives in `app/layout.tsx` via `app/site-header.tsx` and `app/site-footer.tsx`, instead of being duplicated per page; `app/site-header.tsx` is a client component that highlights the active nav link with `usePathname`.
- Per-surface status on the site must stay honest: only Web is "Available now"; Mobile (Android & iOS), Desktop (macOS, Windows, Linux), and API are all "Not yet available" until they ship real runtime artifacts, matching the plugin-catalog status above. Per-language client status must also stay honest: Rust, Go, and TypeScript are "Published" (installable via normal package tooling — `@allwright.dev/core` is live on npm); Java and Python are "From source" (complete, but not yet on a package registry) — reflect this with the shared `app/status-pill.tsx` component, not ad hoc badges.
- "Available now" for Web is about the plugin being real and installable, not about feature completeness — `app/availability/page.tsx`'s `webAvailable` / `webNotYetAvailable` lists are the honest, capability-level detail behind that pill, and must be updated in the same change that adds or removes a client-facing web capability (e.g. a new selector action ships, or screenshots/file upload/network interception/etc. land) so the page never trails the real command surface.
- Core site messaging must foreground the plugin model accurately: allwright is a small core plus installable plugins ("à la carte, not a buffet") — never messaged as one monolithic engine that natively does everything. The plugin catalog shown on the site (web, mobile-android, mobile-ios, desktop-mac, desktop-windows, desktop-linux) is today's set, not a fixed/final one — copy and diagrams must say more plugins get added as new surfaces ship, not imply a closed count.
- The site must also message that allwright is built from the ground up on its own engine — not an aggregator wrapping other automation tools' drivers/frameworks under one CLI. This appears on both `app/page.tsx` (hero + a "Built from the ground up" benefit) and `app/how-it-works/page.tsx` (the plugin-catalog section intro and the "Built from the ground up, not stitched together" section).
- `app/how-it-works/page.tsx` includes a hand-drawn plugin-catalog diagram (`PluginCatalogDiagram`) laying out the core plus each real plugin-catalog entry in a hexagon, solid/installed vs. dashed/not-yet-available, plus a verified CLI snippet (`allwright plugin list`, `allwright plugin install web` — from `rust/allwright-cli`'s real clap subcommands). Keep any future plugin-model diagram/copy in sync with the actual CLI subcommand names if those change.
- `app/sitemap.ts` must list every public page (currently `/`, `/how-it-works`, and `/availability`); add new routes there when adding pages.
- `middleware.ts` implements the `go-get=1` Go module metadata response for the `allwright.dev` vanity import path and must keep matching all routes (`matcher: ["/", "/:path*"]`); do not narrow this when adding new pages/routes.
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
  - `SurfaceSession` as a bidirectional stream
  - `ContextSession` as a bidirectional stream
- `SurfaceSession` carries engine-owned cross-surface lifecycle commands such as web launch, mobile connect, and mobile app launch. This is orchestration and delegation only; the actual browser/device/app work must still happen inside the matching surface plugin.
- The Android client paths across Rust, Go, Java, Python, and TypeScript now route through engine-owned `SurfaceSession` and `ContextSession` RPCs. Clients must remain server-only, and any future surface capability should follow the same pattern instead of adding direct plugin side channels.
- Direct plugin invocation that still exists in non-Rust clients is architectural debt and should be removed. The correct fix is always to extend the server protocol/session model and keep clients server-only.
- The first browser command is `LaunchChromeCommand`.
- `LaunchChromeCommand` is only for opening a plain browser window.
- `LaunchChromeCommand` also returns `initial_page_session_id` for the startup page that Chrome already opened.
- Browser launch commands must not carry URL/navigation input.
- `OpenContextCommand` opens a new context from `SurfaceSession` and returns `context_session_id`.
- `OpenContextCommand` is only valid after the parent surface session has launched.
- `ContextSession` is the follow-up context-scoped stream keyed by `surface_session_id` plus `context_session_id`.
- `ContextSession` must validate that the context belongs to the specified surface session.
- The startup Chrome page must be tracked like any later page and accept the same context-stream commands.
- Future context launch commands must also not carry URL/navigation input.
- Navigation is a separate `NavigatePageCommand` on `ContextSession`.
- Click is a separate `ClickElementCommand` on `ContextSession`.
- Future context creation and context-level communication should be modeled as separate commands inside the surface session stream.
- Browser launch will likely need CDP enabled so the engine can attach to the surface session.
- The implemented web plugin runtime path enables CDP and surfaces browser-level connection metadata.
- The implemented web plugin runtime path tracks browser sessions with engine session ids and associated CDP session information.
- The implemented web plugin runtime path opens tabs through CDP `Target.createTarget`, not GUI scripting.
- The implemented web plugin runtime path discovers the initial launch-created tab through CDP `Target.getTargets`.
- The implemented web plugin runtime path runs tab navigation through CDP `Target.attachToTarget` plus `Page.navigate`.
- The implemented web plugin runtime path waits for `Page.loadEventFired`, then injects a pinned `chromium-bidi` mapper artifact into a hidden mapper target in the same browser session.
- Clear separation of concerns is required: core should own only the generic engine host, plugin discovery/loading, shared session transport, and plugin-missing errors; all web-specific behavior, state, CDP, and BiDi logic belongs in the `web` plugin.
- The same separation applies to mobile: core owns only generic engine host, lazy plugin resolution, shared transport, version matching, and plugin loading; all Android-specific device discovery, ADB interaction, selector matching, source dumping, and native input belong in `mobile-android`.
- Session ownership in core should remain orchestration-only. Core may track generic surface/context session ids and route commands, but it must not grow surface-aware locator engines, selector parsers, automation backends, or device/browser control code.
- `mobile-android` now follows that intended separation in practice: core-owned surface/context routing invokes the plugin, while Android-specific ADB/runtime behavior remains inside the plugin.
- The injected mapper is sourced from the published `chromium-bidi@17.0.2` package but is checked into `rust/allwright-surface-web/third_party/chromium-bidi/17.0.2/` so the publishable web surface crate does not depend on `npm`.
- The intended web plugin path persists a real `bidi_session_id` plus mapper target/session ids after mapper injection.
- The implemented web surface crate includes selector-based click through `script.evaluate` on the tab browsing context.
- Chrome launching logic currently lives in `allwright-surface-web`, uses the browser binary directly, and discovers the CDP WebSocket endpoint via `DevToolsActivePort`.
- Shared surface crates support plugin families, while leaf surface crates provide the publishable install targets.
- `rust/allwright` now hides the engine transport behind a lazy singleton connection and exposes `launch_chrome`, `ping`, `Browser`, `Tab`, and plugin catalog metadata from the lightweight core package.
- The Rust singleton client currently defaults to `http://127.0.0.1:50051`, also respects `ALLWRIGHT_SERVER_ADDR`, can be redirected in-process with `allwright::set_server_addr(...)`, and should only ensure the matching CLI/server path for local addresses; plugin install/load is now core-owned and lazy.
- `rust/allwright/examples/playground.rs` now exercises the engine through the `allwright` crate, supports the launch-created initial tab plus additional browser-session tabs, and closes them through the high-level browser/tab API as a minimal end-to-end test flow.
- The Rust playground example waits for keyboard confirmation before closing the browser session so browser state can be observed manually.
- `go/` now uses module path `allwright.dev`, exposes package name `allwright` from the module root, and relies on `allwright.dev` vanity import metadata that points Go tooling at the GitHub repository subdirectory `go/`.
- The Go singleton client currently uses `ALLWRIGHT_SERVER_ADDR` or falls back to `127.0.0.1:50051`, and should only ensure the matching CLI/server path for local addresses; plugin install/load is now core-owned and lazy.
- `go/examples/playground` is the Go-side example and currently exercises a minimal browser-session test flow directly, without extra ping-style smoke commands.
- The Java client now uses a multi-file layout with top-level public API types: `java/src/main/java/dev/allwright/client/Allwright.java` is the launcher/config facade, `BrowserType.java` owns browser-type launch ergonomics, `Browser.java` owns browser-session behavior, `Page.java` owns page/tab behavior, `Locator.java` owns locator delegation, `LaunchOptions.java` / `RetryConfig.java` / `ResolvedConfig.java` and the other option/result records each live in dedicated files, `SelectorSupport.java` owns selector transport encoding, `CommandSupport.java` owns command retry helpers, `ConfigSupport.java` owns config parsing/merge helpers, and `RuntimeSupport.java` owns the lazy gRPC runtime and stream utility types; do not collapse these back into `Allwright.java`.
- The Java `Browser` and `Page` types now implement `AutoCloseable` for try-with-resources usage, `Browser.pages()` mirrors the other client libraries, and `Page` now includes the same locator-backed selector actions (`count`, `highlight`, `focus`, `fill`, `hover`, `press`, `textContent`, `innerText`, `waitForSelector`) that `Locator` already expected.
- The Java singleton client currently uses `ALLWRIGHT_SERVER_ADDR` or falls back to `127.0.0.1:50051`, supports in-process override with `Allwright.setServerAddr(...)`, and should only ensure the matching CLI/server path for local addresses; plugin install/load is now core-owned and lazy.
- `python/allwright/client.py` now provides the Python client, hides the engine transport behind a lazy singleton connection, and exposes Playwright-style `chromium.launch()`, `Browser`, and `Page` methods instead of public grpc setup.
- The Python client now uses a multi-file layout: `python/allwright/client.py` stays a thin public assembly layer while implementation classes and transport/config helpers live in focused sibling modules.
- The Python singleton client currently uses `ALLWRIGHT_SERVER_ADDR` or falls back to `127.0.0.1:50051`, supports in-process override with `set_server_addr(...)`, and should only ensure the matching CLI/server path for local addresses; plugin install/load is now core-owned and lazy.
- `allwright-dev/package.json` currently keeps Bun as the package manager declaration but uses standard `next dev`, `next build`, and `next start` scripts.
- `allwright-dev/vercel.json` currently pins Vercel site behavior to the Next.js framework with `npm install` and `npm run build` so deployments avoid the Bun build crash path.
- `typescript/core/src/index.ts` now provides the TypeScript/JavaScript client, hides the engine transport behind a lazy singleton connection, and exposes Playwright-style `chromium.launch(...)`, `Browser`, `Page`, and `Locator` methods instead of public grpc-js setup.
- The TypeScript client now uses that same structure: `typescript/core/src/index.ts` stays a thin public barrel, while browser/page/locator/runtime/config/selector logic lives in dedicated files under `typescript/core/src/`.
- Mobile selector transport must mirror the web client shape as closely as possible while remaining mobile-native: client-facing selectors should support explicit prefixes such as `xpath=...`, `css=...`, `id=...`, and UiAutomator-style keys such as `text=...`, `textContains=...`, `resourceId=...`, `descriptionContains=...`, `classNameMatches=...`, boolean flags, `index=...`, and `instance=...`.
- Android locator chaining should follow the same mental model as web locators. Clients should expose `androidApp.locator(...).locator(...)` style chaining, and the mobile transport/plugin should preserve chained selector segments rather than collapsing everything into ad hoc string parsing.
- The Go client now uses a multi-file layout with focused responsibilities: runtime connection state lives in `go/runtime.go`, launch/configured-browser entrypoints in `go/launch.go`, browser-session behavior in `go/browser.go`, tab/page behavior in `go/page.go` plus `go/page_session.go`, locator delegation in `go/locator.go`, command option/proto helpers in `go/command_options.go`, shared public types in `go/types.go`, config loading in `go/config.go`, and selector transport helpers in `go/selectors.go`.
- The Rust client now uses a multi-file layout with focused responsibilities: `rust/allwright/src/client.rs` is a thin assembly/re-export module, shared public client types live in `rust/allwright/src/client_types.rs`, selector transport helpers live in `rust/allwright/src/client_selectors.rs`, runtime singleton/address management lives in `rust/allwright/src/client_runtime.rs`, config discovery/loading/merge logic lives in `rust/allwright/src/client_config.rs`, browser launch logic lives in `rust/allwright/src/client_launch.rs`, browser-session behavior lives in `rust/allwright/src/client_browser.rs`, tab core/session lifecycle lives in `rust/allwright/src/client_tab.rs`, tab element actions live in `rust/allwright/src/client_tab_actions.rs`, tab query/read operations live in `rust/allwright/src/client_tab_query.rs`, locator delegation lives in `rust/allwright/src/client_locator.rs`, and shared command/result helpers live in `rust/allwright/src/client_command.rs`.
- The shared stack-agnostic config contract now lives at the repo root in `allwright.schema.json`, with `allwright.config.yaml` as the preferred human-authored format and `allwright.config.json` also supported against the same schema.
- `typescript/core/src/index.ts` is the first JavaScript loader for that shared config model and exposes helpers such as `findConfigFile()`, `loadConfigFile()`, `resolveConfig()`, and `launchConfiguredBrowser()`.
- New TypeScript work should prefer the `chromium` / `Browser` / `Page` surface; older compatibility helpers like `launchChrome`, `initialTab()`, `newTab()`, and `navigate()` should be treated as transitional.
- The TypeScript singleton client currently uses `ALLWRIGHT_SERVER_ADDR` or falls back to `127.0.0.1:50051`, also supports in-process override with `setServerAddr(...)`, and should only ensure the matching CLI/server path for local addresses; plugin install/load is now core-owned and lazy.
- The repo root `package.json` is now a private npm workspace root for `typescript/core` and `typescript/vitest`; published package metadata lives in each workspace package, not at the repo root.
- `typescript/core/examples/playground.ts` is the TypeScript-side example for a minimal browser-session test flow; use Bun or npm workspaces for local development runs in this repo and keep it focused on real browser work rather than extra ping-style smoke commands.
- Generated TypeScript build output under `typescript/**/dist/` and per-package `.tsbuildinfo` files must not be checked in; they belong in `.gitignore` and should only exist as local/publish artifacts unless the repo explicitly adopts a checked-in-dist policy later.
- Bun is the preferred TypeScript/JavaScript development toolchain in this repo: use `bun install` and `bun run ...` for local workspace development, keep `bun.lock` as the checked-in lockfile, and do not reintroduce `package-lock.json` except for a deliberate tooling-policy change. npm remains acceptable only where the registry transport itself requires it, such as `npm publish`.
- `@allwright.dev/vitest` provides ready-made Vitest fixtures (`browser`, `page`, `allwright`, `android`, `androidApp`) plus a custom retrying `expect` surface aimed at Playwright-style ergonomics. Fixture-level defaults belong here, not in the core SDK clients.
- `@allwright.dev/vitest` is now the first consumer of that shared config model: it auto-resolves `allwright.config.yaml`, `allwright.config.yml`, or `allwright.config.json`, supports shared retry defaults plus per-suite selection through `allwright.suite`, and merges test-level overrides on top instead of inventing a Vitest-only config file shape.
- The current retrying Vitest assertions support both `expect(page)` and `expect(page.locator(...))` forms for text, count, and visibility checks; they are intentionally implemented in the npm test helper layer rather than in the engine protocol for now.
- The Android Vitest fixtures currently default to a longer setup window than web-only flows: `android` connect defaults to `30_000ms`, and `androidApp` launch defaults to `60_000ms`, while explicit `timeoutMs` values still win.
- The other first-party clients currently do not have a fixture layer, so they should not invent Vitest-style fixture defaults. For non-Vitest stacks, timeout defaults should come either from explicit command options, shared core API conventions, or future test-framework-specific helper layers.
- Mobile command ergonomics should preserve Playwright-style retry/timeout expectations where the protocol allows it. Android commands such as `click` and `fill` should honor `timeoutMs`, and future Android locator assertions should preserve the same retry-first mental model as web rather than exposing one-shot raw device calls.
- Regenerating Go stubs should go through `./scripts/generate-go-proto.sh`, which installs pinned `protoc-gen-go@v1.36.10` and `protoc-gen-go-grpc@v1.5.1` into `go/.bin/` and rewrites the checked-in Go generated files under `go/gen/allwright/engine/v1/`.
- Regenerating Rust stubs should go through `./scripts/generate-rust-proto.sh`, which runs `cargo run -p xtask -- generate-rust-proto` against the top-level `proto/` tree and rewrites the checked-in Rust generated files under `rust/allwright/src/`.
- `.github/workflows/ci.yml` verifies Go and Rust proto regeneration by rerunning `./scripts/generate-go-proto.sh` and `./scripts/generate-rust-proto.sh`; generated files must stay committed and in sync, including `rust/allwright/src/allwright.engine.v1.rs`.
- Before finishing any change, review newly created local artifacts and update `.gitignore` when needed; this is especially important for Java/Gradle outputs such as `java/bin/`, `java/build/`, `.gradle-user-home/`, and similar generated directories.
- Browser launch is being generalized away from Chrome-only protocol naming:
- `proto/surfaces/web/v1/web.proto` now defines `BrowserKind`, `LaunchBrowserCommand`, and `BrowserLaunchedEvent`
- `proto/core/v1/browser.proto` routes browser-session launch through the neutral `launch_browser` / `browser_launched` path while keeping the older Chrome path for compatibility
- `rust/allwright-plugin-sdk` now exposes neutral `PluginCommand::LaunchBrowser`, `PluginResult::LaunchBrowser`, `BrowserKind`, and `BrowserLaunchInfo`
- `rust/allwright-surface-web` now implements both Chromium and Firefox on the neutral launch path: Chromium still uses CDP plus the pinned `chromium-bidi` mapper, while Firefox launches the browser binary directly and talks to its native WebDriver BiDi Remote Agent without `geckodriver`
- `rust/allwright/src/engine.rs` now stores opaque backend browser/page handles from the web plugin instead of top-level `cdp_websocket_url` / `target_id` fields, while the public gRPC events remain compatibility-shaped for existing clients
- `rust/allwright/src/plugin_loader.rs` and `rust/allwright-plugin-sdk` now expose backend-neutral browser/page operations (`LaunchBrowser`, `OpenPage`, `NavigatePage`, selector actions, etc.) alongside the older Chromium-specific compatibility commands
- `rust/allwright/src/client.rs` now exposes `launch_browser(...)` plus `launch_firefox(...)`, and Firefox launches through the same high-level `Browser` / `Tab` API
- `typescript/core/src/index.ts`, `python/allwright/client.py`, and `java/src/main/java/dev/allwright/client/Allwright.java` now expose `firefox` / `launch_firefox` entrypoints on top of the neutral launch command while still preserving the older Chromium entrypoints

## Current Dependency Hierarchy

```text
proto/
└── engine/v1/engine.proto

rust/allwright-plugin-sdk
rust/allwright-surface-web
├── library surface helpers
└── runtime-loadable shared library
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
├── single engine server host
├── runtime plugin loader
└── plugin catalog metadata

rust/allwright-cli
├── installable `allwright` binary
└── plugin installation

.github/workflows/ci.yml
└── verify generated Rust proto files are up to date

.github/workflows/release-surface-plugins.yml
└── publish registries, create Go sub-tags, and build/upload release archives for the CLI and installable plugin libraries

scripts/install.sh
└── download and install the released `allwright` CLI on Linux/macOS, preferring common writable bin directories

scripts/install.ps1
└── download and install the released `allwright` CLI on Windows, preferring common writable install locations

scripts/sync-version.sh
└── sync workspace and internal crate versions from a release tag version before release builds

scripts/sync-npm-version.sh
└── sync npm workspace package versions from a release tag version before npm publish

scripts/sync-python-version.sh
└── sync the Python package version from a release tag version before PyPI publish

xtask/
└── repo maintenance commands including Rust proto regeneration
```

## Maintenance Rules

- Keep `README.md` and `Codex.md` aligned.
- Before finishing any change, review newly created local artifacts and update `.gitignore` when needed; this is especially important for Java/Gradle outputs such as `java/bin/`, `java/build/`, `.gradle-user-home/`, and similar generated directories.
- Never hand-edit generated Go proto files under `go/gen/allwright/engine/v1/` or generated Rust proto files under `rust/allwright/src/`; regenerate them from the top-level `proto/` tree.
- The top-level `proto/` directory is the single source of truth for engine contracts; do not fork or hand-maintain parallel protocol definitions per language.
- Prefer documenting ownership changes here at the same time as code changes.
- Keep shared engine contracts and client-facing APIs in `rust/allwright`.
- Keep platform-specific runtime logic out of `rust/allwright`; it belongs in the surface crates.
- Prefer adding explicit extension points for surface plugins like `web`, `mobile-android`, `mobile-ios`, `desktop-mac`, `desktop-windows`, and `desktop-linux` over introducing parallel engine implementations.
- Do not reintroduce executable-style surface plugins; runtime-loaded shared libraries are the intended plugin model.
- Prefer release downloads and installer scripts for end-user installation paths; do not reintroduce Cargo as the default user install dependency.
- Keep release versioning tag-driven; if versions need to change, prefer updating the sync/release flow rather than hardcoding new crate versions by hand.
- Keep trusted publishing wired through GitHub environments for npm and PyPI; avoid reintroducing long-lived npm automation tokens when OIDC can be used.
- When publishability and code generation conflict, prefer repo-level generation plus committed outputs over hand-written generated code or duplicated proto sources.
- Keep Firefox work behind the neutral browser-launch model until there is a real backend; do not hardcode new browser support through Chromium-only API names.
- Keep async boundaries explicit and Tokio-native.
- Keep the gRPC contract minimal until requirements are explicitly defined.
- Preserve the driverless direction when evolving browser and tab session APIs.
