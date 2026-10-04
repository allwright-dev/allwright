# create-allwright

Create a new allwright test project with:

```bash
npm init allwright@latest
```

Or:

```bash
npm create allwright@latest
```

The initializer prompts for:

- `TypeScript` or `JavaScript`
- one or more surfaces such as `Web`, `Mobile Android`, `Mobile iOS`, and `Desktop macOS`
- an optional target directory

It scaffolds a Node project with `package.json`, Vitest config, shared allwright config, starter tests, and a short README for the generated app.

Select Mobile iOS interactively, or scaffold it non-interactively:

```bash
npm init allwright@latest ios-tests -- --yes --ios
```

Select Desktop macOS interactively, or scaffold a `macosApp` project
non-interactively:

```bash
npm init allwright@latest macos-tests -- --yes --macos
```

Use `--all` to scaffold Web, Mobile Android, Mobile iOS, and Desktop macOS
together. The repeatable `--surface` flag supports the ids `web`,
`mobile-android`, `mobile-ios`, and `desktop-mac` for explicit combinations.

This writes an `iosApp` starter test and configures the public universal
Simulator IPA at `https://allwright.dev/Flights-simulator.ipa`. On the first
test run, core installs the `mobile-ios` plugin and its bundled headless XCTest
agent, then the fixture downloads and installs the sample app. Users do not
install either component manually.

The Desktop macOS starter configures the installed Calculator app and writes a
lazy `macosApp` screenshot test. Its first run installs `desktop-mac`, starts a
locally signed copy of the bundled XCUITest runner, and launches Calculator.
It requires macOS 14 or newer, Xcode, and UI Automation permission for the
process running Xcode.

Dependencies are installed automatically with the package manager that ran the initializer. Existing lockfiles take precedence; use `--package-manager npm|yarn|pnpm|bun` to override detection, or `--no-install` to scaffold without installing.
