from __future__ import annotations

import os

from allwright import (
    MobileIosConnectOptions,
    MobileIosLaunchOptions,
    mobile,
    set_server_addr,
    shutdown,
)

DEFAULT_IOS_APP_PATH = "https://allwright.dev/Flights-simulator.ipa"
EXPECTED_ALERT = "No account found. Please sign up first."


def main() -> None:
    set_server_addr(os.getenv("ALLWRIGHT_SERVER_ADDR", "127.0.0.1:50051"))
    device = mobile.ios.connect(
        MobileIosConnectOptions(
            device=os.getenv("ALLWRIGHT_IOS_DEVICE"),
            agent_endpoint=os.getenv("ALLWRIGHT_IOS_AGENT_ENDPOINT"),
            timeout_ms=60_000,
        )
    )
    app = device.launch(
        MobileIosLaunchOptions(
            app_path=os.getenv("ALLWRIGHT_IOS_APP_PATH", DEFAULT_IOS_APP_PATH),
            app_id=os.getenv("ALLWRIGHT_IOS_APP_ID"),
            stop_before_launch=True,
            timeout_ms=120_000,
        )
    )

    try:
        app.get_by_role("button", name="Login", exact=True).click()
        app.get_by_label("Email", exact=True).fill("allwright@example.com")
        app.get_by_label("Password", exact=True).fill("not-a-real-password")
        app.get_by_role("button", name="Submit", exact=True).click()

        alert = app.get_by_text(EXPECTED_ALERT, exact=True).text_content()
        if alert != EXPECTED_ALERT:
            raise AssertionError(f"unexpected alert: {alert!r}")
        print(f"[py-ios-basic] {alert}")
    finally:
        shutdown()


if __name__ == "__main__":
    main()
