# allwright-surface-desktop-windows

Runtime-loaded Windows desktop plugin for Allwright. The plugin starts a bundled,
self-contained .NET agent on a private dynamic loopback port. The agent uses Microsoft
UI Automation 3 COM APIs directly, with Allwright-owned selectors, input, capture,
window discovery, and provider timeouts for Win32, WinForms, WPF, UWP, and WinUI apps.

The agent is deliberately isolated from the engine process. Windows UI Automation
is COM-based and a non-responsive application can wedge an automation call; the
sidecar boundary and bounded provider calls keep Win32 window discovery, capture,
and live input available when an accessibility provider is slow. Requests use typed
JSON over loopback HTTP. No FlaUI, Appium,
WinAppDriver, Developer Mode, or separately installed .NET runtime is required.

Supported shared desktop operations are app launch/close, Playwright-style native
locators, universal-link and custom URL-scheme dispatch through `app.goto(...)`,
click, count, focus, fill, key input, text reads, input/checked/attribute/bounding-box state reads, waits, PNG screenshots, and
JSON/YAML accessibility snapshots. `app_id` is an executable path, a command
resolvable by Windows such as `notepad.exe`, or a packaged-app AUMID containing `!`.

```ts
const windows = await desktop.windows.connect();
const app = await windows.launch({ appId: "notepad.exe" });
await app.getByRole("textbox").fill("Hello from Allwright");
```

Release archives contain `lib/allwright_surface_desktop_windows.dll` and the
self-contained `agent/Allwright.WindowsAgent.exe`. Set `ALLWRIGHT_WINDOWS_AGENT`
to use a locally published agent while developing the plugin.
