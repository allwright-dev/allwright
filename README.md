# allwright

allwright is a driverless browser automation engine aimed at giving you one automation surface across multiple language stacks.

Today, the project centers on a Rust-powered engine with high-level client libraries for Rust, Go, Java, Python, and TypeScript/JavaScript. The goal is a Playwright-like developer experience backed by a browser control path that does not depend on ChromeDriver.

## Why allwright

- One engine for multiple language ecosystems
- High-level browser and page APIs instead of raw transport plumbing
- Driverless Chrome control built around CDP and Chromium BiDi
- Shared protobuf contracts across clients and engine

## Current Status

allwright is under active development. The current flow can launch Chrome, attach to the startup tab, open additional tabs, navigate, and perform basic selector-based click actions through the engine.

The API and implementation are still evolving, so expect active iteration as the engine grows.

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
