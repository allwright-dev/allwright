from __future__ import annotations

import os

from allwright import (
    DesktopWindowsConnectOptions,
    DesktopWindowsLaunchOptions,
    desktop,
    set_server_addr,
    shutdown,
)


def main() -> None:
    set_server_addr(os.getenv("ALLWRIGHT_SERVER_ADDR", "127.0.0.1:50051"))
    session = desktop.windows.connect(
        DesktopWindowsConnectOptions(
            agent_endpoint=os.getenv("ALLWRIGHT_WINDOWS_AGENT_ENDPOINT"),
            timeout_ms=30_000,
        )
    )

    try:
        app = session.launch(
            DesktopWindowsLaunchOptions(
                app_id=os.getenv("ALLWRIGHT_WINDOWS_APP_ID", "notepad.exe"),
                timeout_ms=60_000,
            )
        )
        screenshot = app.screenshot()
        if not screenshot:
            raise AssertionError("Windows screenshot should not be empty")
        print(f"[py-windows-basic] captured {len(screenshot)} bytes")
    finally:
        shutdown()


if __name__ == "__main__":
    main()
