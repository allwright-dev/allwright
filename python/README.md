# allwright Python client

This folder contains the high-level Python client for the allwright engine.

It loads the shared `proto/engine/v1/engine.proto` contract dynamically at runtime and exposes a browser/page API instead of raw gRPC channel setup.

## Install

```bash
pip install -e ./python
```

## Example

```python
from allwright import firefox

browser = firefox.launch()
page = browser.page()
page.goto("https://example.com")
page.click("h1")
browser.close()
```
