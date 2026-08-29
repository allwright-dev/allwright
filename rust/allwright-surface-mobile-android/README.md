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
