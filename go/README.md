
Web state reads: `page.URL(ctx)`, `locator.InputValue(ctx)`, `locator.SelectedOptions(ctx)`, `locator.SelectedText(ctx)`, `locator.IsChecked(ctx)`, `locator.GetAttribute(ctx, name)`, and `locator.BoundingBox(ctx)`. Element reads also accept selectors on `Page`, plus optional `CommandOptions`. Selected options return `[]CapturedOption`; boxes return `*BoundingBox` in frame/page viewport CSS pixels. Missing attributes, unsupported text selections, and hidden/zero-area boxes return nil values with no error. Existing `TextContent` and `InnerText` read element text. See the [shared semantics](../typescript/core/README.md#read-page-and-element-state).

Runnable clients for every implemented surface are available under
`examples/web-basic`, `examples/android-basic`, `examples/ios-basic`,
`examples/macos-basic`, and `examples/windows-basic`. Run the iOS Flights
example with `go run ./examples/ios-basic`; it uses the hosted Simulator IPA by
default. The desktop examples default to Calculator on macOS and Notepad on
Windows and accept `ALLWRIGHT_MAC_APP_ID` or `ALLWRIGHT_WINDOWS_APP_ID`.
