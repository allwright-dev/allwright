# allwright Java client

This folder contains the high-level Java client for the allwright engine.

The project generates Java protobuf and gRPC stubs from the shared `../proto/engine/v1/engine.proto` contract during the Gradle build, then exposes a browser/page API instead of raw channel setup.

## Build

```bash
cd java
export JAVA_HOME=$(/usr/libexec/java_home -v 21)
./gradlew build
```

## Install

The client is published to Maven Central as `dev.allwright:allwright`.

Gradle:

```kotlin
dependencies {
    implementation("dev.allwright:allwright:X.Y.Z")
}
```

Maven:

```xml
<dependency>
    <groupId>dev.allwright</groupId>
    <artifactId>allwright</artifactId>
    <version>X.Y.Z</version>
</dependency>
```

## Publish Prep

For local testing without waiting on a tagged release:

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

Use a Central Portal user token for `OSSRH_USERNAME` / `OSSRH_PASSWORD`.

The tagged GitHub Actions release workflow publishes the Java artifact to Maven Central as `dev.allwright:allwright` by uploading through Sonatype's Central Portal OSSRH Staging API compatibility service and then transferring the deployment into the Central Publisher Portal automatically.

## Example

```java
import dev.allwright.client.Allwright;
import dev.allwright.client.Browser;
import dev.allwright.client.Page;

try (Browser browser = Allwright.firefox().launch()) {
    Page page = browser.page();
    page.goTo("https://example.com");
    page.click("h1");
}
```
