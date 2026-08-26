# allwright Java client

This folder contains the high-level Java client for the allwright engine.

The project generates Java protobuf and gRPC stubs from the shared `../proto/engine/v1/engine.proto` contract during the Gradle build, then exposes a browser/page API instead of raw channel setup.

## Build

```bash
cd java
gradle build
```

## Example

```java
import dev.allwright.client.Allwright;

Allwright.Browser browser = Allwright.firefox().launch();
Allwright.Page page = browser.page();
page.goTo("https://example.com");
page.click("h1");
browser.close();
```
