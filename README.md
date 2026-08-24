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
- the web surface implementation exists in its own crate, but runtime plugin loading is still being wired into the lightweight core
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

## Quick Start

Start the engine:

```bash
cargo run -p allwright -- --listen-addr 127.0.0.1:50051
```

List or register plugins:

```bash
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

At the moment, the lightweight core and CLI/plugin packaging are ahead of runtime plugin loading. Surface crates and split proto ownership are in place, while dynamic runtime activation of installed plugins is still follow-up work.

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
- `rust/allwright-cli`: installable `allwright` CLI package that depends on `allwright-core` and manages plugin registration
- `rust/allwright-plugin-sdk`: shared plugin traits and surface metadata
- `rust/allwright-surface-web`: publishable `web` surface crate
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
- Surface plugins are currently registered through the CLI plugin manifest and are not yet runtime-loaded by the core automatically.
- The Rust workspace is versioned as `0.0.6` across the publishable engine and surface crates.
- The browser control path is intended to stay driverless.
- The repo uses shared proto contracts across all supported client stacks.
- Bun is the preferred local workflow for the TypeScript stack and the `allwright-dev/` site.

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
