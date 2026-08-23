# allwright

allwright is one automation engine for everything you test.

The long-term direction is a single system that can cover web, mobile, desktop, and API automation without forcing teams to stitch together a different tool for every surface. The project is designed so automation can feel consistent across the whole product, not fragmented by platform.

Right now, allwright is being built in public and the browser automation engine is the first active layer. The current implementation is focused on a driverless Chrome control path backed by CDP and Chromium BiDi, with high-level client libraries for Rust, Go, Java, Python, and TypeScript/JavaScript.

## Why allwright

- One engine instead of a pile of disconnected tools
- One automation model that can eventually span web, mobile, desktop, and API work
- High-level browser and page APIs instead of raw transport plumbing
- Driverless browser control built around CDP and Chromium BiDi
- Shared contracts across the engine and all client stacks

## Current Status

allwright is under active development and not positioned as a finished multi-surface platform yet.

The current stage is:

- the core product direction is broader than browser automation alone
- the first shipped implementation work is centered on the browser engine
- the engine can currently launch Chrome, attach to the startup tab, open additional tabs, navigate, and perform basic selector-based click actions
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

## Quick Start

Start the engine:

```bash
cargo run -p allwright -- --listen-addr 127.0.0.1:50051
```

Try the Rust playground against the running engine:

```bash
cargo run -p playground -- --server-addr http://127.0.0.1:50051
```

Open more tabs during the playground flow:

```bash
cargo run -p playground -- --server-addr http://127.0.0.1:50051 --tabs 3
```

## What You Can Try Today

Today’s working path is browser-focused, with a Rust-powered engine and high-level client libraries layered on top.

## Client Experience

allwright is designed around high-level browser objects rather than asking application code to manage raw gRPC connections.

Rust example:

```rust
let browser = allwright_client::launch_chrome(Default::default()).await?;
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

- `rust/`: Rust engine, Rust client, and Rust playground
- `go/`: Go client and Go playground
- `java/`: Java client project
- `python/`: Python client package
- `typescript/`: TypeScript/JavaScript client package and playground
- `proto/`: shared protobuf and gRPC contracts
- `allwright-dev/`: public website project for `allwright.dev`

## Development Notes

- The engine currently runs as a gRPC server.
- The current implementation focus is browser automation, but the product direction is wider.
- The browser control path is intended to stay driverless.
- The repo uses shared proto contracts across all supported client stacks.
- Bun is the preferred local workflow for the TypeScript stack and the `allwright-dev/` site.

For repo-specific contribution guidance, see [CONTRIBUTING.md](CONTRIBUTING.md).

For AI handoff and deeper repo conventions, see [Codex.md](Codex.md).

## Running Other Stacks

Go playground:

```bash
cd go
go run ./cmd/playground --server-addr 127.0.0.1:50051
```

TypeScript build:

```bash
cd typescript
bun install
bun run build
```

TypeScript playground:

```bash
cd typescript
bun run src/playground.ts --server-addr 127.0.0.1:50051
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
