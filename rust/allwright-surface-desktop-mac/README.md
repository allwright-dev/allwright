# allwright-surface-desktop-mac

Runtime-loaded macOS desktop plugin for allwright. It drives native macOS
applications through a bundled, headless XCUITest runner; core only routes
opaque desktop and app-session handles.

The release archive contains the plugin dylib plus an unsigned prebuilt
`.xctestrun`, `AllwrightAgentUITests-Runner.app`, and runtime entitlement
template. On first `desktop.mac.connect(...)`, the plugin copies the agent into
a local cache, ad-hoc signs only that copy, and starts it on port 8200. No CI or
user certificate is selected. Set `agent_endpoint` in a client, or set both
`ALLWRIGHT_MAC_AGENT_XCTESTRUN` and `ALLWRIGHT_MAC_AGENT_ENTITLEMENTS` for
plugin development, to use explicit agent artifacts instead.

Supported operations are application launch/termination, native selectors,
click, count, focus, fill, key input, text reads, selector waits, screenshots,
and JSON/YAML accessibility snapshots. macOS 14 or newer with Xcode is
required, and the process running Xcode must be allowed to use UI automation
in macOS Privacy & Security settings. Windows and Linux desktop automation are
separate future plugins.
