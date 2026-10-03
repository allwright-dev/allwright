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

The runtime supports connect, app provisioning and launch, click, count, focus,
fill, key input, text reads, selector waits, screenshots, close, deep links,
file chooser/download hooks, and JSON or YAML accessibility snapshots with AI
element references. WebViews participate through the accessibility elements
XCTest exposes; arbitrary in-page JavaScript and a general WebView CSS/DOM
session are outside the native command surface.

## Install and run

The macOS release archive bundles the dylib plus separate prebuilt Simulator
and physical-device `.xctestrun`/runner app pairs. Install them together with:

```sh
allwright plugin install mobile-ios
```

The normal client auto-install path downloads the same archive when any bundled
component is absent. On `mobile.ios.connect()`, the plugin selects the requested
Simulator or physical device, starts the bundled runner with parallel testing
disabled, and waits for port `8100`. The runner has no target host application,
so it never presents an Allwright screen over the app under test.

Connect through a high-level client. TypeScript example:

   ```ts
   import { mobile } from "@allwright.dev/core";

   const device = await mobile.ios.connect({ device: "iPhone 17 Pro" });
   const app = await device.launch({
     appPath: "https://allwright.dev/Flights-simulator.ipa",
   });
   await app.getByRole("button", { name: "Login", exact: true }).click();
   ```

The public sample IPA contains a universal (`arm64` and `x86_64`) Simulator
build. `appPath` accepts a local `.app` directory, a local `.zip`/`.ipa` archive,
or an HTTP(S) archive URL. The plugin downloads and extracts archives, installs
the app on the selected target, reads its bundle identifier, and launches it.
`appId` is optional when `appPath` is present; when both are present they must
match. Simulator bundles must contain a Simulator binary. Physical-device
bundles must contain an ARM64 device binary with valid signing for that device;
the two formats are not interchangeable.

Actions and reads auto-wait inside the native bridge. `click`, `focus`, `fill`,
and key input wait for the target to exist, be enabled, and become hittable;
text reads wait for the target to exist. Client scripts must not add polling
loops around these commands. Pass the normal command `timeoutMs` only when the
default 10-second deadline is not appropriate.

Selectors use the common mobile transport. `id=`/`css=#...` resolve iOS
accessibility identifiers; `text=` resolves labels/values; `className=` accepts
XCTest element type names; basic XPath name/label/type forms are supported.
Apps and locators also expose Playwright-shaped `getByRole`, `getByText`,
`getByLabel`, and `getByTestId` builders. Strings support exact or
case-insensitive substring matching, regular expressions are supported, and
role locators accept `name`, `checked`, `disabled`, and `selected`. Chained
semantic locators stay scoped to native descendants and use the same automatic
waiting as raw selectors.

`app.goto(url)` / `app.navigate(url)` opens a universal link or custom URL
scheme through `simctl openurl` on Simulators and the device URL payload route
on physical devices. `screenshot({ fullPage: true })` scrolls the first native
scroll container, stitches its captures, and restores the original position.

Native app contexts support the same `fileChooser` and `download` hook shape as
Android. The engine performs the polling internally. Uploads are staged into a
Files/app Documents container and selected through XCTest; downloads are
observed in the launched app or Files Documents/Downloads containers, checked
for a stable size, copied to engine staging, and streamed back to the client.
Private containers and cloud-only files fail clearly rather than pretending to
be portable.

Accessibility snapshots support `default`, `codegen`, `autoexpect`, and `ai`
modes in JSON or YAML. AI mode adds scoped `aria-ref` values to actionable
elements, which can be targeted with `app.locator("ref=<id>")`. A reference is
invalidated when the native accessibility hierarchy changes and must be
refreshed from a new snapshot.

## Physical devices

Pass a connected device's name or UDID through the normal `device` connect
option. The plugin copies the prebuilt ARM64 runner into its local cache,
discovers an Apple Development identity and a provisioning profile containing
that device, materializes wildcard entitlements, re-signs the runner, starts it
through XCTest, and forwards port `8100` through the system usbmuxd socket. No
agent source checkout, manual runner installation, or separate `iproxy`
process is required.

Apple still requires the device to be trusted, Developer Mode/UI Automation to
be enabled, and the device to be registered in a local development provisioning
profile. To avoid ever replacing an unrelated installed app, automatic
discovery only uses wildcard or Allwright-specific profiles. A dedicated exact
profile may be selected explicitly for CI or advanced setups:

```sh
export ALLWRIGHT_IOS_SIGNING_IDENTITY="Apple Development: Example (TEAMID)"
export ALLWRIGHT_IOS_PROVISIONING_PROFILE=/path/to/profile.mobileprovision
export ALLWRIGHT_IOS_AGENT_BUNDLE_ID=com.example.allwright-agent.xctrunner
```

`appPath` uses the same local path/archive/URL input for a physical device, but
the contained `.app` must be an ARM64 device build already signed for that
device. The plugin extracts and installs it automatically with `devicectl`.
Simulator apps and physical-device apps are not interchangeable.

Passing a custom `agentEndpoint` still disables automatic runner bootstrap and
attaches to that endpoint directly.
