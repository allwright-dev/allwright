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
let hook = page.register_hook(allwright::NEW_PAGE).await?;
page.click("a[target=_blank]").await?;
let new_page = hook.wait().await?;
```

```rust,no_run
let hook = page.register_hook(allwright::FILE_CHOOSER).await?;
page.click("button.open-upload").await?;
let chooser = hook.wait().await?;
chooser.set_file("fixtures/document.pdf").await?;
```

```rust,no_run
let hook = page.register_hook(allwright::DOWNLOAD).await?;
page.click("a.download-report").await?;
let download = hook.wait().await?;
download
    .save_as(format!("artifacts/{}", download.suggested_filename()))
    .await?;
```

These paths are local to the Rust test process and are streamed through a
remote Allwright server when necessary. Android app contexts expose the same
`register_hook(FILE_CHOOSER)` / `register_hook(DOWNLOAD)` lifecycle and return
mobile chooser/download handles.

Web state reads: `page.url().await`, `locator.input_value().await`, `locator.selected_options().await`, `locator.selected_text().await`, `locator.is_checked().await`, `locator.get_attribute(name).await`, and `locator.bounding_box().await`. Element methods also accept selectors on `Page`; `_with_options` variants accept `CommandOptions`. Selected options return `Vec<CapturedOption>`; boxes return `Option<BoundingBox>` in frame/page viewport CSS pixels. Missing attributes, unsupported text selections, and hidden/zero-area boxes return `None`. Existing `text_content()` and `inner_text()` read element text. See the [shared semantics](../../typescript/core/README.md#read-page-and-element-state).
