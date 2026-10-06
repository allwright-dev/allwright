# allwright-surface-desktop-windows

Runtime-loaded Windows desktop plugin for Allwright. The plugin starts a bundled,
self-contained .NET agent on loopback port `8300`; the agent uses FlaUI's UIA3
backend to automate Win32, WinForms, WPF, UWP, and WinUI applications.

The agent is deliberately isolated from the engine process. Windows UI Automation
is COM-based and a non-responsive application can wedge an automation call; the
sidecar boundary lets the engine time out a request and replace the agent without
embedding UIA state in core. Requests use typed JSON over loopback HTTP. No Appium,
WinAppDriver, Developer Mode, or separately installed .NET runtime is required.

Supported shared desktop operations are app launch/close, Playwright-style native
locators, click, count, focus, fill, key input, text reads, waits, PNG screenshots,
and JSON/YAML accessibility snapshots. `app_id` is an executable path, a command
resolvable by Windows such as `notepad.exe`, or a packaged-app AUMID containing `!`.

```ts
const windows = await desktop.windows.connect();
const app = await windows.launch({ appId: "notepad.exe" });
await app.getByRole("textbox").fill("Hello from Allwright");
```

Release archives contain `lib/allwright_surface_desktop_windows.dll` and the
self-contained `agent/Allwright.WindowsAgent.exe`. Set `ALLWRIGHT_WINDOWS_AGENT`
to use a locally published agent while developing the plugin.
