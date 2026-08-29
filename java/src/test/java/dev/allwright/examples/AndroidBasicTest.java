package dev.allwright.examples;

import static org.junit.jupiter.api.Assumptions.assumeTrue;

import dev.allwright.client.Allwright;
import dev.allwright.client.AndroidApp;
import dev.allwright.client.AndroidDevice;
import dev.allwright.client.MobileAndroidConnectOptions;
import dev.allwright.client.MobileAndroidLaunchOptions;
import org.junit.jupiter.api.Test;

final class AndroidBasicTest {
    private static final String DEFAULT_ANDROID_DEVICE = "emulator-5554";
    private static final String DEFAULT_ANDROID_APP_ID = "com.example.airticket";
    private static final String DEFAULT_ANDROID_TAP_SELECTOR =
            "Id=com.example.airticket:id/bottom_nav_account";
    private static final String DEFAULT_ANDROID_FILL_SELECTOR = "xpath=//*[@text=\"Email\"]";
    private static final String DEFAULT_ANDROID_FILL_VALUE = "user@example.com";

    @Test
    void androidBasic() {
        assumeTrue(
                "true".equalsIgnoreCase(System.getenv("ALLWRIGHT_RUN_ANDROID_EXAMPLE")),
                "set ALLWRIGHT_RUN_ANDROID_EXAMPLE=true to run this local example"
        );

        String appId = System.getenv().getOrDefault("ALLWRIGHT_ANDROID_APP_ID", DEFAULT_ANDROID_APP_ID);
        String apkPath = System.getenv("ALLWRIGHT_ANDROID_APK_PATH");

        Allwright.setServerAddr(System.getenv().getOrDefault("ALLWRIGHT_SERVER_ADDR", "127.0.0.1:50051"));

        try {
            AndroidDevice device = Allwright.mobile().android().connect(
                    new MobileAndroidConnectOptions(
                            System.getenv().getOrDefault("ALLWRIGHT_ANDROID_DEVICE", DEFAULT_ANDROID_DEVICE),
                            System.getenv("ALLWRIGHT_ANDROID_ADB_ENDPOINT"),
                            false,
                            30_000
                    )
            );

            AndroidApp app = device.launch(
                    new MobileAndroidLaunchOptions(
                            apkPath,
                            appId,
                            System.getenv("ALLWRIGHT_ANDROID_APP_ACTIVITY"),
                            false,
                            60_000
                    )
            );

            app.click(System.getenv().getOrDefault(
                    "ALLWRIGHT_ANDROID_TAP_SELECTOR",
                    DEFAULT_ANDROID_TAP_SELECTOR
            ));
            app.fill(
                    System.getenv().getOrDefault(
                            "ALLWRIGHT_ANDROID_FILL_SELECTOR",
                            DEFAULT_ANDROID_FILL_SELECTOR
                    ),
                    System.getenv().getOrDefault("ALLWRIGHT_ANDROID_FILL_VALUE", DEFAULT_ANDROID_FILL_VALUE)
            );
        } finally {
            Allwright.shutdown();
        }
    }
}
