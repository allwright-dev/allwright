from __future__ import annotations

import os

from allwright import (
    MobileAndroidConnectOptions,
    MobileAndroidLaunchOptions,
    mobile,
    set_server_addr,
    shutdown,
)

DEFAULT_ANDROID_DEVICE = "emulator-5554"
DEFAULT_ANDROID_APP_ID = "com.example.airticket"
DEFAULT_ANDROID_TAP_SELECTOR = "Id=com.example.airticket:id/bottom_nav_account"
DEFAULT_ANDROID_FILL_SELECTOR = 'xpath=//*[@text="Email"]'
DEFAULT_ANDROID_FILL_VALUE = "user@example.com"


def launch_options_from_env() -> MobileAndroidLaunchOptions:
    apk_path = os.getenv("ALLWRIGHT_ANDROID_APK_PATH")
    app_id = os.getenv("ALLWRIGHT_ANDROID_APP_ID", DEFAULT_ANDROID_APP_ID)
    launch_activity = os.getenv("ALLWRIGHT_ANDROID_APP_ACTIVITY")
    return MobileAndroidLaunchOptions(
        apk_path=apk_path,
        app_id=app_id,
        launch_activity=launch_activity,
        timeout_ms=60_000,
    )


def main() -> None:
    set_server_addr(os.getenv("ALLWRIGHT_SERVER_ADDR", "127.0.0.1:50051"))
    device = mobile.android.connect(
        MobileAndroidConnectOptions(
            device=os.getenv("ALLWRIGHT_ANDROID_DEVICE", DEFAULT_ANDROID_DEVICE),
            adb_endpoint=os.getenv("ALLWRIGHT_ANDROID_ADB_ENDPOINT"),
            timeout_ms=30_000,
        )
    )

    app = device.launch(launch_options_from_env())

    try:
        app.click(
            os.getenv(
                "ALLWRIGHT_ANDROID_TAP_SELECTOR",
                DEFAULT_ANDROID_TAP_SELECTOR,
            )
        )
        app.fill(
            os.getenv("ALLWRIGHT_ANDROID_FILL_SELECTOR", DEFAULT_ANDROID_FILL_SELECTOR),
            os.getenv("ALLWRIGHT_ANDROID_FILL_VALUE", DEFAULT_ANDROID_FILL_VALUE),
        )
        print(f"[py-android-basic] app_session_id={app.session_id}")
    finally:
        shutdown()


if __name__ == "__main__":
    main()
