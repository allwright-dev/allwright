# @allwright.dev/vitest

Vitest fixtures for allwright with Playwright-style `browser` and `page`
injection, plus Android, iOS, macOS, and Windows app fixtures for native and hybrid tests.

Wrap the Vitest configuration with `allwrightVitestConfig`. It starts Allwright once before workers run and shuts it down once after the complete test run. Existing user global setup files remain supported and run after Allwright starts, then tear down before Allwright stops.

```ts
import { allwrightVitestConfig } from "@allwright.dev/vitest/config";
import { defineConfig } from "vitest/config";

export default allwrightVitestConfig(defineConfig({
  test: {
    globalSetup: ["./test/my-global-setup.ts"],
  },
}));
```

Install:

```bash
bun add -d vitest @allwright.dev/vitest
```

```ts
import { expect, test } from "@allwright.dev/vitest";

test("opens a page", async ({ page }) => {
  await page.goto("https://themoderninternet.vercel.app");
  await page.click(
    "xpath=//div[contains(@class,'card')][.//h2[normalize-space()='Form Inputs']]//button[normalize-space()='Visit page']",
  );
  await expect(page.locator('xpath=//h1[text()="Form Inputs"]')).toHaveText("Form Inputs");
});

test("opens an Android app", async ({ androidApp }) => {
  await androidApp.click('Id=com.example.airticket:id/bottom_nav_account');
});

test("opens an iOS app", async ({ iosApp }) => {
  await iosApp.getByRole("button", { name: "Login", exact: true }).click();
});

test("opens a macOS app", async ({ macosApp }) => {
  await macosApp.getByRole("button", { name: "New Document", exact: true }).click();
});

test("opens a Windows app", async ({ windowsApp }) => {
  await windowsApp.getByRole("textbox").fill("Hello from Allwright");
});
```

Checked-in example specs cover every implemented surface:
[Web](./examples/basic.spec.ts), [Android](./examples/android-flights.spec.ts),
[iOS](./examples/ios-flights.spec.ts),
[macOS](./examples/macos-calculator.spec.ts), and
[Windows](./examples/windows-notepad.spec.ts). Native specs use the app target
configured for their surface in `allwright.config.yaml` or
`allwright.config.json`.

The fixture package reads the shared stack-agnostic config format through `@allwright.dev/core`.
Use `allwright.config.yaml` by default, or `allwright.config.json` if you prefer. Both follow the same root schema in `allwright.schema.json`.

```yaml
schemaVersion: 1

server:
  addr: 127.0.0.1:50051

web:
  browser:
    name: firefox
    binary: /Applications/Firefox.app/Contents/MacOS/firefox

expect:
  timeoutMs: 7000
  intervalMs: 100

suites:
  smoke:
    web:
      browser:
        name: chromium
```

Vitest can override or select from that shared config without switching to a Vitest-specific file:

```ts
import { expect, test as base } from "@allwright.dev/vitest";

const test = base.extend({
  allwright: async ({}, use) => {
    await use({ suite: "smoke" });
  }
});
```

Android fixtures are available alongside the web fixtures:

```ts
import { expect, test as base } from "@allwright.dev/vitest";

const test = base.extend({
  allwright: async ({}, use) => {
    await use({
      android: {
        launchOptions: {
          apkPath: "/absolute/path/to/app.apk",
          appId: "com.example.airticket",
        },
      },
    });
  },
});

test("android only", async ({ androidApp }) => {
  await androidApp.fill('xpath=//*[@text="Email"]', "user@example.com");
  await expect(androidApp.locator('xpath=//*[@text="Email"]')).toHaveText("user@example.com");
});

test("hybrid web and android", async ({ page, androidApp }) => {
  await page.goto("https://themoderninternet.vercel.app");
  await androidApp.click('Id=com.example.airticket:id/bottom_nav_account');
});
```

iOS uses the same app and locator shape. Configure the native agent endpoint
and bundle identifier once, then keep platform bootstrap out of test bodies:

```yaml
# allwright.config.yaml
mobile:
  ios:
    device: iPhone 17 Pro
    app:
      binary: https://allwright.dev/Flights-simulator.ipa
```

Set `ALLWRIGHT_IOS_AGENT_ENDPOINT` when the agent is not reachable at
`http://127.0.0.1:8100`. The app value may also be a local `.app`, `.zip`, or
`.ipa`; the fixture downloads/extracts it when needed, installs it on the
selected simulator, derives its bundle identifier, and launches it. Override
it with `ALLWRIGHT_IOS_APP_PATH`. No manual installation or polling belongs in
the test. The test remains platform-client code only:

```ts
import { expect, test } from "@allwright.dev/vitest";

test("logs in", async ({ iosApp }) => {
  await iosApp.getByRole("button", { name: "Login", exact: true }).click();
  await iosApp.getByLabel("Email", { exact: true }).fill("user@example.com");
  await expect(iosApp.getByText("Welcome back", { exact: true })).toBeVisible();
});
```

macOS apps use an installed application bundle identifier. The fixture starts
the bundled desktop XCUITest runner and launches the configured app lazily:

```yaml
# allwright.config.yaml
desktop:
  mac:
    app:
      id: com.apple.TextEdit
```

```ts
import { expect, test } from "@allwright.dev/vitest";

test("opens a document window", async ({ macosApp }) => {
  await macosApp.getByRole("button", { name: "New Document", exact: true }).click();
  await expect(macosApp.getByRole("textField")).toBeVisible();
});
```

Use `ALLWRIGHT_MAC_AGENT_ENDPOINT` for an external agent or
`ALLWRIGHT_MAC_APP_ID` to override the configured bundle identifier.

Windows apps use an executable path, command name, or packaged-app AUMID. The
fixture starts the bundled FlaUI UIA3 agent and launches the app lazily:

```yaml
# allwright.config.yaml
desktop:
  windows:
    app:
      id: notepad.exe
```

```ts
import { expect, test } from "@allwright.dev/vitest";

test("edits a document", async ({ windowsApp }) => {
  await windowsApp.getByRole("textbox").fill("Hello from Allwright");
  await expect(windowsApp.getByRole("textbox")).toContainText("Hello");
});
```

Use `ALLWRIGHT_WINDOWS_AGENT_ENDPOINT` for an external agent or
`ALLWRIGHT_WINDOWS_APP_ID` to override the configured application identifier.

Fixtures are lazy on first use. Injecting `browser`, `page`, `android`,
`androidApp`, `ios`, `iosApp`, `macos`, `macosApp`, `windows`, or `windowsApp`
does not launch or connect immediately; the underlying session is created only
when the test first performs an action through that fixture. Sync metadata
properties are only available after the lazy fixture has been realized by a
prior awaited call.

Available fixtures:

- `browser`: launched web browser
- `page`: initial web page
- `android`: connected Android device session
- `androidApp`: launched Android app page
- `ios`: connected iOS device session
- `iosApp`: launched iOS app page
- `macos`: connected macOS desktop session
- `macosApp`: launched macOS app page
- `windows`: connected Windows desktop session
- `windowsApp`: launched Windows app page

`androidApp` launches using `allwright.android.launchOptions` first, then falls back to `config.mobile.android`.
`iosApp` does the same with `allwright.ios.launchOptions` and `config.mobile.ios`.
`macosApp` does the same with `allwright.macos.launchOptions` and `config.desktop.mac`.
`windowsApp` does the same with `allwright.windows.launchOptions` and `config.desktop.windows`.

Android, iOS, macOS, and Windows apps and locators support the native action and
expectation subset: `click`, `count`, `focus`, `fill`, `press`, `textContent`, `innerText`,
`waitForSelector`, and `screenshot`. Mobile expectations provide `toHaveText`,
`toContainText`, `toHaveCount`, and `toBeVisible` with the same retry controls
as web fixtures. Hover and highlight remain web-only.

The opt-in accessibility integration test exercises the public page fixture,
bundled protobuf, running engine and dynamically loaded web plugin. From this
repository, after building the packages, run:

```sh
ALLWRIGHT_TEST_BROWSER=chromium bun run --cwd typescript/vitest vitest run tests/accessibility.spec.ts
ALLWRIGHT_TEST_BROWSER=firefox bun run --cwd typescript/vitest vitest run tests/accessibility.spec.ts
```

Set `ALLWRIGHT_TEST_LIVE_SITE=1` to reproduce the Form Inputs navigation on the
public example site. The default fixture is a local data URL. Use
`ALLWRIGHT_CLI_PATH` and `ALLWRIGHT_SERVER_ADDR` to select the runtime under test;
client, engine and installed web plugin versions must match.

### Negated assertions and locator exclusion

Every Allwright page/app and locator matcher supports both Vitest's `.not` form
and the callable `.not()` alias. Negation retries the opposite condition using
the same timeout and interval options:

```ts
await expect(page.getByRole('status')).not.toHaveText('Loading');
await expect(page.getByTestId('spinner')).not().toBeVisible();
await expect(page).not.toHaveCount('.error', 1);
await expect(androidApp.locator('text="Loading"')).not().toBeVisible();
```

Negative visibility succeeds when the element is hidden or absent. Selector,
transport, and session errors still fail; they are not evidence of invisibility.
Negative text assertions require a successful text read. Matcher objects are
immutable, and a second `.not` toggles back to positive assertions. Ordinary
Vitest value assertions retain their native `.not` syntax.

Web locators also support exclusion by element identity, with further chaining:

```ts
const enabledActions = page.getByRole('button')
  .not(page.getByRole('button', { disabled: true }));
await expect(enabledActions).not().toHaveCount(0);
```

Both locators must belong to the same page. `.not(other)` removes matching nodes;
`filter({ hasNot: other })` removes candidates containing matching descendants.
The same exclusion API is available in the core web clients as TypeScript/Java
`.not(other)`, Go `.Not(other)`, Rust `.not(&other)`, and Python `.not_(other)`.
Native Android locator exclusion is not exposed by this web selector API.

### Retrying state expectations

Assert against the page or locator so each attempt reads fresh state:

```ts
await expect(page).toHaveURL('https://example.com/dashboard');
await expect(page).toHaveURL(/\/dashboard(?:\?|$)/, { timeoutMs: 10_000 });
await expect(page.locator('input')).toHaveValue(/ready/i);
await expect(page.locator('select')).toHaveSelectedOptions(['a', 'b']);
await expect(page.locator('select')).toHaveSelectedOptions([{ value: 'a', label: /Alpha/ }]);
await expect(page.locator('textarea')).toHaveSelectedText('selected words');
await expect(page.locator('input[type=checkbox]')).toBeChecked();
await expect(page.locator('button[type=submit]')).toBeEnabled();
await expect(page.locator('input')).toBeEditable();
await expect(page.locator('input')).toBeFocused();
await expect(page.locator('.spinner')).toBeHidden();
await expect(page.locator('main')).toBeAttached();
await expect(page.locator('h1')).toHaveText('Welcome');
await expect(page.locator('a')).toHaveAttribute('href', /dashboard/);
await expect(page.locator('button')).toHaveBoundingBox({ width: 100, height: 40 });
await expect(page.locator('input[type=radio]')).not().toBeChecked();
```

Every element matcher also has a page/app form, such as `expect(page).toHaveValue('#name', 'Alice')` or `expect(androidApp).toBeEnabled('text="Continue"')`. Value, checked-state, attribute, bounding-box, attached/hidden, enabled/disabled, focused, and editable matchers work across web, Android, iOS, macOS, and Windows. URL, selected-option, and selected-text matchers remain web-only.

All Allwright page/app/locator expectations retry until they match or the timeout expires, including `.not` and `.not()`. Defaults come from the shared assertion configuration, falling back to 5 seconds and a 100 ms interval. Override these with `{ timeoutMs, intervalMs }`. In-flight reads and nested command timeout hints are bounded by the remaining assertion budget; `timeoutMs: 0` makes one observation without polling. Ordinary assertions on already captured values, such as `expect(await page.url()).toBe(...)`, use standard Vitest behavior and do not re-read the page.

String URL expectations match the complete URL exactly; regex expectations match the current URL on every attempt without modifying the regex's `lastIndex`. Values, selected text, and attributes also accept strings or regexes. `null` asserts a missing attribute or unsupported textbox selection; an empty string is distinct. Selected-option arrays match in order and require the same number of options. Entries can be exact values, value regexes, or objects specifying value, label, and/or index. Bounding-box expectations compare only the supplied coordinates/dimensions exactly in viewport CSS pixels; `null` expects no visible box. `.not.toBeChecked()` expects an unchecked control. Missing elements and read errors cannot satisfy negation.
