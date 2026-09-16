# allwright-core Rust crate

`allwright-core` is the lightweight Rust engine core for the project.

It includes:

- a high-level Rust client API for talking to the allwright engine
- the engine server implementation used by the installable `allwright` CLI
- the shared plugin catalog used by CLI-side plugin registration

The surrounding Rust workspace now also publishes:

- `allwright`
- `allwright-plugin-sdk`
- `allwright-surface-web`
- `allwright-surface-mobile`
- `allwright-surface-mobile-android`
- `allwright-surface-mobile-ios`
- `allwright-surface-desktop`
- `allwright-surface-desktop-mac`
- `allwright-surface-desktop-windows`
- `allwright-surface-desktop-linux`

Installing the `allwright` package is intended to provide the CLI plus this lightweight core together, while surface crates are added separately as plugins.

Typed hooks use one generic registration/wait lifecycle. For example, register
the web-owned new-page hook before the action that opens a tab:

```rust,no_run
let hook = browser.register_hook(allwright::NEW_PAGE).await?;
page.click("a[target=_blank]").await?;
let new_page = hook.wait().await?;
```
