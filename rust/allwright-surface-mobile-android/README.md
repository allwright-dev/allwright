# allwright-surface-mobile-android

Android plugin for the allwright mobile surface.

Current design direction:

- Use a native Rust runtime that drives Android through `adb` and the built-in UI hierarchy dump/input tools.
- Support native Android views, hybrid apps, and browser-wrapped/WebView-backed apps as first-class targets.
- Mirror the `web` surface contract: clients send a `connect` command with Android-specific options and receive a session plus initial page handle back.
- Resolve `connect` through ADB device discovery: use the requested device name or serial when provided, otherwise attach to the first available Android device or emulator.
- Keep page-scoped commands aligned with `web`. The first implemented cut is `connect`, `launch_app`, `click_element`, and `fill_element`.
- Reuse the same selector transport convention clients already know: `css=...` and `xpath=...`, including chained locator segments.
- Accept UiAutomator2-style selector strategies directly from clients, including `text=...`, `textContains=...`, `textStartsWith=...`, `textMatches=...`, `description=...`, `descriptionContains=...`, `descriptionStartsWith=...`, `descriptionMatches=...`, `className=...`, `classNameMatches=...`, `resourceId=...`, `resourceIdMatches=...`, `packageName=...`, `packageNameMatches=...`, state flags such as `clickable=true` or `selected=true`, plus `index=` and `instance=`.
- Bootstrap sessions through connected devices or emulators discovered over ADB.
- Ship a standalone runtime artifact so `allwright plugin install mobile-android` can download the plugin library from GitHub Releases.

Current local smoke path:

- `connect` discovers devices through `adb devices -l`, selects the named/serial-matched device when provided, and otherwise uses the first available `device` entry.
- `launch_app` installs the APK with `adb install -r`, resolves the package from `--app-id` or best-effort APK metadata parsing, then starts the app with `am start` or `monkey`.
- `click_element` and `fill_element` resolve selectors from the dumped Android UI hierarchy and drive interaction through native `adb shell input` commands.
- `allwright plugin install mobile-android` installs the standalone runtime library, but a full engine-routed Android session path still depends on the remaining mobile server integration work.
- Run the smoke test with `bash scripts/test-mobile-android.sh /path/to/app.apk 'xpath=//*[@text="Login"]' --app-id your.package.name`.

## Accessibility snapshots

Android app contexts in Rust, Go, Java, Python, TypeScript, and Vitest expose `accessibility_snapshot` / `AccessibilitySnapshot` / `accessibilitySnapshot`, using the same options as web (`format`, `mode`, timeout). JSON is the default; YAML quotes strings to preserve their types in YAML 1.1 and 1.2.

The version-1 `documents` array contains the app context ID, native package/activity, platform, optional AI snapshot ID, and an `application` root. Each node has `role`, `name`, `states`, `properties`, and `children`. Native properties include class, resource ID, bounds and absolute XPath. Roles map common Android widget classes; names prefer content description over text. Password names are redacted. Unknown/custom classes remain generic. UI Automator exposes only a subset of accessibility semantics; this is not a browser ARIA tree or a full WebView DOM snapshot.

- `default`: retain the native hierarchy and exposed semantics.
- `codegen`: the same native tree in standard serialization.
- `autoexpect`: omit zero-area nodes while retaining their rendered descendants. Disabled nodes with bounds remain present.
- `ai`: add synthetic `aria-ref` fields on enabled, positive-area clickable, focusable or editable nodes.

No application source, live attributes, or dumped XML is modified. Use a returned reference with `app.locator("ref=<id>")` for click, fill, focus, key press, count, text reads, and waits. References must be standalone selectors. Explicit positional XPath, such as `xpath=/hierarchy/node[1]/node[2]`, is also supported; positions are one-based XML sibling positions, independent of Android's `index` attribute.

The plugin caches references by connection and app context, mapping each to an absolute XPath. Every reference lookup dumps fresh XML and compares the complete hierarchy and foreground package/activity before resolving the path. A mismatch invalidates the generation permanently; capture another AI snapshot to obtain new references. Unchanged AI snapshots reuse references. App close clears the cache; entries expire after ten minutes and the oldest are evicted above 128 app contexts. References are not durable identifiers: identical recycled nodes and screen changes between dump and input cannot be detected atomically.

The engine keeps lazily loaded libraries resident to preserve plugin state. Restart it after rebuilding/replacing a loaded plugin.

Run `cargo test -p allwright-surface-mobile-android -p allwright-plugin-sdk --offline` for unit coverage. For the simulated-device transport test, first build with `cargo build -p allwright -p allwright-surface-mobile-android`, then run `ALLWRIGHT_TEST_MOBILE_SNAPSHOT=1 bun test typescript/core/tests/mobile-snapshot.integration.test.ts` from the repository root. This exercises gRPC and the actual dynamic library, with a deterministic ADB fixture instead of a connected device.
