# allwright-surface-web

Installable web surface plugin for the allwright engine.

## Protocol Rule

This is a hard rule for all future web-plugin development: Chromium web element operations must execute through WebDriver BiDi. This includes click, hover, focus, fill, key input, selector checks, text reads, highlighting, and screenshots.

CDP is permitted only for Chromium browser and tab lifecycle, bootstrapping the Chromium BiDi mapper, and transporting commands to that mapper. It must not be used to inspect the DOM or dispatch user input in a normal web automation path.

## Accessibility snapshots

`page.accessibilitySnapshot({ format: "json" | "yaml" })` returns a serialized
string. JSON is the default. Both formats encode the same ordinary mappings,
arrays, strings, booleans, and numbers; role/name/state are separate fields.
There are no Playwright snapshot keys, regular expressions, or custom YAML tags
to interpret. All YAML string keys and values are quoted so YAML 1.1 and 1.2
parsers preserve names such as `yes`, `on`, and date-like strings. Use
`JSON.parse`, `json.loads`, or a standard YAML parser.

```ts
const snapshot = JSON.parse(await page.accessibilitySnapshot());
const yaml = await page.accessibilitySnapshot({ format: "yaml", timeoutMs: 10_000 });
```

Other clients expose the same operation:

| Client | YAML snapshot |
| --- | --- |
| Rust | `page.accessibility_snapshot_with_options(AccessibilitySnapshotOptions { format: AccessibilitySnapshotFormat::Yaml, ..Default::default() }).await?` |
| Go | `page.AccessibilitySnapshot(ctx, AccessibilitySnapshotOptions{Format: "yaml"})` |
| Python | `page.accessibility_snapshot(AccessibilitySnapshotOptions(format="yaml"))` |
| Java | `page.accessibilitySnapshot(new AccessibilitySnapshotOptions("yaml"))` |

The [JSON Schema](accessibility.schema.json) describes version 1. For example:

```yaml
version: 1
documents:
  - contextId: page-context
    parentContextId: null
    url: https://example.test/
    title: Example
    root:
      role: document
      name: Example
      states: {}
      properties: {}
      children:
        - role: checkbox
          name: Accept terms
          states:
            checked: mixed
          properties: {}
          children: []
```

`documents` contains the top page followed by its iframe documents in depth-first
order. `parentContextId` links each frame document to its parent; context IDs are
opaque browser session identifiers, not selectors or persistent element IDs.
Each document is collected in its own browsing context, including cross-origin
frames. Frames retain `iframe` nodes in their parent's DOM tree, while their
contents appear once in `documents`. Frame documents are collected independently,
including frames whose embedding element is hidden. A detached frame or script
failure fails the command instead of silently returning an incomplete snapshot;
normal command timeout/retry options apply. Frames are sampled sequentially, so
this is not an atomic snapshot of a page that is changing during collection.

Nodes have `role`, accessible `name`, `states`, `properties`, and ordered
`children`. Ordinary text uses `role: text`. Containers without accessibility
semantics are flattened. States include native and explicit ARIA checked,
selected, expanded, pressed, disabled, required, readonly, heading level, and
numeric range values where available. Undefined states are omitted, explicit
false remains a boolean, and mixed check states are strings. Properties include
accessible descriptions, control values, placeholders, link URLs, and ARIA
metadata. Password and file input values are omitted.

The collector walks the composed DOM, including open shadow roots, assigned
slots, and `aria-owns` relationships with cycle/duplicate protection. By default it excludes
hidden and inert subtrees within each document, but includes off-screen content.
Referenced hidden labels can still contribute to accessible names. Collection
and serialization live in the web plugin, and execution uses WebDriver BiDi for
both Chromium and Firefox.

This is a DOM-derived accessibility representation, not the browser's native
platform accessibility tree or an accessibility conformance audit. Closed shadow
roots, browser-internal widgets, canvas internals, and accessibility semantics
exposed only through ElementInternals are not available to this DOM walker.
Native role/name behavior can differ from the browser's accessibility engine.
Role, name, description, and visibility computation use
[Allwright-owned accessibility semantics](ACCESSIBILITY.md). Playwright and W3C
specifications serve as behavioral references; no external accessibility library
or copied Playwright code is bundled. This independent implementation does not
claim Playwright-equivalent maturity or complete specification conformance.

Validation:

```sh
cargo test -p allwright-surface-web --lib
cargo test -p allwright-surface-web --test accessibility -- --ignored
ALLWRIGHT_TEST_BROWSER=firefox cargo test -p allwright-surface-web --test accessibility -- --ignored
```

The opt-in browser test launches a temporary browser profile and a local fixture
server. It covers structured JSON/YAML equivalence, labels/descriptions, typed
states, hidden content, password values, shadow DOM/slots, ARIA ownership cycles,
off-screen content, and same-origin/cross-origin frames.

Snapshot modes are independent of the output format:

| Mode | Behavior |
| --- | --- |
| `default` (default) | Accessible content; generic wrappers flattened; no DOM writes. |
| `ai` | Accessible or rendered content, including generic elements. Adds `aria-ref` attributes and matching snapshot fields for rendered elements that receive pointer events. |
| `autoexpect` | Content must be both accessible and rendered; no DOM writes. |
| `codegen` | Default semantic tree with the same standard JSON/YAML encoding. No pattern or regular-expression syntax. |

```ts
const snapshot = JSON.parse(await page.accessibilitySnapshot({ mode: "ai", format: "json" }));
// For a node in the current document:
await page.click(`[aria-ref="${node["aria-ref"]}"]`);
```

References are opaque strings, unique across documents with a random document
prefix. They stay stable while the element, role, and accessible name remain
unchanged. Query them in the snapshot document's frame (or within its open shadow
root); a plain document CSS query does not cross frame or shadow boundaries.
Text nodes and non-rendered/non-pointer elements have no reference. A reference
does not guarantee an element is enabled, unobscured, or currently actionable.
On the next AI capture, stale attributes assigned by the collector are removed;
other modes leave existing attributes alone. Navigation creates a new reference
registry. The collector owns `aria-ref` on referenced elements and replaces any
existing value there. DOM changes can invalidate references; capture again when
needed. These modes take behavioral inspiration from Playwright but do not claim
identical tree output.

### Iframes as pages

Resolve an iframe locator to a page using `await page.locator("iframe").Frame({ timeoutMs: 10_000 })`
(TypeScript; `.frame()` is also supported). Go uses `locator.Frame(ctx, CommandOptions{Timeout: ...})`,
Rust uses `locator.frame_with_options(...)`, and Java/Python use `locator.frame(options)`.
The returned page supports ordinary locators and can resolve nested frames the same way.

Resolution requires exactly one iframe, waits for its document to reach `readyState = complete`
and remain free of DOM mutations for 200 ms, and retries within the command timeout
(default 10 seconds). This is a document readiness check, not a network-idle guarantee.
Cross-origin frames are resolved through [WebDriver BiDi window values](https://www.w3.org/TR/webdriver-bidi/#type-script-WindowProxyRemoteValue); their contents
are accessed in the child browsing context, without reading the child DOM from the parent.
Chromium uses the existing BiDi mapper transport; Firefox uses native BiDi.
Closing a frame page releases its Allwright session without closing the containing tab
or removing the iframe. A detached frame handle fails; resolve the locator again after replacement.
