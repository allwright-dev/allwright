# allwright CLI

Installable `allwright` package that ships the command-line interface plus the lightweight `allwright-core` engine dependency.

Run `allwright update` to replace the current executable with the latest platform release, or
`allwright update --version vX.Y.Z` to install a pinned release. Downloads are verified by running
the staged binary's `--version` command before replacement.
Restart any already-running `allwright serve` process after the update.
