# allwright Python client

This folder contains the high-level Python client for the allwright engine.

It loads the shared `proto/engine/v1/engine.proto` contract dynamically at runtime and exposes a browser/page API instead of raw gRPC channel setup.

## Install

```bash
pip install allwright
```

Installing from a local checkout instead:

```bash
pip install -e ./python
```

## Example

```python
from allwright import firefox

browser = firefox.launch()
page = browser.page()
page.goto("https://themoderninternet.vercel.app")
page.click(
    "xpath=//div[contains(@class,'card')][.//h2[normalize-space()='Form Inputs']]"
    "//button[normalize-space()='Visit page']"
)
page.wait_for_selector('xpath=//h1[text()="Form Inputs"]')
browser.close()
```

For an action that opens a tab, register the generic typed hook first:

```python
from allwright import hooks

hook = page.register_hook(hooks.new_page)
page.click("a[target=_blank]")
new_page = hook.wait()
```

```python
hook = page.register_hook(hooks.file_chooser)
page.click("button.open-upload")
chooser = hook.wait()
chooser.set_files("fixtures/document.pdf")
```

```python
hook = page.register_hook(hooks.download)
page.click("a.download-report")
download = hook.wait()
download.save_as(f"artifacts/{download.suggested_filename}")
```

These paths are local to the Python test process and are streamed through a
remote Allwright server when necessary. Android app contexts support
`hooks.file_chooser` and `hooks.download` with the same lifecycle.

Web state reads: `page.url()`, `locator.input_value()`, `locator.selected_options()`, `locator.selected_text()`, `locator.is_checked()`, `locator.get_attribute(name)`, and `locator.bounding_box()`. Element methods also accept selectors on `Page`. Selected options return `CapturedOption(value, label, index)` objects; boxes return `BoundingBox(x, y, width, height)` in frame/page viewport CSS pixels. Missing attributes, unsupported text selections, and hidden/zero-area boxes return `None`. Existing `text_content()` and `inner_text()` read element text. See the [shared semantics](../typescript/core/README.md#read-page-and-element-state).
