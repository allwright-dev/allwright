# allwright

[![crates.io](https://img.shields.io/crates/v/allwright-core?label=crates.io)](https://crates.io/crates/allwright-core)
[![Go Reference](https://pkg.go.dev/badge/allwright.dev.svg)](https://pkg.go.dev/allwright.dev)
[![Maven Central](https://img.shields.io/maven-central/v/dev.allwright/allwright?label=maven%20central)](https://central.sonatype.com/artifact/dev.allwright/allwright)
[![PyPI](https://img.shields.io/pypi/v/allwright?label=pypi)](https://pypi.org/project/allwright/)
[![npm](https://img.shields.io/npm/v/%40allwright.dev%2Fcore?label=npm)](https://www.npmjs.com/package/@allwright.dev/core)

allwright is one automation engine for everything you test.

The long-term direction is a single system that can cover web, mobile, desktop, and API automation without forcing teams to stitch together a different tool for every surface. The project is designed so automation can feel consistent across the whole product, not fragmented by platform.

That direction also applies to extensibility: allwright should stay one engine at its core, while surface modules like `web`, `mobile-android`, `mobile-ios`, `desktop-mac`, `desktop-windows`, and `desktop-linux` can be installed separately as plugins instead of fragmenting the runtime into multiple engines.

Right now, allwright is being built in public with installable web, Android, iOS, and macOS desktop runtimes. Chromium runs through Chromium BiDi, Firefox through its native WebDriver BiDi Remote Agent, Android through native ADB integration, and Apple platforms through bundled XCUITest runners. High-level clients are available for Rust, Go, Java, Python, and TypeScript/JavaScript.

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
- web, Android, iOS, macOS desktop, and Windows desktop automation use the same engine and lazy plugin boundary
- the Rust workspace now separates a lightweight `allwright-core` from the installable `allwright` CLI and surface crates
- `web`, `mobile-android`, `mobile-ios`, `desktop-mac`, and `desktop-windows` ship as separately installable runtime plugins
- Linux desktop and API automation remain planned
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
- `web` and `mobile-android`: installable runtime surface plugins loaded by the core at runtime
- `mobile-ios`: an experimental installable macOS plugin with bundled, auto-started headless XCUITest runners for iOS Simulators and physical devices; device runners are re-signed locally from the user's Apple development credentials
- `desktop-mac`: installable macOS plugin with a bundled, auto-started XCUITest runner
- `desktop-windows`: installable Windows x64 plugin with a bundled, self-contained FlaUI UIA3 agent
- `desktop-linux`: planned surface plugin with a publishable crate boundary, but not yet an installable runtime artifact

What `plugin install` means today:

- for a supported plugin, it downloads the matching platform archive from GitHub Releases into the local allwright plugin directory and records it in the manifest
- `allwright serve` always starts the engine server
- the core loads a plugin library only when a command first needs that surface
- when a required plugin is unavailable, its commands fail with a plugin-required error while the core server still runs
- the iOS and macOS archives include their native XCUITest runner artifacts and require macOS with Xcode

Linux desktop and API support are still being completed; the shipped surfaces share one server-only client transport.

Release automation today:

- pushing a tag such as `vX.Y.Z` triggers the GitHub Actions release workflow
- that workflow creates the Go submodule tag `go/vX.Y.Z`, verifies the Go client in `go/`, and warms the public Go proxy for `allwright.dev`
- that workflow publishes the Java client to Maven Central as `dev.allwright:allwright` using the checked-in Gradle wrapper, a Central Portal user token, and a follow-up transfer call through Sonatype's Central Portal OSSRH Staging API compatibility service
- that workflow publishes the Python client to PyPI as `allwright` using PyPI Trusted Publishing via GitHub Actions OIDC
- that workflow publishes the npm workspace packages `@allwright.dev/core`, `@allwright.dev/vitest`, and `create-allwright` using npm Trusted Publishing via GitHub Actions OIDC
- that workflow builds the `allwright` CLI and installable web, Android, iOS, and macOS desktop plugin archives and uploads them to the matching GitHub Release
- `allwright plugin install web` resolves the local OS and architecture, then downloads the matching release asset
- the Rust, Go, Java, Python, and TypeScript clients now auto-bootstrap the matching `allwright` CLI and `web` plugin for their own version when they target a local server address and nothing is running yet
- those clients also reuse an already-healthy local server when one exists, and only tear down the server process if that specific client started it
- the release workflow syncs the Rust workspace version from the Git tag before building, so the tag is the release source of truth

## Runnable Examples

Each first-party client includes runnable examples for all five implemented
surfaces:

| Client | Web | Android | iOS | macOS | Windows |
| --- | --- | --- | --- | --- | --- |
| Rust | [`web_basic`](./rust/allwright/examples/web_basic.rs) | [`android_basic`](./rust/allwright/examples/android_basic.rs) | [`ios_basic`](./rust/allwright/examples/ios_basic.rs) | [`macos_basic`](./rust/allwright/examples/macos_basic.rs) | [`windows_basic`](./rust/allwright/examples/windows_basic.rs) |
| Go | [`web-basic`](./go/examples/web-basic/main.go) | [`android-basic`](./go/examples/android-basic/main.go) | [`ios-basic`](./go/examples/ios-basic/main.go) | [`macos-basic`](./go/examples/macos-basic/main.go) | [`windows-basic`](./go/examples/windows-basic/main.go) |
| Java | [`WebBasicTest`](./java/src/test/java/dev/allwright/examples/WebBasicTest.java) | [`AndroidBasicTest`](./java/src/test/java/dev/allwright/examples/AndroidBasicTest.java) | [`IosBasicTest`](./java/src/test/java/dev/allwright/examples/IosBasicTest.java) | [`MacosBasicTest`](./java/src/test/java/dev/allwright/examples/MacosBasicTest.java) | [`WindowsBasicTest`](./java/src/test/java/dev/allwright/examples/WindowsBasicTest.java) |
| Python | [`web_basic`](./python/examples/web_basic.py) | [`android_basic`](./python/examples/android_basic.py) | [`ios_basic`](./python/examples/ios_basic.py) | [`macos_basic`](./python/examples/macos_basic.py) | [`windows_basic`](./python/examples/windows_basic.py) |
| TypeScript | [`web-basic`](./typescript/core/examples/web-basic.ts) | [`android-basic`](./typescript/core/examples/android-basic.ts) | [`ios-basic`](./typescript/core/examples/ios-basic.ts) | [`macos-basic`](./typescript/core/examples/macos-basic.ts) | [`windows-basic`](./typescript/core/examples/windows-basic.ts) |
| Vitest | [`basic`](./typescript/vitest/examples/basic.spec.ts) | [`android-flights`](./typescript/vitest/examples/android-flights.spec.ts) | [`ios-flights`](./typescript/vitest/examples/ios-flights.spec.ts) | [`macos-calculator`](./typescript/vitest/examples/macos-calculator.spec.ts) | [`windows-notepad`](./typescript/vitest/examples/windows-notepad.spec.ts) |

## Quick Start

Install the CLI:

```bash
curl -fsSL https://raw.githubusercontent.com/allwright-dev/allwright/main/scripts/install.sh | bash
```

The installer verifies the downloaded binary's version. If an older package-manager-installed
`allwright` appears earlier on `PATH`, it reports the exact shadowing executable instead of
silently making the new install look stale.

Update an existing installation in place from the latest GitHub release:

```bash
allwright update
```

Use `allwright update --version vX.Y.Z` to install a specific release. The updater downloads the
same platform archive as the installer, validates the new binary before replacing the currently
running executable, and honors `ALLWRIGHT_REPOSITORY` and `ALLWRIGHT_GITHUB_TOKEN` for alternate
or private release repositories. On Windows, replacement completes immediately after the update
command exits because Windows locks the running executable. Restart an already-running
`allwright serve` process after updating; new server processes use the updated embedded core.

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
allwright plugin install desktop-mac
allwright plugin install desktop-windows
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

Today’s working paths cover web, Android, iOS, and native macOS applications through one Rust-powered engine and five high-level client libraries.

The practical path today is:

- use the `allwright` CLI as the installable entrypoint
- install the surface plugin you need (`web`, `mobile-android`, `mobile-ios`, `desktop-mac` on macOS, or `desktop-windows` on Windows x64)
- use the Rust, Go, Java, Python, or TypeScript clients against the running engine server, or let the client auto-start a matching local server on first use

Each working surface is loaded into core only when used. Clients always talk to the engine server; they never invoke plugin libraries or native agents directly.

macOS desktop automation uses the same app/locator shape as native mobile:

Across the client libraries, that shared shape is exposed as `NativeApp` and
`NativeLocator` (`NativeAppLocator` in TypeScript). Existing platform-specific
type names remain available as compatibility aliases or wrappers.

```ts
import { desktop } from "@allwright.dev/core";

const mac = await desktop.mac.connect();
const app = await mac.launch({ appId: "com.apple.TextEdit" });
await app.getByRole("button", { name: "New Document" }).click();
await app.screenshot({ path: "textedit.png" });
```

Vitest can own the same startup path lazily through the `macosApp` fixture:

```yaml
desktop:
  mac:
    app:
      id: com.apple.TextEdit
```

```ts
import { test } from "@allwright.dev/vitest";

test("creates a document", async ({ macosApp }) => {
  await macosApp.getByRole("button", { name: "New Document" }).click();
});
```

The `desktop-mac` plugin copies its bundled unsigned XCUITest runner to a local
cache, ad-hoc signs that copy without selecting a certificate, and auto-starts
it. It supports
native selectors, semantic role/text/label/test-id locators, click, count,
focus, fill, key input, text reads, waits, screenshots, and accessibility
snapshots. It requires macOS 14 or newer with Xcode installed and UI automation
permission enabled for the process running Xcode.

Windows uses the matching lazy `windowsApp` fixture. The configured `id` can be
an executable path, a command such as `notepad.exe`, or a packaged-app AUMID:

```yaml
desktop:
  windows:
    app:
      id: notepad.exe
```

```ts
import { test } from "@allwright.dev/vitest";

test("edits a document", async ({ windowsApp }) => {
  await windowsApp.getByRole("textbox").fill("Hello from Allwright");
});
```

The `windows` and `windowsApp` fixtures remain lazy, use 30-second connect and
60-second launch defaults, and accept per-test overrides through
`allwright.windows.connectOptions` and `allwright.windows.launchOptions`.

Web pages support accessibility snapshots in JSON (default) or standard YAML:

```ts
const tree = JSON.parse(await page.accessibilitySnapshot());
const yaml = await page.accessibilitySnapshot({ format: "yaml" });
```

Both formats decode to the same versioned structure with explicit roles, names,
states, properties, children, and iframe documents. The method is available in
all five clients and the Vitest page fixture. Accessibility computation is implemented
in Allwright-owned JavaScript using specifications and Playwright as references. See the
[accessibility snapshot contract and limitations](rust/allwright-surface-web/README.md#accessibility-snapshots).

Actions that create a new tab can be coordinated with a typed hook. Registering
the hook captures the source page state before the action, so the tab is not missed
if it opens before the test starts waiting:

```ts
import { hooks } from "@allwright.dev/core";

const newPageHook = await page.registerHook(hooks.newPage);
await page.click("a[target=_blank]");
const newPage = await newPageHook.wait();
```

File uploads use the same lifecycle and return a typed file chooser:

```ts
const chooserHook = await page.registerHook(hooks.fileChooser);
await page.click("button.open-upload");
const chooser = await chooserHook.wait();
await chooser.setFiles(["fixtures/photo.png"]);
```

Downloads follow the same lifecycle. The hook resolves when the download starts;
`saveAs` waits for it to finish and copies it to the requested path:

```ts
const downloadHook = await page.registerHook(hooks.download);
await page.click("a.download-report");
const download = await downloadHook.wait();
await download.saveAs(`artifacts/${download.suggestedFilename}`);
```

Upload and download paths are always resolved by the client on the machine
running the test. Files are streamed through the engine, so the same API works
when the Allwright server runs on another machine.

Android app contexts support the same file hooks (but not `newPage`):

```ts
const chooserHook = await app.registerHook(hooks.fileChooser);
await app.click("button.attach");
await (await chooserHook.wait()).setFiles("fixtures/photo.png");

const downloadHook = await app.registerHook(hooks.download);
await app.click("button.download");
const download = await downloadHook.wait();
await download.saveAs(`artifacts/${download.suggestedFilename}`);
```

Android uploads currently support the system DocumentsUI picker with one file.
Android downloads observe files created in the device's public Downloads directory.

JavaScript alerts, confirms, and prompts use the same typed hook lifecycle:

```ts
const dialogHook = await page.registerHook(hooks.dialog);
await page.click("#ask-name");
const dialog = await dialogHook.wait({ timeoutMs: 5_000 });
console.log(dialog.type, dialog.message, dialog.defaultValue);
await dialog.accept("Ada", { timeoutMs: 5_000 }); // or accept() to keep the default
// await dialog.dismiss({ timeoutMs: 5_000 });
```

Register before triggering the dialog. Click, press, fill, focus, and hover
input actions yield when a dialog opens; handle it before further page actions. Hooks are one-shot and page-scoped; only
one dialog hook/pending dialog is allowed per page. Waiting and accept/dismiss
support command timeouts. Registration intentionally does not: cancelling setup
can orphan a browser subscription. Handling is single-attempt even when a
timeout is supplied, so it can never be replayed against a later dialog. Explicit
empty prompt text clears the default. Text is only valid when accepting a prompt. Unhandled
JavaScript dialogs remain open; always register and handle expected dialogs.
Navigation-triggered and before-unload dialogs are not covered by this input-action flow.

Other clients expose `hooks.dialog` (Python), `Hooks.DIALOG` (Java),
`Hooks.Dialog` (Go), and `DIALOG` (Rust). Rust accepts `Option<&str>`; other
clients accept optional prompt text. Android dialog hooks are not supported.

Hook registration and waiting are generic engine operations. New-page,
file-chooser, download, and dialog hook types/results remain owned by their surface.

Iframes resolve to normal pages, including nested and cross-origin frames:

```ts
const frame = await page.locator("#checkout-frame").frame({ timeoutMs: 10_000 });
await frame.locator("input[name=email]").fill("buyer@example.com");
const nested = await frame.locator("iframe").frame();
```

Resolution waits for exactly one matching iframe whose document is complete and
has gone 200 ms without DOM mutations (default timeout 10 seconds). Closing a
frame page releases its session without closing the parent tab. Go spells it
`locator.Frame(ctx, ...)`, Rust `locator.frame().await?`, and Java/Python
`locator.frame(options)`.

## Client Experience

allwright is designed around high-level browser objects rather than asking application code to manage raw gRPC connections.

Rust example:

```rust
let browser = allwright::launch_firefox(Default::default()).await?;
let tab = browser.initial_tab()?;
tab.navigate("https://example.com").await?;
tab.click("a").await?;
browser.close().await?;
```

Go example:

```go
browser, err := allwright.LaunchFirefox(ctx, allwright.LaunchOptions{})
tab := browser.InitialTab()
_, err = tab.Navigate(ctx, "https://example.com")
_, err = tab.Click(ctx, "a")
err = browser.Close(ctx)
```

TypeScript example:

```ts
import { firefox } from "./src/index.js";

const browser = await firefox.launch({});
const page = browser.page();
await page.goto("https://example.com");
await page.click("a");
await browser.close();
```

Web clicks support left, middle, and right mouse buttons plus one-to-three click sequences. Each
client also exposes `dblclick` (Go: `DblClick`) on pages and locators.

Java example:

```java
import dev.allwright.client.Allwright;
import dev.allwright.client.Browser;
import dev.allwright.client.Page;

try (Browser browser = Allwright.firefox().launch()) {
    Page page = browser.page();
    page.goTo("https://example.com");
    page.click("a");
}
```

Python example:

```python
from allwright import firefox

browser = firefox.launch()
page = browser.page()
page.goto("https://example.com")
page.click("a")
browser.close()
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
- `rust/allwright-surface-desktop-mac`: runtime-loaded macOS plugin backed by a bundled XCUITest runner
- `rust/allwright-surface-desktop-windows`: runtime-loaded Windows plugin backed by a bundled FlaUI UIA3 sidecar
- `rust/allwright-surface-desktop-linux`: publishable `desktop-linux` surface crate
- `go/`: published Go client `allwright.dev` and Go playground
- `java/`: published Java client `dev.allwright:allwright` on Maven Central
- `python/`: published Python client package `allwright` on PyPI
- `typescript/core`: published TypeScript client package `@allwright.dev/core`
- `typescript/vitest`: published Vitest fixture package `@allwright.dev/vitest`
- `proto/`: shared protobuf and gRPC contracts
  The root service entrypoint remains `proto/engine/v1/engine.proto`, while shared core messages now live under `proto/core/v1/` and the web surface messages now live under `proto/surfaces/web/v1/`.
- `allwright-dev/`: public website project for `allwright.dev`

## Development Notes

- The engine currently runs as a gRPC server.
- Web, Android, iOS, macOS desktop, and Windows desktop runtimes are implemented; Linux and API support remain future work.
- Installing the `allwright` package is intended to deliver the CLI plus the lightweight engine core together.
- The project should keep a single engine core even as surface modules become separately installable plugins.
- The `web`, `mobile-android`, `mobile-ios`, `desktop-mac`, and `desktop-windows` plugins are installed through GitHub Release downloads and loaded by core at runtime.
- Desktop Linux remains disabled as an install target until its runtime binary exists.
- The Rust workspace version is synced from the release tag during GitHub release builds.
- The browser control path is intended to stay driverless.
- The repo uses shared proto contracts across all supported client stacks.
- Bun is the preferred local workflow for the TypeScript stack and the `allwright-dev/` site.

## Releasing Plugins

Maintainers can run this locally from their own machine to prepare a release commit and push the matching tag:

```bash
./scripts/prepare-release.sh X.Y.Z
```

The script is a local maintainer helper, not a CI/CD step. It requires a clean local `main` checkout, syncs every checked-in package version to `X.Y.Z`, pushes the release-prep commit to `origin/main`, then creates and pushes the root tag as `vX.Y.Z`.

Create and push a version tag:

```bash
git tag vX.Y.Z
git push origin vX.Y.Z
```

That tag triggers `.github/workflows/release-surface-plugins.yml`, which publishes clients and builds the current CLI and plugin archives:

- `allwright.dev` Go module publish by creating `go/vX.Y.Z`, verifying the `go/` module, and warming `proxy.golang.org`
- `allwright` publish to PyPI after syncing `python/pyproject.toml` from the tag
- `@allwright.dev/core` publish to npm after syncing `typescript/core/package.json` from the tag
- `@allwright.dev/vitest` publish to npm after syncing `typescript/vitest/package.json` and its dependency on `@allwright.dev/core` from the tag
- `allwright` CLI archives for the current OS matrix
- `allwright-surface-web` plugin archives for the current OS matrix
- `allwright-surface-mobile-android` plugin archives for the current OS matrix
- `allwright-surface-mobile-ios` plugin archives with Simulator and physical-device XCUITest runners for both macOS architectures
- `allwright-surface-desktop-mac` plugin archives with a macOS XCUITest runner for both macOS architectures
- `allwright-surface-desktop-windows` plugin archives with a self-contained FlaUI UIA3 agent for Windows x64
- crates.io publish for every Rust core, CLI, and surface-plugin crate after syncing and verifying every crate version from the tag

- Linux `x86_64-unknown-linux-gnu`
- Windows `x86_64-pc-windows-msvc`
- macOS `aarch64-apple-darwin`

Configure the `CARGO_REGISTRY_TOKEN` repository secret before pushing a release tag if you want the crates.io publish job to succeed.
The release workflow also sets `CARGO_PUBLISH_ALLOW_DIRTY=1` because it syncs crate versions from the tag inside CI before calling `cargo publish`.
Configure PyPI Trusted Publishing for `allwright` before pushing a release tag:

- owner: `allwright-dev`
- repository name: `allwright`
- workflow name: `release-surface-plugins.yml`
- environment name: `pypi`

The Python publish job uses the `pypi` GitHub environment plus OIDC and does not require a PyPI API token.
Configure npm Trusted Publishing for `@allwright.dev/core` on npmjs.com before pushing a release tag:

- provider: `GitHub Actions`
- organization or user: `allwright-dev`
- repository: `allwright`
- workflow filename: `release-surface-plugins.yml`
- allowed action: `npm publish`

Configure npm Trusted Publishing for `@allwright.dev/vitest` with the same values.
Configure npm Trusted Publishing for `create-allwright` with the same values.

The npm publish job uses the `Prod` GitHub environment plus OIDC instead of an `NPM_TOKEN`, which avoids bypass-2FA tokens entirely.

## Publishing The Go Module

The Go client is published as the vanity import path `allwright.dev`, backed by the `go/` subdirectory in this repository.

You only create the root release tag manually:

```bash
git tag vX.Y.Z
git push origin vX.Y.Z
```

The release workflow creates the Go-specific tag `go/vX.Y.Z` on the same commit, which is the format Go requires for a module rooted in the `go/` subdirectory. The workflow also runs `go mod tidy`, verifies `go.mod` and `go.sum` stay clean, runs `go test ./...`, and asks `proxy.golang.org` for `allwright.dev@vX.Y.Z` to help the new version show up faster.

Consumers can then install or upgrade with:

```bash
go get allwright.dev@vX.Y.Z
```

The `allwright-dev/` site already serves the `go-import` metadata for `allwright.dev`, so `go get` can resolve the vanity import path back to this repository's `go/` subdirectory.

## Publishing The Java Package

The Java client is published from the `java/` directory to Maven Central as `dev.allwright:allwright`.

You only create the root release tag manually:

```bash
git tag vX.Y.Z
git push origin vX.Y.Z
```

The release workflow builds `java/` with the checked-in Gradle wrapper using `ALLWRIGHT_VERSION=X.Y.Z`, then publishes to Maven Central by uploading through Sonatype's Central Portal OSSRH Staging API compatibility service and transferring the deployment into the Central Publisher Portal.

Consumers can then depend on it with Gradle:

```kotlin
dependencies {
    implementation("dev.allwright:allwright:X.Y.Z")
}
```

or Maven:

```xml
<dependency>
    <groupId>dev.allwright</groupId>
    <artifactId>allwright</artifactId>
    <version>X.Y.Z</version>
</dependency>
```

## Publishing The Python Package

The Python client is published from the `python/` directory as the PyPI project `allwright`.

You only create the root release tag manually:

```bash
git tag vX.Y.Z
git push origin vX.Y.Z
```

The release workflow syncs `python/pyproject.toml` to `X.Y.Z`, builds the source distribution and wheel from `python/`, and publishes them to PyPI through Trusted Publishing.

## Publishing The TypeScript Workspace

The TypeScript client lives in `typescript/core` as `@allwright.dev/core`.
The Vitest fixture package lives in `typescript/vitest` as `@allwright.dev/vitest`.
The project initializer lives in `typescript/create` as `create-allwright` and powers `npm init allwright`.
It can scaffold Web, Mobile Android, Mobile iOS, Desktop macOS, and Desktop Windows projects
interactively. Use `--ios` / `--surface mobile-ios` for scripted iOS setup or
`--macos` / `--surface desktop-mac` for a `macosApp` starter. Use `--windows` /
`--surface desktop-windows` for a `windowsApp` starter. Generated native projects
rely on lazy core plugin installation, so initialization does not require a
manual agent install.

You only create the root release tag manually:

```bash
git tag vX.Y.Z
git push origin vX.Y.Z
```

The release workflow syncs all three package versions to `X.Y.Z`, updates `@allwright.dev/vitest` to depend on the matching `@allwright.dev/core` version, builds the workspace, then publishes `@allwright.dev/core`, `@allwright.dev/vitest`, and `create-allwright` in that order.

## Installer Scripts

- `scripts/install.sh`: installs the latest or requested `allwright` CLI release on Linux and macOS
- `scripts/install.ps1`: installs the latest or requested `allwright` CLI release on Windows PowerShell
- installed CLIs can subsequently update themselves with `allwright update`
- `scripts/generate-go-proto.sh`: installs pinned Go protobuf generators locally under `go/.bin/` and regenerates the checked-in Go bindings from the canonical top-level `proto/` tree
- `scripts/generate-rust-proto.sh`: regenerates `rust/allwright/src/proto_generated.rs` from the canonical top-level `proto/` tree
- `scripts/sync-version.sh`: syncs the Rust workspace and internal crate versions from a release version string such as `X.Y.Z`
- `scripts/sync-npm-version.sh`: syncs the npm workspace package versions from a release version string such as `X.Y.Z`
- `scripts/sync-python-version.sh`: syncs the Python package version from a release version string such as `X.Y.Z`
- users do not need to clone the repo; both scripts can be run directly from GitHub with `curl`, `wget`, or PowerShell `irm`

Go proto regeneration:

```bash
./scripts/generate-go-proto.sh
```

This keeps `proto/` as the single source of truth while regenerating the checked-in Go bindings in `go/gen/allwright/engine/v1`.

Rust proto regeneration:

```bash
./scripts/generate-rust-proto.sh
```

This keeps `proto/` as the single source of truth while regenerating the checked-in Rust bindings in `rust/allwright/src/proto_generated.rs` and `rust/allwright/src/allwright.engine.v1.rs`.
CI also verifies that the checked-in Go and Rust generated proto outputs are up to date on pushes to `main` and on pull requests.

Both scripts support:

- `ALLWRIGHT_VERSION` to pin a specific release tag such as `vX.Y.Z`, or `latest`
- `ALLWRIGHT_INSTALL_DIR` to override the destination directory (default: `$HOME/.local/bin` on Linux/macOS, `%LOCALAPPDATA%\Microsoft\WindowsApps` on Windows)
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
bun run build
```

TypeScript playground:

```bash
bun run example:playground -- --server-addr 127.0.0.1:50051
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

Accessibility snapshots also accept `mode: "ai"` to inject queryable `aria-ref`
attributes and include matching IDs in JSON or YAML, for example
`page.accessibilitySnapshot({ mode: "ai", format: "json" })` in TypeScript.
See the [web snapshot documentation](rust/allwright-surface-web/README.md) for
all modes and reference lifetime.

### Semantic web locators

Web pages and locators support `getByRole`, `getByText`, `getByLabel`, `getByPlaceholder`, `getByAltText`, `getByTitle`, and `getByTestId`, with equivalent naming in Rust, Go, Java, and Python. Match strings or regexes, filter role states, and combine semantic locators with CSS/XPath chains:

```ts
await page.getByRole('listitem')
  .filter({ has: page.getByRole('heading', { name: 'Beta', exact: true }), visible: true })
  .getByRole('button', { name: 'Buy', disabled: false })
  .click();
await page.getByLabel('Email address').fill('you@example.com');
```

Android and iOS apps and locators share the mobile subset `getByRole`,
`getByText`, `getByLabel`, and `getByTestId`. These resolve against native
accessibility metadata and retain automatic action/read retries, so scripts do
not need polling loops:

```ts
await app.getByRole('button', { name: 'Login', exact: true }).click();
await app.getByLabel('Email', { exact: true }).fill('user@example.com');
```

Filters support `has`, `hasNot`, `hasText`, `hasNotText`, and `visible`; locators also support `first`, `last`, and `nth`. See the [web locator reference](rust/allwright-surface-web/SELECTORS.md) for options, language conventions, and current limits.

Exclude matching web elements with `locator.not(otherLocator)` (Python `not_`, Go
`Not`, Rust `not(&other)`). Vitest assertions on web and Android support both
`expect(locator).not.toHaveText('Loading')` and
`expect(locator).not().toBeVisible()`, with retrying negation. See the
[Vitest negation examples](typescript/vitest/README.md#negated-assertions-and-locator-exclusion).

Android apps also expose accessibility snapshots across all clients, including the Vitest `androidApp` fixture:

```ts
const tree = JSON.parse(await app.accessibilitySnapshot({ mode: "ai" }));
const yaml = await app.accessibilitySnapshot({ format: "yaml" });
// Use an aria-ref returned on a node in the AI tree:
await app.locator(`ref=${node["aria-ref"]}`).click();
```

Android uses a session-scoped cache of absolute XPath references without modifying the app or source XML. References expire and fail as stale when a fresh hierarchy differs; capture a new AI snapshot after screen changes. See the [Android snapshot contract](rust/allwright-surface-mobile-android/README.md#accessibility-snapshots) for modes and native limitations. iOS now has an experimental XCUITest-backed plugin with bundled Simulator and physical-device runners. It includes deep-link navigation, native file chooser/download hooks, full-page screenshots, and JSON/YAML accessibility snapshots with scoped AI references. The public [`Flights-simulator.ipa`](https://allwright.dev/Flights-simulator.ipa) is a universal Simulator sample that the plugin downloads, installs, and launches without manual provisioning. For a registered physical device, the plugin re-signs the prebuilt ARM64 runner from local Apple development credentials, forwards it through usbmuxd, and installs a supplied signed device `.app`/`.ipa` automatically.

Web pages and locators also support reading the current URL, live input values, selected dropdown options and textbox text, checkbox/radio checked state, attributes, and bounding boxes. See the [state capture examples](typescript/core/README.md#read-page-and-element-state).
