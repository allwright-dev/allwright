# allwright CLI

Installable `allwright` package that ships the command-line interface plus the lightweight `allwright-core` engine dependency.

Run `allwright update` to replace the current executable with the latest platform release, or
`allwright update --version vX.Y.Z` to install a pinned release. Downloads are verified by running
the staged binary's `--version` command before replacement.
Restart any already-running `allwright serve` process after the update.

Inspect a native Windows application with:

```powershell
allwright inspector desktop-windows "C:\Windows\System32\notepad.exe"
```

The inspector starts a loopback-only web application, opens it in the default
browser, and exposes the current screenshot, UIA3 hierarchy, element details,
suggested locators, and an element click action. A URL can be used as the target
to inspect the foreground browser window.
