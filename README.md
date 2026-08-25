# allwright

allwright is one automation engine for everything you test.

The long-term direction is a single system that can cover web, mobile, desktop, and API automation without forcing teams to stitch together a different tool for every surface. The project is designed so automation can feel consistent across the whole product, not fragmented by platform.

That direction also applies to extensibility: allwright should stay one engine at its core, while surface modules like `web`, `mobile-android`, `mobile-ios`, `desktop-mac`, `desktop-windows`, and `desktop-linux` can be installed separately as plugins instead of fragmenting the runtime into multiple engines.

Right now, allwright is being built in public and the browser automation engine is the first active layer. The current implementation is focused on a driverless Chrome control path backed by CDP and Chromium BiDi, with high-level client libraries for Rust, Go, Java, Python, and TypeScript/JavaScript.

## Why allwright

- One engine instead of a pile of disconnected tools
- One automation model that can eventually span web, mobile, desktop, and API work
- One extensibility model where surface plugins plug into the core engine
- High-level browser and page APIs instead of raw transport plumbing
- Driverless browser control built around CDP and Chromium BiDi
- Shared contracts across the engine and all client stacks

## Current Status

allwright is under active development and not positioned as a finished multi-surface platform yet.

The current stage is:

- the core product direction is broader than browser automation alone
- the first shipped implementation work is centered on the browser engine and its future plugin boundary
- the Rust workspace now separates a lightweight `allwright-core` from the installable `allwright` CLI and surface crates
- the `web` surface now ships as a separately installable runtime plugin library
- the other surface crates already exist as publishable boundaries, but only `web` is currently installable as a standalone runtime artifact
- the public API and internal architecture are still evolving as the project grows toward wider surface coverage

If you are evaluating the repo today, the clearest signal is the direction: allwright is aiming to become a unified automation engine, and browser automation is the first concrete step on that path.

## Direction

allwright is being shaped around a simple idea: teams should not need one framework for web, another for mobile, another for desktop, and a separate story for API validation.

The project direction is to make those surfaces feel like one automation system:

- Web: real browser flows that click, type, and navigate like a person would
- Mobile: the same test logic extended toward native and hybrid apps
- Desktop: automation for full desktop application workflows
- API: backend checks that stay aligned with the same user-facing flows and data

That broader direction matters more than the current implementation footprint. The repo may be browser-first today, but the product purpose is cross-surface automation under one roof.

## Extensibility Direction

allwright should remain a single engine, not a family of separate engines.

As the project grows into more surfaces, extensibility should follow a plugin model:

- the core engine stays responsible for lifecycle, sessions, transport, and the shared automation model
- surface modules such as `web`, `mobile-android`, `mobile-ios`, `desktop-mac`, `desktop-windows`, and `desktop-linux` should be attachable as plugins
- those plugins should be installable separately so users only take on the surfaces they need
- plugin boundaries should extend the engine instead of forcing client libraries to learn different runtimes

In practice, that means future architecture work should prefer a stable engine core with explicit extension points over splitting web, mobile, desktop, or API support into unrelated executables.

## Plugin Ecosystem

For users, the intended install model is simple:

- install `allwright` once to get the CLI plus the lightweight engine core
- add only the surface plugins you need, starting with `web`
- keep one command-line entrypoint and one engine lifecycle, even as more surfaces arrive

Today, the plugin ecosystem looks like this:

- `allwright`: installable CLI package that starts the engine server and manages plugin installation
- `web`: the first installable runtime surface plugin today, loaded by the core at runtime
- `mobile-android`, `mobile-ios`, `desktop-mac`, `desktop-windows`, and `desktop-linux`: planned surface plugins with publishable crate boundaries, but not yet installable runtime artifacts

What `plugin install` means today:

- for `web`, it downloads the matching platform archive from GitHub Releases into the local allwright plugin directory and records the installed plugin in the manifest
- `allwright serve` always starts the engine server
- when `web` is installed, the core engine loads the installed `web` plugin library at runtime for browser/web commands
- when `web` is not installed, browser/web commands fail with a plugin-required error while the core server still runs
- the non-web surface crates are still intentionally behind this installability switch until their runtime binaries are ready

So the user-facing install model is now real for `web`, while the broader multi-surface plugin ecosystem is still being completed.

Release automation today:

- pushing a tag such as `vX.Y.Z` triggers the GitHub Actions release workflow
- that workflow publishes the root TypeScript package to npm as `@allwright.dev/core` using npm Trusted Publishing via GitHub Actions OIDC
- that workflow builds both the `allwright` CLI and `allwright-surface-web` plugin for the current release matrix and uploads the archives to the matching GitHub Release
- `allwright plugin install web` resolves the local OS and architecture, then downloads the matching release asset
- the release workflow syncs the Rust workspace version from the Git tag before building, so the tag is the release source of truth

## Quick Start

Install the CLI:

```bash
curl -fsSL https://raw.githubusercontent.com/allwright-dev/allwright/main/scripts/install.sh | bash
```

or:

```bash
wget -qO- https://raw.githubusercontent.com/allwright-dev/allwright/main/scripts/install.sh | bash
```

If you already have the repo checked out:

```bash
chmod +x ./scripts/install.sh
./scripts/install.sh
```

The installer prefers a writable directory that is already on `PATH`. If your shell still says `command not found: allwright`, export the printed install directory into `PATH` for the current session.

Windows PowerShell:

```powershell
irm https://raw.githubusercontent.com/allwright-dev/allwright/main/scripts/install.ps1 | iex
```

If you already have the repo checked out:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\install.ps1
```

Start the engine core through the CLI:

```bash
allwright serve --listen-addr 127.0.0.1:50051
```

List or install plugins:

```bash
allwright plugin list
allwright plugin install web
```

If you are working from the repo checkout instead:

```bash
cargo run -p allwright -- serve --listen-addr 127.0.0.1:50051
cargo run -p allwright -- plugin list
cargo run -p allwright -- plugin install web
```

Try the Rust playground against the running engine core:

```bash
cargo run -p allwright-core --example playground -- --server-addr http://127.0.0.1:50051
```

Open more tabs during the playground flow:

```bash
cargo run -p allwright-core --example playground -- --server-addr http://127.0.0.1:50051 --tabs 3
```

## What You Can Try Today

Today’s working path is browser-focused, with a Rust-powered engine and high-level client libraries layered on top.

The practical path today is:

- use the `allwright` CLI as the installable entrypoint
- install the `web` plugin
- use the Rust, Go, Java, Python, or TypeScript clients against the running engine server

At the moment, the `web` runtime path is wired through the installable plugin model and loaded into the core at runtime. The other surface crates and split proto ownership are in place, while additional plugin runtime activation is still follow-up work.

## Client Experience

allwright is designed around high-level browser objects rather than asking application code to manage raw gRPC connections.

Rust example:

```rust
let browser = allwright::launch_chrome(Default::default()).await?;
let tab = browser.initial_tab()?;
tab.navigate("https://example.com").await?;
tab.click("a").await?;
browser.close().await?;
```

Go example:

```go
browser, err := allwright.LaunchChrome(ctx, allwright.LaunchOptions{})
tab := browser.InitialTab()
_, err = tab.Navigate(ctx, "https://example.com")
_, err = tab.Click(ctx, "a")
err = browser.Close(ctx)
```

TypeScript example:

```ts
import { chromium } from "./src/index.js";

const browser = await chromium.launch({});
const page = browser.page();
await page.goto("https://example.com");
await page.click("a");
await browser.close();
```

## Repository Guide

- `rust/allwright`: lightweight `allwright-core` Rust package with the client API, proto bindings, and gRPC engine core
- `rust/allwright-cli`: installable `allwright` CLI package that depends on `allwright-core` and installs supported plugins
- `rust/allwright-plugin-sdk`: shared plugin traits and surface metadata
- `rust/allwright-surface-web`: publishable `web` surface crate that ships the first standalone runtime plugin library
- `rust/allwright-surface-mobile`: shared mobile surface abstractions
- `rust/allwright-surface-mobile-android`: publishable `mobile-android` surface crate
- `rust/allwright-surface-mobile-ios`: publishable `mobile-ios` surface crate
- `rust/allwright-surface-desktop`: shared desktop surface abstractions
- `rust/allwright-surface-desktop-mac`: publishable `desktop-mac` surface crate
- `rust/allwright-surface-desktop-windows`: publishable `desktop-windows` surface crate
- `rust/allwright-surface-desktop-linux`: publishable `desktop-linux` surface crate
- `go/`: Go client and Go playground
- `java/`: Java client project
- `python/`: Python client package
- `typescript/`: TypeScript/JavaScript client package and playground
- `proto/`: shared protobuf and gRPC contracts
  The root service entrypoint remains `proto/engine/v1/engine.proto`, while shared core messages now live under `proto/core/v1/` and the web surface messages now live under `proto/surfaces/web/v1/`.
- `allwright-dev/`: public website project for `allwright.dev`

## Development Notes

- The engine currently runs as a gRPC server.
- The current implementation focus is browser automation, but the product direction is wider.
- Installing the `allwright` package is intended to deliver the CLI plus the lightweight engine core together.
- The project should keep a single engine core even as surface modules become separately installable plugins.
- The `web` surface plugin is now installable through the CLI via GitHub Release downloads and is loaded by the core engine at runtime.
- The remaining surface plugins are still intentionally disabled as install targets until their runtime binaries exist.
- The Rust workspace version is synced from the release tag during GitHub release builds.
- The browser control path is intended to stay driverless.
- The repo uses shared proto contracts across all supported client stacks.
- Bun is the preferred local workflow for the TypeScript stack and the `allwright-dev/` site.

## Releasing Plugins

Create and push a version tag:

```bash
git tag vX.Y.Z
git push origin vX.Y.Z
```

That tag triggers `.github/workflows/release-surface-plugins.yml`, which builds and uploads the current web plugin archives for:

- `@allwright.dev/core` publish to npm after syncing `package.json` from the tag
- `allwright` CLI archives for the current OS matrix
- `allwright-surface-web` plugin archives for the current OS matrix
- crates.io publish for the Rust `web` profile after syncing every crate version from the tag

- Linux `x86_64-unknown-linux-gnu`
- Windows `x86_64-pc-windows-msvc`
- macOS `aarch64-apple-darwin`

Configure the `CARGO_REGISTRY_TOKEN` repository secret before pushing a release tag if you want the crates.io publish job to succeed.
The release workflow also sets `CARGO_PUBLISH_ALLOW_DIRTY=1` because it syncs crate versions from the tag inside CI before calling `cargo publish`.
Configure npm Trusted Publishing for `@allwright.dev/core` on npmjs.com before pushing a release tag:

- provider: `GitHub Actions`
- organization or user: `allwright-dev`
- repository: `allwright`
- workflow filename: `release-surface-plugins.yml`
- allowed action: `npm publish`

The npm publish job uses GitHub-hosted runners and OIDC instead of an `NPM_TOKEN`, which avoids bypass-2FA tokens entirely.

## Installer Scripts

- `scripts/install.sh`: installs the latest or requested `allwright` CLI release on Linux and macOS
- `scripts/install.ps1`: installs the latest or requested `allwright` CLI release on Windows PowerShell
- `scripts/sync-version.sh`: syncs the Rust workspace and internal crate versions from a release version string such as `X.Y.Z`
- `scripts/sync-npm-version.sh`: syncs the root npm package version from a release version string such as `X.Y.Z`
- users do not need to clone the repo; both scripts can be run directly from GitHub with `curl`, `wget`, or PowerShell `irm`

Both scripts support:

- `ALLWRIGHT_VERSION` to pin a specific release tag such as `vX.Y.Z`
- `ALLWRIGHT_INSTALL_DIR` to override the destination directory
- `ALLWRIGHT_REPOSITORY` to target a fork or alternate GitHub repository

For repo-specific contribution guidance, see [CONTRIBUTING.md](CONTRIBUTING.md).

For AI handoff and deeper repo conventions, see [Codex.md](Codex.md).

## Running Other Stacks

Go playground:

```bash
cd go
go run ./examples/playground --server-addr 127.0.0.1:50051
```

TypeScript build:

```bash
bun install
npm run build
```

TypeScript playground:

```bash
bun run typescript/examples/playground.ts --server-addr 127.0.0.1:50051
```

## Testing

Rust:

```bash
cargo test
```

Go:

```bash
cd go
go test ./...
```

## Contributing

Contributions are welcome. See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

This project is licensed under the MIT License. See [LICENSE](LICENSE).
