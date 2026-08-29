# allwright-surface-mobile-android

Android plugin for the allwright mobile surface.

Current design direction:

- Use `UiAutomator2` as the Android automation backend.
- Support native Android views, hybrid apps, and browser-wrapped/WebView-backed apps as first-class targets.
- Mirror the `web` surface contract: clients send a `connect` command with Android-specific options and receive a session plus initial page handle back.
- Resolve `connect` through ADB device discovery: use the requested device name or serial when provided, otherwise attach to the first available Android device or emulator.
- Keep page-scoped commands aligned with `web`. The first implemented cut is `connect`, `launch_app`, and `click_element`.
- Reuse the same selector transport convention clients already know: `css=...` and `xpath=...`, including chained locator segments.
- Bootstrap sessions through connected devices or emulators discovered over ADB.
- Ship a standalone runtime artifact so `allwright plugin install mobile-android` can download the plugin library from GitHub Releases.

Current local smoke path:

- `connect` discovers devices through `adb devices -l`, selects the named/serial-matched device when provided, and otherwise uses the first available `device` entry.
- `launch_app` installs the APK with `adb install -r`, resolves the package from `--app-id` or best-effort APK metadata parsing, then starts the app with `am start` or `monkey`.
- `click_element` uses the bundled Python bridge in `scripts/mobile_android_uiautomator2_bridge.py`, so the selected Python environment must have `uiautomator2` installed.
- `allwright plugin install mobile-android` installs the standalone runtime library, but a full engine-routed Android session path still depends on the remaining mobile server integration work.
- Run the smoke test with `bash scripts/test-mobile-android.sh /path/to/app.apk 'xpath=//*[@text="Login"]' --app-id your.package.name`.
