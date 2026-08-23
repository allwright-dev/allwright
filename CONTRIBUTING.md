# Contributing to allwright

Thank you for your interest in contributing to allwright.

## Getting Started

1. Fork the repository and create a branch for your change.
2. Make focused changes with clear commit messages.
3. Run the relevant tests or checks for the parts you changed.
4. Open a pull request with a short description of the change and any validation you ran.

## Development Guidelines

- Keep changes scoped to the problem you are solving.
- Update documentation when behavior, architecture, or developer workflow changes.
- Keep `README.md` and `Codex.md` aligned when either one needs to reflect a project-level change.
- Follow the current repo ownership boundaries:
  - `rust/engine-lib` owns engine behavior and the gRPC server surface.
  - platform crates are support layers and should not take ownership of engine logic.
  - shared contracts belong under `proto/`.
- Preserve the project's driverless browser automation direction and avoid introducing ChromeDriver into the primary control path.

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
