# Accessibility implementation

The JavaScript in `src/accessibility_semantics.js` and `src/accessibility.js` is
Allwright-owned source. It is embedded directly into the web plugin and evaluated
through BiDi. There is no accessibility package to download, code-generation step,
or dependency on a Playwright runtime.

## References

Implementation decisions are informed by the specifications and by inspecting
Playwright's behavior and source; Playwright source is not copied or bundled.

- [ARIA in HTML](https://www.w3.org/TR/html-aria/): native roles, contextual landmarks, and presentation semantics.
- [Accessible Name and Description Computation](https://www.w3.org/TR/accname-1.2/): reference traversal, text alternatives, and precedence. Version 1.2 is a working draft.
- [WAI-ARIA 1.2](https://www.w3.org/TR/wai-aria-1.2/): role tokens, name restrictions, states, and properties.
- [Playwright accessibility helpers](https://github.com/microsoft/playwright/blob/v1.63.0/packages/injected/src/roleUtils.ts): a behavioral reference for browser edge cases.

## Implemented behavior

- Recognized explicit-role fallback, native control roles, named form/section
  landmarks, contextual header/footer landmarks, grid cells, and explicit row headers.
- Presentation inheritance for list/table structural children, with focusability
  and global-ARIA conflict handling.
- Ordered, deduplicated ID references; accessible-name precedence through
  `aria-labelledby`, `aria-label`, native labels/alternatives, permitted content,
  and title/placeholder fallbacks. Reference cycles terminate.
- Hidden referenced labels may contribute text while hidden ordinary descendants
  do not. Inline text stays contiguous; block elements and line breaks separate it.
- Multiple native labels, fieldset legends, table captions, figure captions,
  SVG titles/descriptions, image alternatives, and embedded control values.
- Accessible descriptions use `aria-describedby` before `aria-description`, then
  supported native descriptions and titles. Empty referenced descriptions do not
  fall back to lower-priority text.
- Composed-DOM traversal, open shadow roots, slots, inert/hidden subtrees, closed
  details, off-screen content, and ARIA ownership cycle/duplicate protection.
- Quoted CSS-generated text, including computed string escapes. Image/counter
  content is not converted into guessed text.
- Typed native/ARIA state collection, including mixed checkbox state and rejection
  of mixed radio/switch state. See the snapshot README for the output contract.

## Maintenance and limits

Changes should add local fixture cases to `tests/fixtures/accessibility.html` and
assertions to `tests/accessibility.rs`, then pass in Chromium and Firefox. Test
fixtures are authored for Allwright. JSON and YAML must continue to decode to the
same document structure.

This implementation is new. Passing these targeted regressions does not establish
full W3C conformance or Playwright-equivalent coverage. It does not inherit
Playwright's test history, maintenance, or support merely by referencing its
behavior. Allwright owns maintenance of this code.

Closed shadow roots, browser-internal widgets, canvas internals, and
ElementInternals-only accessibility information are unavailable. Complete CSS
content grammar, every HTML/SVG naming rule, every presentation-inheritance case,
all ARIA extension roles, and browser-specific native AX differences are not
claimed to be covered. The output is a DOM-derived representation, not a native
platform accessibility tree. Frame sampling and visibility limits are documented
in the [snapshot README](README.md#accessibility-snapshots).
