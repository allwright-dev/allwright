# allwright Java client

This folder contains the high-level Java client for the allwright engine.

The project generates Java protobuf and gRPC stubs from the shared `../proto/engine/v1/engine.proto` contract during the Gradle build, then exposes a browser/page API instead of raw channel setup.

## Build

```bash
cd java
export JAVA_HOME=$(/usr/libexec/java_home -v 21)
./gradlew build
```

## Publish Prep

The Java artifact is configured for Maven Central publication under `dev.allwright`.

```bash
cd java
export JAVA_HOME=$(/usr/libexec/java_home -v 21)
ALLWRIGHT_VERSION=0.0.7 ./gradlew publishToMavenLocal
```

For Sonatype publishing, provide:

- `OSSRH_USERNAME`
- `OSSRH_PASSWORD`
- `SIGNING_KEY`
- `SIGNING_PASSWORD`

The tagged GitHub Actions release workflow now publishes the Java artifact to Maven Central as `dev.allwright:allwright` when those secrets are configured.

## Example

```java
import dev.allwright.client.Allwright;

try (Allwright.Browser browser = Allwright.firefox().launch()) {
    Allwright.Page page = browser.page();
    page.goTo("https://example.com");
    page.click("h1");
}
```
