# Firefox Refactor Plan

This document turns the current "support Firefox" discussion into a concrete refactor plan for this repository.

## Goal

Keep one high-level automation API across clients while making the web engine capable of running more than one browser backend.

Target user-facing shape:

- `chromium.launch()`
- `firefox.launch()`
- shared `Browser`, `Page`, `Locator`, and assertion APIs above the engine

## Current State

The current implementation is deeply Chromium-shaped in three layers:

1. Protobuf command and event names are Chrome-specific.
2. Engine state and command handling assume Chrome + CDP + injected Chromium BiDi.
3. The web plugin SDK exposes only Chrome/CDP-oriented commands and result types.

The main seams today are:

- [proto/core/v1/browser.proto](/Users/atmaramn/data/personal/gh/allwright/proto/core/v1/browser.proto:1)
- [proto/surfaces/web/v1/web.proto](/Users/atmaramn/data/personal/gh/allwright/proto/surfaces/web/v1/web.proto:1)
- [rust/allwright/src/engine.rs](/Users/atmaramn/data/personal/gh/allwright/rust/allwright/src/engine.rs:1)
- [rust/allwright-plugin-sdk/src/lib.rs](/Users/atmaramn/data/personal/gh/allwright/rust/allwright-plugin-sdk/src/lib.rs:1)
- [rust/allwright-surface-web/src/lib.rs](/Users/atmaramn/data/personal/gh/allwright/rust/allwright-surface-web/src/lib.rs:1)

## What Is Too Chrome-Specific Right Now

### 1. Protobuf surface

Current examples:

- `LaunchChromeCommand`
- `ChromeLaunchedEvent`
- `cdp_websocket_url`
- `ChromiumBidiInjectionEvent`

Problem:

- These names leak backend choice into every client library.
- Firefox support would either require parallel Firefox-only messages or continued misuse of Chrome names.

### 2. Engine state

Current engine state in [rust/allwright/src/engine.rs](/Users/atmaramn/data/personal/gh/allwright/rust/allwright/src/engine.rs:1) stores:

- `cdp_websocket_url`
- `process_id`
- `bidi_mapper`
- tab `target_id`
- tab `browsing_context_id`

Problem:

- These are transport/backend details, not browser-agnostic session facts.
- They make the engine itself own Chromium implementation concepts directly.

### 3. Plugin SDK

Current SDK types in [rust/allwright-plugin-sdk/src/lib.rs](/Users/atmaramn/data/personal/gh/allwright/rust/allwright-plugin-sdk/src/lib.rs:1) include:

- `ChromeLaunchInfo`
- `ChromeTabInfo`
- `ChromiumBidiMapperInfo`
- `PluginCommand::OpenChromeWindow`
- `PluginCommand::NavigateChromeTab`
- `PluginCommand::ClickElementViaCdp`

Problem:

- The engine cannot ask for generic "launch browser", "open page", or "click selector" behavior.
- Every plugin contract is named after the current Chromium transport.

### 4. Web plugin implementation

Current implementation in [rust/allwright-surface-web/src/lib.rs](/Users/atmaramn/data/personal/gh/allwright/rust/allwright-surface-web/src/lib.rs:1) is a single Chromium/CDP codepath:

- Chrome discovery and launch
- CDP websocket lifecycle
- hidden mapper target injection
- DOM interaction via CDP

Problem:

- There is no backend seam inside the plugin.
- Firefox would force either a second giant file or awkward branching inside CDP-centric functions.

## Recommended Direction

Do not add "Firefox support" as special-case code beside the current Chrome path.

Instead:

1. Make the engine/browser protocol backend-neutral.
2. Introduce a web backend abstraction inside the web plugin.
3. Keep Chromium as the first backend implementation.
4. Add Firefox as the second backend once the abstraction exists.

## Proposed End State

### Engine-level concepts

The engine should know about:

- browser kind
- browser session
- page/tab session
- navigation
- selector actions
- wait/query primitives

The engine should not directly know about:

- CDP websocket URLs
- Chrome target IDs
- mapper target IDs
- Chromium-specific injected scripts

### Backend-level concepts

A web backend implementation can know about:

- Chrome CDP
- Chromium BiDi mapper
- Firefox launch details
- Firefox remote protocol / BiDi transport

That information should stay behind a backend interface.

## Refactor Phases

### Phase 1. Add neutral names alongside current names

Purpose:

- create a safe migration path without breaking all clients at once

Changes:

1. Add `BrowserKind` enum to protobuf and SDK.
2. Add neutral launch command and launch event messages:
   - `LaunchBrowserCommand`
   - `BrowserLaunchedEvent`
3. Keep `LaunchChromeCommand` and `ChromeLaunchedEvent` temporarily as compatibility aliases.

Suggested fields:

- `browser_kind`
- `browser_binary`
- `note`
- `initial_tab_session_id`

Do not expose `cdp_websocket_url` in the neutral event unless we intentionally want transport data in the public API.

### Phase 2. Introduce backend-neutral engine state

Purpose:

- stop storing Chromium transport details directly in engine structs

Refactor [rust/allwright/src/engine.rs](/Users/atmaramn/data/personal/gh/allwright/rust/allwright/src/engine.rs:1):

Replace:

- `cdp_websocket_url`
- `bidi_mapper`
- `target_id`

With something shaped like:

```rust
enum BrowserBackendKind {
    Chromium,
    Firefox,
}

struct BrowserSessionState {
    backend: BrowserBackendKind,
    process_id: Option<u32>,
    backend_session: BackendBrowserSession,
}

struct TabSessionState {
    browser_session_id: String,
    backend_tab: BackendTabSession,
    current_url: Option<String>,
}
```

Where `BackendBrowserSession` and `BackendTabSession` are engine-owned opaque structs or enums.

### Phase 3. Generalize the plugin SDK

Purpose:

- let the engine ask the plugin for browser-agnostic behavior

Refactor [rust/allwright-plugin-sdk/src/lib.rs](/Users/atmaramn/data/personal/gh/allwright/rust/allwright-plugin-sdk/src/lib.rs:1) to add neutral command/result types.

Add neutral equivalents for:

- launch browser
- discover initial page
- open page
- close page
- navigate page
- click
- count
- highlight
- focus
- fill
- hover
- press
- get text
- wait for selector

Suggested naming pattern:

- `BrowserLaunchInfo`
- `PageInfo`
- `NavigatePageInfo`
- `ClickElementInfo`

Suggested command naming pattern:

- `LaunchBrowser`
- `DiscoverInitialPage`
- `OpenPage`
- `ClosePage`
- `NavigatePage`
- `ClickElement`

Keep the old Chrome/CDP command variants temporarily and map them internally to the new neutral path during migration.

### Phase 4. Create a backend abstraction inside the web plugin

Purpose:

- isolate Chromium code and make room for Firefox

Refactor [rust/allwright-surface-web/src/lib.rs](/Users/atmaramn/data/personal/gh/allwright/rust/allwright-surface-web/src/lib.rs:1) into modules like:

- `backend/mod.rs`
- `backend/chromium.rs`
- `backend/firefox.rs`
- `backend/types.rs`

Suggested trait:

```rust
trait WebBackend {
    type BrowserSession;
    type PageSession;

    fn kind(&self) -> BrowserBackendKind;

    async fn launch(&self, options: LaunchBrowserOptions) -> Result<BrowserLaunchInfo, String>;
    async fn discover_initial_page(&self, session: &Self::BrowserSession) -> Result<PageInfo, String>;
    async fn open_page(&self, session: &Self::BrowserSession) -> Result<PageInfo, String>;
    async fn close_page(&self, session: &Self::BrowserSession, page: &Self::PageSession) -> Result<(), String>;
    async fn navigate_page(&self, page: &Self::PageSession, url: &str) -> Result<NavigatePageInfo, String>;
    async fn click(&self, page: &Self::PageSession, selector: &str) -> Result<ClickInfo, String>;
    async fn count(&self, page: &Self::PageSession, selector: &str) -> Result<ElementCountInfo, String>;
    async fn wait_for_selector(&self, page: &Self::PageSession, selector: &str, visible: bool) -> Result<WaitForSelectorInfo, String>;
}
```

The first backend implementation should just wrap today's Chromium functions.

### Phase 5. Keep Chromium-specific extras behind capability flags

Purpose:

- avoid blocking Firefox support on Chromium-only features

Examples of Chromium-specific features today:

- injected Chromium BiDi mapper metadata
- target IDs
- mapper session IDs
- package version reporting from Chromium BiDi

These should become:

- backend capability flags
- optional backend diagnostics

They should not remain part of the required happy path for generic navigation and selector actions.

### Phase 6. Add Firefox MVP

Firefox MVP should support:

- launch browser
- discover initial page
- open page
- close page
- navigate
- click
- fill
- hover
- text content / inner text
- count
- wait for selector

Not required for MVP:

- full parity with Chromium-only diagnostics
- every advanced input edge case
- browser-specific metadata parity

## API Shape Recommendation

### Client-facing API

TypeScript, Python, Go, and Rust clients should move toward:

- `chromium.launch()`
- `firefox.launch()`

And optionally later:

- `browserType.launch({ kind: "firefox" })`

The existing `chromium` export can remain unchanged while Firefox is added beside it.

### Assertion layer

Do not put high-level assertions into the engine.

Instead:

- engine exposes reliable wait/query primitives
- client/test packages implement `expect(page.locator(...))` and similar retrying assertions

That matches the current direction of `@allwright.dev/vitest`.

## Concrete First PR Sequence

The safest order in this repo is:

1. Add this refactor plan document.
2. Add `BrowserKind` plus neutral protobuf messages while keeping current Chrome messages.
3. Add neutral SDK types/commands beside the current Chrome/CDP types.
4. Refactor the engine to call neutral SDK commands internally.
5. Move Chromium implementation into `backend/chromium.rs`.
6. Keep behavior identical and get Chromium green again.
7. Add `backend/firefox.rs` with a launch + navigate MVP.
8. Expose `firefox.launch()` in clients.

## Success Criteria

We are "ready for Firefox" when:

- the engine no longer stores raw CDP concepts as its main session model
- protobuf and SDK APIs are browser-neutral
- the web plugin has an explicit backend abstraction
- Chromium remains the default stable backend
- Firefox can be added without renaming everything again

## Recommendation For The Very Next Change

The next code change should be:

- add `BrowserKind`
- add neutral launch messages to protobuf
- add neutral SDK command/result types

That is the smallest move that creates real space for Firefox without forcing a big-bang rewrite.
