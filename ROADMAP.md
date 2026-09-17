# Allwright roadmap

## Playwright element-command parity

Audit date: 2026-09-17

Baseline: Playwright 1.62.1 / `playwright-go` v0.6201.1, matching the
versions used by this repository. The comparison is against Playwright's
[`Locator`](https://playwright.dev/docs/api/class-locator) and
[`LocatorAssertions`](https://playwright.dev/docs/api/class-locatorassertions)
APIs because locators are Playwright's current element interaction API.

Existing Allwright coverage includes click, right-click, double-click, hover,
fill, clear, focus, element screenshot, bounding-box capture, attribute
capture, count, text content, inner HTML, input value, visibility/hidden
checks, checked-state verification, and select by value or label.

### P0 — missing core element actions

- [ ] Upload files — `locator.setInputFiles()`; accept one file, multiple
  files, a directory, in-memory payloads, and an empty list to clear files.
- [ ] Check a checkbox/radio — `locator.check()`.
- [ ] Uncheck a checkbox — `locator.uncheck()`.
- [ ] Set checked state explicitly — `locator.setChecked(checked)`; this can
  share the implementation and UI with check/uncheck.
- [ ] Drag one element to another — `locator.dragTo(target)` with optional
  source and target positions.
- [ ] Drop external files or data on an element — `locator.drop(payload)`.
- [ ] Tap an element — `locator.tap()` for touch-enabled contexts.
- [ ] Blur an element — `locator.blur()`.
- [ ] Scroll an element into view — `locator.scrollIntoViewIfNeeded()`.
- [ ] Select an element's text — `locator.selectText()`.
- [ ] Press a key on a specific element — `locator.press(key)`. Allwright's
  current `press` command targets the page keyboard, not a WebElement.
- [ ] Type sequentially into a specific element —
  `locator.pressSequentially(text)`, including the per-key delay option.
- [ ] Dispatch a DOM event — `locator.dispatchEvent(type, eventInit)`.

### P1 — missing capture, state, and wait commands

- [ ] Capture rendered text — `locator.innerText()`.
- [ ] Capture rendered text from every match — `locator.allInnerTexts()`.
- [ ] Capture text content from every match — `locator.allTextContents()`.
- [ ] Capture an accessibility tree — `locator.ariaSnapshot()`.
- [ ] Capture/verify enabled state — `locator.isEnabled()`.
- [ ] Capture/verify disabled state — `locator.isDisabled()`.
- [ ] Capture/verify editable state — `locator.isEditable()`.
- [ ] Evaluate JavaScript against one matched element — `locator.evaluate()`.
- [ ] Evaluate JavaScript against all matched elements —
  `locator.evaluateAll()`.
- [ ] Wait for a custom element predicate — `locator.waitForFunction()`.
- [ ] Extend element waits to `attached` and `detached` states. The current
  wait command only exposes Playwright's `visible` and `hidden` states.
- [ ] Extend select-option support to option index and multiple values. The
  current commands expose only one value or one label.
- [ ] Highlight an element for debugging — `locator.highlight()`.
- [ ] Remove an element highlight — `locator.hideHighlight()`.

### P1 — missing web-first element assertions

Add positive and negative forms where Playwright supports them:

- [ ] Attached/detached — `toBeAttached()`.
- [ ] Enabled and disabled — `toBeEnabled()`, `toBeDisabled()`.
- [ ] Editable/read-only — `toBeEditable()`.
- [ ] Empty/non-empty — `toBeEmpty()`.
- [ ] Focused/not focused — `toBeFocused()`.
- [ ] In/out of viewport, including intersection ratio — `toBeInViewport()`.
- [ ] Contains CSS classes — `toContainClass()`.
- [ ] Accessible description — `toHaveAccessibleDescription()`.
- [ ] Accessible error message — `toHaveAccessibleErrorMessage()`.
- [ ] Accessible name — `toHaveAccessibleName()`.
- [ ] Attribute value — `toHaveAttribute()`.
- [ ] Exact class value/list — `toHaveClass()`.
- [ ] Computed CSS property — `toHaveCSS()`.
- [ ] DOM id — `toHaveId()`.
- [ ] JavaScript property — `toHaveJSProperty()`.
- [ ] ARIA role — `toHaveRole()`.
- [ ] Multiple selected values — `toHaveValues()`.
- [ ] ARIA snapshot — `toMatchAriaSnapshot()`.

### Partial parity to finish while adding commands

- [ ] Expose the applicable Playwright options for existing actions: position,
  button, modifiers, click count, force, trial, delay, scroll behavior, and
  navigation waiting. Do not model these as separate commands.
- [ ] Add indeterminate checkbox support to the existing checked-state
  verification.
- [ ] Support regular expressions, case sensitivity, and `useInnerText` on
  text assertions.
- [ ] Support the useful element screenshot options (animations, caret, mask,
  omitted background, scale, style, and image type/quality).

### Explicitly out of scope as standalone commands

The following `Locator` methods are not missing user actions. They construct,
combine, narrow, or inspect locators and should be represented by Allwright's
element/locator model where needed: `all`, `and`, `or`, `filter`, `first`,
`last`, `nth`, `locator`, `frameLocator`, `contentFrame`, `getByAltText`,
`getByLabel`, `getByPlaceholder`, `getByRole`, `getByTestId`, `getByText`,
`getByTitle`, `normalize`, `describe`, `description`, and `page`.

`elementHandle`, `elementHandles`, and `evaluateHandle` are also excluded
because they expose low-level handles rather than portable test commands.
`type()` is deprecated by Playwright; Allwright should implement
`pressSequentially()` instead. `err()` is a Go binding helper, not a
Playwright element command.

### Definition of done for each new command

- The command is declared for `WebElement` in the command catalog.
- Executor preparation, reporting, and Playwright execution are implemented.
- AI/web-intent mapping is added where the command has a natural-language
  equivalent.
- Playwright project export either emits the command or returns an explicit,
  tested unsupported-export error.
- Unit tests cover successful execution, timeout/error reporting, variable
  capture (when applicable), and iframe/parent-locator resolution.
- The frontend command editor can configure all required inputs without raw
  JSON.
