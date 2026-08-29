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
