# Allwright iOS Agent

`AllwrightIOSAgent` is the native driver bridge for the `mobile-ios` surface.
iOS does not permit a normal host process to drive arbitrary app UI, so the
bridge lives in an XCUITest runner and performs all app and element operations
through XCTest.

The release workflow builds this target with `build-for-testing` and bundles
its `.xctestrun` plus runner app inside the `mobile-ios` plugin archive. The
plugin installs and starts that runner automatically for simulators. It listens
on port `8100` (or `ALLWRIGHT_IOS_AGENT_PORT`) and stays alive for engine
commands. The protocol is private to the Rust surface plugin; test clients must
connect to the allwright engine, never directly to this server.

When invoking the runner from `xcodebuild`, disable parallel testing so XCTest
does not move the listener into an unreachable cloned simulator:

```sh
xcodebuild test-without-building \
  -project swiftui/AllwrightIOSAgent/AllwrightIOSAgent.xcodeproj \
  -scheme AllwrightIOSAgent \
  -destination 'platform=iOS Simulator,id=<simulator-udid>' \
  -parallel-testing-enabled NO
```

The agent owns auto-waiting. Action commands wait for an existing, enabled,
hittable element and reads wait for existence, using the command deadline.
Client scripts should issue one command and must not poll XCTest themselves.

The UI test target deliberately has no `TEST_TARGET_NAME`, and `testAgent` does
not launch `XCUIApplication()` without a bundle identifier. Therefore the
runner stays headless until a client asks it to launch the actual app under
test; the Allwright SwiftUI shell never covers that app.

The runner may target any installed app by bundle identifier using
`XCUIApplication(bundleIdentifier:)`. Physical devices still require
developer-team signing and host/device port forwarding.
