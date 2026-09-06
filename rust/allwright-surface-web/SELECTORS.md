# Web locators

Pages and locators in all five clients support semantic selectors alongside CSS and XPath. Every chained builder searches within the preceding locator; filters narrow its existing matches. Evaluation is fresh for each action or query.

```ts
const product = page.getByRole('listitem').filter({
  has: page.getByRole('heading', { name: 'Beta', exact: true }),
  hasNotText: /sold out/i,
  visible: true,
});
await product.getByRole('button', { name: 'Buy', disabled: false }).click();
await page.getByLabel('Email address').fill('you@example.com');
await page.getByTestId('products').locator('li').nth(1).getByText('Details').click();
```

| Builder | Matching |
| --- | --- |
| `getByRole` | Explicit or implicit ARIA role and accessible name |
| `getByText` | DOM text, whitespace normalized for string matching; excludes matching ancestors in favor of matching descendants |
| `getByLabel` | `aria-labelledby`, `aria-label`, or associated native labels |
| `getByPlaceholder` | `placeholder` attribute |
| `getByAltText` | `alt` on images, inputs, and image-map areas |
| `getByTitle` | `title` attribute |
| `getByTestId` | `data-testid`, exact and case-sensitive for strings |

Text arguments and role `name` accept strings or regexes. Strings default to case-insensitive substring matching; `exact: true` uses case-sensitive equality after whitespace normalization. Regexes use their own flags and ignore `exact`. Role options also include `checked`, `disabled`, `expanded`, `includeHidden`, `level`, `pressed`, and `selected`; omitted state options impose no constraint. `includeHidden` defaults to false and controls accessibility exclusion, while `filter({ visible })` checks rendered visibility. Mixed checkboxes do not match either boolean checked value.

`filter` supports `has`, `hasNot`, `hasText`, `hasNotText`, and `visible`, in any combination and across repeated filters. Inner locators evaluate relative to each candidate and must use the same page. `first()`, `last()`, and zero-based `nth(index)` select from the current result set; negative indices count from the end. TypeScript also accepts filter options in `locator(selector, options)`.

Language conventions:

- TypeScript/Vitest: camelCase builders, `RoleOptions`, `TextOptions`, `LocatorFilterOptions`, native `RegExp`.
- Python: snake_case builders and keyword options (`include_hidden`, `has_not`, etc.), `re.Pattern`; `.first` and `.last` are properties.
- Go: `GetByRole`, `GetByText`, etc.; optional `RoleOptions` / `TextOptions`, `LocatorFilterOptions`, and `TextPattern{Regex, Flags}`. Pointer boolean options preserve omitted versus false. Text arguments accept a string or `TextPattern`.
- Rust: snake_case builders and `_with_options` variants, `RoleOptions`, `TextOptions`, `LocatorFilterOptions`; strings convert into `TextMatcher`, or use `TextMatcher::Regex { regex, flags }`.
- Java: camelCase builders, `new RoleOptions().setName(...).setChecked(false)`, `new TextOptions(true)`, `new LocatorFilterOptions().setHas(...)`, and `Pattern`.

Regex patterns execute as ECMAScript regexes in the browser. Use compatible syntax across languages. Python and Java map case-insensitive, multiline, and dotall flags; unsupported flags fail locally.

The web plugin owns the resolver, uses Allwright's accessibility helpers, and evaluates through WebDriver BiDi on Chromium and Firefox. Clients send JSON-quoted `aw=` segments through the existing selector transport. CSS and semantic queries traverse open shadow roots; XPath retains native DOM scoping. Duplicate nodes from overlapping scopes are removed.

These are Playwright-style APIs, not a full Playwright implementation. They retain Allwright's existing action/retry behavior and accessibility limits documented in [ACCESSIBILITY.md](ACCESSIBILITY.md). Native accessibility trees, closed shadow roots, frame-locator APIs, configurable test-ID attributes, and locator `and`/`or` are outside this implementation. Test IDs use `data-testid`. Selector chains remain within the current browsing context.

Behavioral reference: [Playwright locator API](https://playwright.dev/docs/api/class-locator). No Playwright implementation or accessibility dependency is bundled.

## Excluding matching elements

`locator.not(otherLocator)` removes nodes also matched by `otherLocator`, preserving
the candidate order and allowing further chaining. This differs from
`filter({ hasNot: otherLocator })`, which checks descendants. Both locators must
belong to the same page. Exclusion uses the surrounding query's root scope, including
when composed inside a relative `has`/`hasNot` query.

```ts
const actions = page.getByRole('button').not(page.getByRole('button', { disabled: true }));
await actions.first().click();
```

Go uses `.Not(other)`, Rust `.not(&other)`, Python `.not_(other)` (`not` is a keyword),
and Java/TypeScript `.not(other)`. The transport adds an `exclude` semantic segment;
the web plugin evaluates the other locator and subtracts matching element identities.
