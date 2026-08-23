# allwright

Allwright is a Rust workspace for an automation engine that is now driven by the Tokio async runtime end to end.

The engine now starts as a gRPC server. The first gRPC layout is intentionally minimal: one proto package, one server service, and one basic starter RPC.

This project is being shaped as a driverless browser automation engine. The intended browser control model is Chrome + CDP-backed sessions, with a pinned `chromium-bidi` mapper artifact injected over CDP so the engine can expose higher-level automation without depending on ChromeDriver.

The repo is now organized by top-level technology folders:

- `rust/`: Rust workspace crates
- `go/`: Go module, generated stubs, and Go playground
- `java/`: Java client project that generates gRPC stubs from the shared proto
- `python/`: Python client package that loads the shared proto dynamically at runtime
- `allwright.dev/`: Next.js marketing site for the `allwright.dev` domain and Vercel deployment
- `typescript/`: TypeScript/JavaScript package for the JS stack
- `proto/`: shared protobuf and gRPC contracts consumed by every stack

Within that layout, the project hierarchy is intentionally split by ownership:

- `rust/allwright` owns the CLI surface
- `rust/engine-lib` owns all engine code, including the gRPC server surface
- `rust/engine-lib` further uses `rust/mobile-lib`, `rust/desktop-lib`, and `rust/web-lib`
- `rust/mobile-lib`, `rust/desktop-lib`, and `rust/web-lib` are platform support libraries beneath `rust/engine-lib`

Supporting platform crates sit under the platform library layer:

- `rust/mobile-lib` may depend on `rust/ios-lib` and `rust/android-lib`
- `rust/desktop-lib` may depend on `rust/windows-lib`, `rust/macos-lib`, and `rust/linux-lib`

## Architecture

- `rust/allwright`: CLI entrypoint built with `clap`
- `rust/allwright-client`: reusable Rust client with a high-level browser/tab API over the engine server
- `rust/playground`: end-to-end client for exercising the engine gRPC server
- `go/`: Go module with generated engine stubs, a Go client package, and a Go playground
- `java/`: Gradle-based Java client project with generated engine stubs and a high-level browser/page API
- `python/`: Python client package with dynamic proto loading and a high-level browser/page API
- `allwright.dev/`: standalone Next.js site for public marketing pages and documentation entrypoints
- `typescript/`: TypeScript/JavaScript stack folder containing the JS client package and playground
- `proto/`: shared protobuf contract root
- `rust/engine-lib`: owner of all engine code and the gRPC server crate
- `rust/mobile-lib`: mobile support library consumed by `rust/engine-lib`
- `rust/ios-lib`: iOS support library under mobile
- `rust/android-lib`: Android support library under mobile
- `rust/web-lib`: web support library consumed by `rust/engine-lib`
- `rust/desktop-lib`: desktop support library consumed by `rust/engine-lib`
- `rust/windows-lib`: Windows support library under desktop
- `rust/macos-lib`: macOS support library under desktop
- `rust/linux-lib`: Linux support library under desktop

The workspace shares a single Tokio dependency through the root `Cargo.toml` and uses the multi-thread runtime.

## gRPC Layout

- Proto root: `proto/`
- Package: `allwright.engine.v1`
- Service: `EngineService`
- Starter RPC: `Ping`
- Browser RPC: `BrowserSession`
- Tab RPC: `TabSession`
- Generated Rust code: built at compile time by `rust/engine-lib/build.rs`
- Generated gRPC client code: also built at compile time and used by `rust/allwright-client` and `rust/playground`
- Generated Go proto/gRPC code: checked in under `go/gen/allwright/engine/v1`
- The Java stack generates protobuf and gRPC stubs from `proto/engine/v1/engine.proto` during the Gradle build in `java/`
- The Python stack loads `proto/engine/v1/engine.proto` dynamically at runtime through `grpcio-tools`
- The TypeScript stack currently loads the shared proto dynamically from `proto/engine/v1/engine.proto` at runtime rather than checking in generated stubs

This is only the base gRPC layout. It is intentionally small so the API can grow from a clear starting point instead of guessing future engine contracts too early.

`BrowserSession` is a bidirectional streaming RPC. The intended first command is `launch_chrome`, which asks the engine to open a plain Chrome window and keep the stream alive for further session commands.

`OpenTabCommand` lives on `BrowserSession`. It opens a new tab over CDP and returns a `tab_session_id` in `TabOpenedEvent`. Clients then attach to `TabSession` with both `browser_session_id` and `tab_session_id` to continue tab-scoped communication.

`OpenTabCommand` is only valid after that browser session has successfully launched Chrome.

`LaunchChromeCommand` now also surfaces the already-open startup tab through `ChromeLaunchedEvent.initial_tab_session_id`, so clients can immediately attach a `TabSession` to Chrome's first tab and send navigation commands there too.

## Driverless Direction

- The engine is intended to be driverless.
- We do not want a ChromeDriver dependency in the control path.
- Browser sessions will likely need to be opened with CDP enabled so the engine can attach and track them.
- Browser launch now enables CDP and returns the discovered browser WebSocket endpoint from Chrome's `DevToolsActivePort`.
- Each browser session should be tracked by an engine session id.
- After the first tab navigation in a browser session, the engine injects a pinned `chromium-bidi` mapper artifact into a hidden mapper target in that same Chrome session and reuses that mapper for later tabs.
- Tab navigation should remain a separate command from browser or tab creation.

Reference used for this direction:
- GoogleChromeLabs `chromium-bidi`: https://github.com/GoogleChromeLabs/chromium-bidi

From that repository, `chromium-bidi` is described as an implementation layer translating between BiDi and CDP and running inside a Chrome tab. That is the basis for treating tab injection plus CDP session tracking as the likely engine path. Source: https://github.com/GoogleChromeLabs/chromium-bidi

## Generated Code

The Rust code generated from `proto/engine/v1/engine.proto` is not checked into the repository.

It is generated by Cargo through `rust/engine-lib/build.rs` and written into Cargo's build output directory, for example:

```text
target/debug/build/engine-lib-*/out/allwright.engine.v1.rs
```

`rust/engine-lib/src/lib.rs` loads that generated file through:

```rust
tonic::include_proto!("allwright.engine.v1");
```

The Go code generated from the same proto is checked in under:

```text
go/gen/allwright/engine/v1/
```

That generated package is consumed by:

- `go/client`
- `go/cmd/playground`

The Rust public client surface intentionally does not expose raw gRPC connection management either.

- `allwright-client` creates the engine transport lazily as a singleton
- the default Rust client server address is `http://127.0.0.1:50051`
- `ALLWRIGHT_SERVER_ADDR` can override that address
- `allwright_client::set_server_addr(...)` can override it inside the current process without exposing raw dial/setup APIs
- the public Rust surface is centered on higher-level `Browser` and `Tab` objects rather than raw proto streams

The public Go-facing API intentionally does not expose raw gRPC connection management.

- `go/client` uses package name `allwright`
- the engine connection is created lazily as a singleton
- the default server address is `127.0.0.1:50051`
- `ALLWRIGHT_SERVER_ADDR` can override that address without exposing a public `Dial` API
- the public Go surface is centered on higher-level browser and tab objects rather than raw proto streams

The public TypeScript/JavaScript-facing API intentionally follows that same model.

- `typescript/` is a single shared stack folder for both TypeScript and JavaScript consumers
- `typescript/src/index.ts` creates the engine transport lazily as a singleton
- the default TypeScript client server address is `127.0.0.1:50051`
- `ALLWRIGHT_SERVER_ADDR` can override that address
- `setServerAddr(...)` can override it inside the current process
- the public TypeScript surface is centered on Playwright-like `chromium`, `Browser`, and `Page` objects rather than raw grpc-js streams
- Bun is the preferred development/runtime tool for working in this repo's TypeScript stack

For the standalone `allwright.dev/` Next.js site, Bun is also the preferred local development workflow even though the app is intended for Vercel deployment.
The site currently targets the stable Next.js 16 line and uses Tailwind CSS v4 via the official PostCSS plugin.

The public Python-facing API follows the same high-level pattern.

- `python/` is a dedicated Python package folder
- `python/allwright/client.py` creates the engine transport lazily as a singleton
- the default Python client server address is `127.0.0.1:50051`
- `ALLWRIGHT_SERVER_ADDR` can override that address
- `set_server_addr(...)` can override it inside the current process
- the public Python surface is centered on `chromium`, `Browser`, and `Page` objects rather than raw grpc setup

The public Java-facing API follows that same pattern too.

- `java/` is a dedicated Gradle project for Java consumers
- `java/src/main/java/dev/allwright/client/Allwright.java` creates the engine transport lazily as a singleton
- the default Java client server address is `127.0.0.1:50051`
- `ALLWRIGHT_SERVER_ADDR` can override that address
- `setServerAddr(...)` can override it inside the current process
- the public Java surface is centered on `chromium`, `Browser`, and `Page` objects rather than raw gRPC setup

## Regenerating Proto Code

After changing any `.proto` file under `proto/`, regenerate the generated Rust by running:

```bash
cargo build -p engine-lib
```

To regenerate the Go proto and gRPC client code, first install the standard Go plugins locally:

```bash
GOBIN=$PWD/go/.bin go install google.golang.org/protobuf/cmd/protoc-gen-go@v1.36.10
GOBIN=$PWD/go/.bin go install google.golang.org/grpc/cmd/protoc-gen-go-grpc@v1.5.1
```

Then generate the Go code:

```bash
PATH=$PWD/go/.bin:$PATH protoc \
  -I proto \
  --go_out=go --go_opt=module=allwright-go \
  --go-grpc_out=go --go-grpc_opt=module=allwright-go \
  proto/engine/v1/engine.proto
```

You can also regenerate it as part of normal development commands:

```bash
cargo test
```

or

```bash
cargo run -p allwright -- --listen-addr 127.0.0.1:50051
```

Notes:

- `protoc` is provided automatically by the vendored `protoc-bin-vendored` build dependency.
- Generated files live under `target/` and should be treated as build artifacts, not hand-edited source files.
- The Go generated files under `go/gen/` are checked-in source and should be regenerated, not hand-edited.

## First Browser Command

The first browser-oriented engine command is defined in proto as part of `BrowserSession`.

- RPC: `BrowserSession`
- Stream direction: bidirectional
- Launch command: `LaunchChromeCommand`
- Purpose: open a plain Chrome browser window and keep a live engine stream available for follow-up commands
- Browser launch event returns CDP metadata including `cdp_websocket_url` and `user_data_dir`
- Browser launch event also returns `initial_tab_session_id` for the startup tab Chrome opens by default
- Tab open command: `OpenTabCommand`
- Tab stream attach path: `TabSession`
- Tab navigation command: `NavigateTabCommand`
- Tab click command: `ClickElementCommand`
- Chromium BiDi mapper asset: `rust/web-lib/third_party/chromium-bidi/17.0.2/mapperTab.js`

At the moment, the server attempts to open the Chrome browser binary through `web-lib` platform helpers with no ChromeDriver dependency:

- macOS: launches the Chrome binary directly with `--remote-debugging-port=0`
- Linux: launches a Chrome-compatible binary directly with `--remote-debugging-port=0`
- Windows: launches a Chrome-compatible binary directly with `--remote-debugging-port=0`

The current launch flow also creates an isolated user data dir and waits for Chrome's `DevToolsActivePort` file so the engine can discover the browser-level CDP WebSocket endpoint.

Important boundary:

- `LaunchChromeCommand` is only for launching the browser window itself.
- Future tab launch commands should also avoid carrying any URL.
- Navigation is a separate `NavigateTabCommand` over `TabSession`.
- Click is a separate `ClickElementCommand` over `TabSession`.
- `OpenTabCommand` opens a tab and returns `tab_session_id`.
- `LaunchChromeCommand` also returns the first already-open Chrome tab as `initial_tab_session_id`.
- `OpenTabCommand` requires a launched parent browser session.
- `TabSession` is the separate follow-up stream for tab-scoped communication.
- `TabSession` requires the parent `browser_session_id`.
- The startup Chrome tab is tracked exactly like later tabs and accepts the same `NavigateTabCommand`.
- `OpenTabCommand` is CDP-driven through `Target.createTarget`.
- The initial launch-created tab is discovered over CDP through `Target.getTargets`.
- `NavigateTabCommand` is CDP-driven through `Target.attachToTarget` plus `Page.navigate`.
- The current navigation path waits for `Page.loadEventFired`, injects the pinned `chromium-bidi` mapper into a hidden mapper target, and emits `chromium_bidi_injection` with `bidi_session_id`, mapper target/session ids, and package version.
- `ClickElementCommand` is BiDi-driven through `script.evaluate` on the tab browsing context and currently targets a CSS selector.
- The checked-in mapper artifact is sourced from the published `chromium-bidi@17.0.2` package but does not require `npm` at build time or runtime.

The stream contract is present now; richer browser control can be added as new proto commands and events without changing the top-level RPC shape.

## Tokio Runtime Model

- The CLI uses `#[tokio::main]` as the top-level runtime entrypoint.
- `rust/allwright` launches the gRPC server owned by `rust/engine-lib`.
- `rust/engine-lib` hosts the tonic server implementation and all engine behavior.
- Platform libraries remain support crates that `rust/engine-lib` can call into.
- `rust/mobile-lib` can fan out into `rust/ios-lib` and `rust/android-lib`.
- `rust/desktop-lib` can fan out into `rust/windows-lib`, `rust/macos-lib`, and `rust/linux-lib`.

This gives the project one consistent async execution model instead of mixing sync orchestration with future platform-specific async work.

## Ownership Tree

```text
proto/
└── engine/v1/engine.proto

rust/allwright (CLI)
└── rust/engine-lib (owns all engine code)
    ├── uses rust/mobile-lib
    │   ├── rust/ios-lib
    │   └── rust/android-lib
    ├── uses rust/desktop-lib
    │   ├── rust/windows-lib
    │   ├── rust/macos-lib
    │   └── rust/linux-lib
    └── uses rust/web-lib
```

## Run

```bash
cargo run -p allwright -- --listen-addr 127.0.0.1:50051
```

## Playground

Use `rust/playground` as the local end-to-end client for the engine server.

`rust/playground` now uses the public `allwright-client` crate from `rust/allwright-client` rather than talking to tonic streams directly.

Run the Rust playground test flow:

```bash
cargo run -p playground -- --server-addr http://127.0.0.1:50051
```

Open a custom number of tabs during the Rust playground flow:

```bash
cargo run -p playground -- --server-addr http://127.0.0.1:50051 --tabs 3
```

The current browser-session playground flow:

- calls `allwright_client::launch_chrome(...)`
- uses the returned `Browser` object and its `initial_tab()`
- sends navigation and optional click through high-level `Tab` methods
- opens additional tabs through `Browser::new_tab()`
- keeps Chrome open until keyboard confirmation
- closes tabs and the browser through high-level `Tab::close()` and `Browser::close()`
- optionally sends `ClickElementCommand` on each tab stream
- prints the post-navigation `chromium_bidi_injection` event with real mapper session metadata after `Page.loadEventFired`
- waits for keyboard input before closing the browser session

The current `CloseBrowserSessionCommand` path also terminates the launched Chrome process for that browser session.

## Go Client

The repo also contains a Go module for exercising the same engine API from Go.

- Module root: `go/`
- High-level public package: `go/client` with package name `allwright`
- Generated proto/gRPC package: `go/gen/allwright/engine/v1`
- Go playground command: `go/cmd/playground`

The intended Go usage is closer to Playwright-style objects than to raw gRPC plumbing:

```go
browser, err := allwright.LaunchChrome(ctx, allwright.LaunchOptions{})
tab := browser.InitialTab()
_, err = tab.Navigate(ctx, "https://example.com")
_, err = tab.Click(ctx, "a")
err = browser.Close(ctx)
```

The transport is hidden behind a singleton runtime. Consumers should not manually dial gRPC from the public API surface.

## TypeScript Client

The repo also contains a single shared TypeScript/JavaScript package under `typescript/`.

- Package root: `typescript/`
- High-level public client entrypoint: `typescript/src/index.ts`
- Playground entrypoint: `typescript/src/playground.ts`
- Shared contract source: `proto/engine/v1/engine.proto`
- Main public browser entrypoint: `chromium.launch(...)`

The intended TypeScript usage is also Playwright-like:

```ts
import { chromium } from "./src/index.js";

const browser = await chromium.launch({});
const page = browser.page();
await page.goto("https://example.com");
await page.click("a");
await browser.close();
```

The current TypeScript client also keeps a few compatibility helpers for the earlier shape, but new work should prefer the `chromium` / `Browser` / `Page` surface.

The transport is hidden behind a singleton runtime. Consumers should not manually construct grpc-js clients from the public API surface.

For local development in this repo, use Bun to install dependencies and run the package:

```bash
cd typescript
bun install
bun run build
```

Run the TypeScript playground test flow:

```bash
cd typescript
bun run src/playground.ts --server-addr 127.0.0.1:50051
```

Run the Go playground test flow:

```bash
cd go
go run ./cmd/playground --server-addr 127.0.0.1:50051
```

Run the Go playground flow with an optional BiDi click:

```bash
cd go
go run ./cmd/playground \
  --server-addr 127.0.0.1:50051 \
  --navigate-url https://example.com \
  --click-selector 'a'
```

## Test

```bash
cargo test
```

```bash
cd go
go test ./...
```

## Workspace Layout

```text
.
├── Cargo.toml
├── Cargo.lock
├── Codex.md
├── README.md
├── go/
├── proto/
├── typescript/
└── rust/
    ├── allwright/
    ├── allwright-client/
    ├── android-lib/
    ├── desktop-lib/
    ├── engine-lib/
    ├── ios-lib/
    ├── linux-lib/
    ├── macos-lib/
    ├── mobile-lib/
    ├── playground/
    ├── web-lib/
    └── windows-lib/
```

## Next Steps

- Replace placeholder platform support implementations with real integrations.
- Expand the gRPC API beyond the starter `Ping` service once engine contracts are designed.
- Introduce browser and tab session identifiers explicitly in engine events and commands.
- Expand the current BiDi command surface beyond selector-based click into richer actions, script evaluation, and event subscriptions.
- Expand the Go playground beyond the current initial-tab test flow if we want fuller Go-side end-to-end coverage.
- Introduce shared async traits and error types owned by `rust/engine-lib`.
- Grow `rust/playground` into the default end-to-end test harness for engine workflows.
- Keep `Codex.md` current as the handoff document for future AI sessions.
- Add integration tests around engine startup and platform coordination.
