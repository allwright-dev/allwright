# Development

This document is the local-first runbook for testing allwright from this repo without publishing anything.

## Rules

- the engine is the root for every surface
- clients talk only to the engine server
- the engine resolves and delegates to plugins
- this document covers local web and Android example flows only

## What works locally today

- web examples through the local engine plus local `web` plugin
- Android examples through the local engine plus local `mobile-android` plugin

## Prerequisites

- Rust and Cargo
- Bun
- Go
- Python 3
- Java 21
- Chromium or Firefox for web examples
- `adb` plus an Android emulator or device for Android examples

## Local environment

Run from the repo root:

```bash
cd ~/data/personal/gh/allwright
```

Use an isolated local allwright home:

```bash
export ALLWRIGHT_HOME="$PWD/.local/allwright-home"
export ALLWRIGHT_SERVER_ADDR="127.0.0.1:50051"
mkdir -p "$ALLWRIGHT_HOME"
```

Optional clean reset:

```bash
rm -rf "$ALLWRIGHT_HOME"
mkdir -p "$ALLWRIGHT_HOME"
```

## Build local engine and plugins

```bash
cargo build -p allwright -p allwright-surface-web -p allwright-surface-mobile-android
```

This stays fully local. `allwright plugin install ...` will use the locally built artifacts from this workspace when the versions match.

## Start the local engine

In one terminal:

```bash
cargo run -p allwright -- serve --listen-addr 127.0.0.1:50051
```

## Install local plugins

In another terminal:

```bash
export ALLWRIGHT_HOME="$PWD/.local/allwright-home"
cargo run -p allwright -- plugin install web
cargo run -p allwright -- plugin install mobile-android
```

## Shared Android example env vars

Android examples now assume the repo sample app contract by default:

- device: `emulator-5554`
- app id: `com.example.airticket`
- account selector: `Id=com.example.airticket:id/bottom_nav_account`
- email selector: `xpath=//*[@text="Email"]`
- fill value: `user@example.com`

So in the common case you only need a booted emulator/device plus the local engine and plugin.

Optional overrides:

```bash
export ALLWRIGHT_ANDROID_DEVICE="emulator-5554" # only if you want a non-default target
export ALLWRIGHT_ANDROID_APK_PATH="/absolute/path/to/app.apk" # optional, if you want to install from apk instead of app id
export ALLWRIGHT_ANDROID_APP_ID="com.example.airticket" # optional override
export ALLWRIGHT_ANDROID_APP_ACTIVITY="" # optional override
export ALLWRIGHT_ANDROID_TAP_SELECTOR="Id=com.example.airticket:id/bottom_nav_account" # optional override
export ALLWRIGHT_ANDROID_FILL_SELECTOR='xpath=//*[@text="Email"]' # optional override
export ALLWRIGHT_ANDROID_FILL_VALUE="user@example.com" # optional override
```

## TypeScript examples

Build the local packages once:

```bash
bun install
bun run build
```

Web example:

This matches the shared sample flow exactly:

- open `https://themoderninternet.vercel.app`
- click the `Visit page` button inside the `Form Inputs` card
- assert the `Form Inputs` heading

The checked-in web examples use this XPath locator by default:

```text
xpath=//div[contains(@class,'card')][.//h2[normalize-space()='Form Inputs']]//button[normalize-space()='Visit page']
```

After the click, the examples explicitly wait for this exact destination heading locator before reading text:

```text
xpath=//h1[text()="Form Inputs"]
```

```bash
bun run --filter @allwright.dev/core example:web
```

Android example:

```bash
bun run --filter @allwright.dev/core example:android
```

Files:

- [typescript/core/examples/web-basic.ts](typescript/core/examples/web-basic.ts)
- [typescript/core/examples/android-basic.ts](typescript/core/examples/android-basic.ts)

## Python examples

Install the local client editable:

```bash
python3 -m pip install -e ./python
```

Web example:

```bash
PYTHONPATH=python python3 python/examples/web_basic.py
```

Android example:

```bash
PYTHONPATH=python python3 python/examples/android_basic.py
```

Files:

- [python/examples/web_basic.py](python/examples/web_basic.py)
- [python/examples/android_basic.py](python/examples/android_basic.py)

## Go examples

Web example:

```bash
cd go
go run ./examples/web-basic
```

Android example:

```bash
cd go
go run ./examples/android-basic
```

Files:

- [go/examples/web-basic/main.go](go/examples/web-basic/main.go)
- [go/examples/android-basic/main.go](go/examples/android-basic/main.go)

## Rust examples

Web example:

```bash
cargo run -p allwright-core --example web_basic
```

Android example:

```bash
cargo run -p allwright-core --example android_basic
```

Files:

- [rust/allwright/examples/web_basic.rs](rust/allwright/examples/web_basic.rs)
- [rust/allwright/examples/android_basic.rs](rust/allwright/examples/android_basic.rs)

## Java examples

Use the opt-in JUnit example tests:

Web example:

```bash
cd java
export JAVA_HOME=$(/usr/libexec/java_home -v 21)
export GRADLE_USER_HOME="$PWD/../.gradle-user-home"
ALLWRIGHT_RUN_WEB_EXAMPLE=true ./gradlew --no-daemon test --tests dev.allwright.examples.WebBasicTest
```

## Web launch notes

On macOS, older local runs could print Firefox helper lines like `Firefox GPU Helper`, `RemoteAgent`, `WebDriver BiDi listening`, or `Exiting due to channel error` directly into the terminal. Those messages came from inherited browser stdio rather than from an allwright engine failure. The web plugin launch now detaches browser stdout and stderr so the example path stays quiet.

Android example:

```bash
cd java
export JAVA_HOME=$(/usr/libexec/java_home -v 21)
export GRADLE_USER_HOME="$PWD/../.gradle-user-home"
ALLWRIGHT_RUN_ANDROID_EXAMPLE=true ./gradlew --no-daemon test --tests dev.allwright.examples.AndroidBasicTest
```

Files:

- [java/src/test/java/dev/allwright/examples/WebBasicTest.java](java/src/test/java/dev/allwright/examples/WebBasicTest.java)
- [java/src/test/java/dev/allwright/examples/AndroidBasicTest.java](java/src/test/java/dev/allwright/examples/AndroidBasicTest.java)

## Quick verification commands

These are useful after editing client code:

```bash
bun run build
```

```bash
cd go && go test ./...
```

```bash
python3 -m compileall python/allwright python/examples
```

```bash
cd rust && cargo test -p allwright
```

```bash
cd java && GRADLE_USER_HOME="$PWD/../.gradle-user-home" ./gradlew --no-daemon build
```
