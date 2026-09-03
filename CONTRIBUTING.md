# Contributing to allwright

Thank you for your interest in contributing to allwright.

## Getting Started

1. Fork the repository and create a branch for your change.
2. Make focused changes with clear commit messages.
3. Run the relevant tests or checks for the parts you changed.
4. Open a pull request with a short description of the change and any validation you ran.

## Development Guidelines

- For local setup, scripts, and day-to-day repo workflow details, see [DEVELOPMENT.md](DEVELOPMENT.md).
- Keep changes scoped to the problem you are solving.
- Update documentation when behavior, architecture, or developer workflow changes.
- Keep `README.md` and `Codex.md` aligned when either one needs to reflect a project-level change.
- Follow the current repo ownership boundaries:
  - `rust/allwright` owns the lightweight engine behavior and the gRPC server surface.
  - `rust/allwright-cli` owns the installable `allwright` command-line package and plugin-manifest workflow.
  - `rust/allwright-plugin-sdk` owns shared plugin traits and metadata.
  - surface-specific code belongs in the publishable surface crates, not inline inside `rust/allwright`.
  - shared contracts belong under `proto/`.
  - future surface capabilities should extend the single engine through explicit plugin boundaries such as `web`, `mobile-android`, `mobile-ios`, `desktop-mac`, `desktop-windows`, and `desktop-linux` rather than introducing separate engines.
- Preserve the project's driverless browser automation direction and avoid introducing ChromeDriver into the primary control path.
- Hard rule for the web plugin: all Chromium web element operations, including click, hover, focus, fill, key input, selector checks, text reads, highlighting, and screenshots, must execute through WebDriver BiDi. CDP may only support browser/tab lifecycle, Chromium BiDi mapper bootstrap, and mapper transport; it must not perform DOM inspection or user-input actions in a normal web automation path.

## Testing

Run the checks that match the stack you changed. Examples may include:

- Rust: `cargo test`
- Go: `go test ./...`
- Java: `./gradlew test`
- Python: run the relevant test command for the package
- TypeScript or `allwright-dev`: use the project's package scripts with Bun or npm as documented in the local package

If you cannot run a relevant check, mention that clearly in your pull request.

## Pull Requests

Please include:

- a concise summary of the change
- any related issue or context
- notes about tradeoffs or follow-up work
- the tests or verification steps you ran

## Code of Conduct

By participating in this project, you agree to engage respectfully and constructively with other contributors.
