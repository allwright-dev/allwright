from __future__ import annotations

import os

from allwright import (
    DesktopMacConnectOptions,
    DesktopMacLaunchOptions,
    desktop,
    set_server_addr,
    shutdown,
)


def main() -> None:
    set_server_addr(os.getenv("ALLWRIGHT_SERVER_ADDR", "127.0.0.1:50051"))
    session = desktop.mac.connect(
        DesktopMacConnectOptions(
            agent_endpoint=os.getenv("ALLWRIGHT_MAC_AGENT_ENDPOINT"),
            timeout_ms=30_000,
        )
    )

    try:
        app = session.launch(
            DesktopMacLaunchOptions(
                app_id=os.getenv("ALLWRIGHT_MAC_APP_ID", "com.apple.calculator"),
                timeout_ms=60_000,
            )
        )
        screenshot = app.screenshot()
        if not screenshot:
            raise AssertionError("macOS screenshot should not be empty")
        print(f"[py-macos-basic] captured {len(screenshot)} bytes")
    finally:
        shutdown()


if __name__ == "__main__":
    main()
