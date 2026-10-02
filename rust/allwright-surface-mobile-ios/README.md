# allwright-surface-mobile-ios

iOS surface plugin for the allwright engine. The plugin is a runtime-loaded
Rust `cdylib`; native device actions are delegated to the XCUITest runner in
`swiftui/AllwrightIOSAgent`.

## Architecture

Clients keep the same shape used by other surfaces:

```text
client -> allwright gRPC engine -> mobile-ios plugin -> HTTP bridge -> XCUITest
```

Only the XCUITest process touches `XCUIApplication` and `XCUIElement`. The
plugin owns protocol translation and session routing. Clients never load the
plugin or call the agent directly.

The first runtime cut supports connect, app provisioning and launch, click,
count, focus, fill, key input, text reads, selector waits, screenshots, close,
and JSON accessibility source capture. File chooser/download hooks, deep links,
and direct WebView DOM automation are not yet supported.

## Install and run

The macOS release archive bundles the dylib, `.xctestrun`, and XCTest runner
app. Install them together with:

```sh
allwright plugin install mobile-ios
```

The normal client auto-install path downloads the same archive when any bundled
component is absent. On `mobile.ios.connect()`, the plugin selects the requested
simulator (or a booted/available iPhone), starts the bundled runner with
parallel testing disabled, and waits for port `8100`. The runner has no target
host application, so it never presents an Allwright screen over the app under
test.

Connect through a high-level client. TypeScript example:

   ```ts
   import { mobile } from "@allwright.dev/core";

   const device = await mobile.ios.connect({ device: "iPhone 17 Pro" });
   const app = await device.launch({
     appPath: "https://example.test/MyApp-simulator.zip",
   });
   await app.locator("id=sign-in").click();
   ```

`appPath` accepts a local `.app` directory, a local `.zip`/`.ipa` archive, or
an HTTP(S) archive URL. The plugin downloads and extracts archives, installs the
app on the selected simulator, reads its bundle identifier, and launches it.
`appId` is optional when `appPath` is present; when both are present they must
match. The app bundle must be built for the iOS Simulator. Device `.ipa` files
cannot run in a simulator, and physical-device installation/signing is not yet
part of this path.

Actions and reads auto-wait inside the native bridge. `click`, `focus`, `fill`,
and key input wait for the target to exist, be enabled, and become hittable;
text reads wait for the target to exist. Client scripts must not add polling
loops around these commands. Pass the normal command `timeoutMs` only when the
default 10-second deadline is not appropriate.

Selectors use the common mobile transport. `id=`/`css=#...` resolve iOS
accessibility identifiers; `text=` resolves labels/values; `className=` accepts
XCTest element type names; basic XPath name/label/type forms are supported.

## Physical-device constraint

Simulator provisioning is automatic. Physical devices still need a runner
signed for the developer team and host/device port forwarding; pass that
forwarded URL as `agentEndpoint`. A custom endpoint intentionally disables the
simulator auto-bootstrap path.
