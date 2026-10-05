package dev.allwright.examples;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assumptions.assumeTrue;

import dev.allwright.client.Allwright;
import dev.allwright.client.NativeApp;
import dev.allwright.client.IosDevice;
import dev.allwright.client.MobileIosConnectOptions;
import dev.allwright.client.MobileIosLaunchOptions;
import dev.allwright.client.RoleOptions;
import dev.allwright.client.TextOptions;
import org.junit.jupiter.api.Test;

final class IosBasicTest {
    private static final String DEFAULT_IOS_APP_PATH = "https://allwright.dev/Flights-simulator.ipa";
    private static final String EXPECTED_ALERT = "No account found. Please sign up first.";

    @Test
    void iosBasic() {
        assumeTrue(
                "true".equalsIgnoreCase(System.getenv("ALLWRIGHT_RUN_IOS_EXAMPLE")),
                "set ALLWRIGHT_RUN_IOS_EXAMPLE=true to run this local example"
        );

        Allwright.setServerAddr(System.getenv().getOrDefault("ALLWRIGHT_SERVER_ADDR", "127.0.0.1:50051"));

        try {
            IosDevice device = Allwright.mobile().ios().connect(
                    new MobileIosConnectOptions(
                            System.getenv("ALLWRIGHT_IOS_DEVICE"),
                            System.getenv("ALLWRIGHT_IOS_AGENT_ENDPOINT"),
                            false,
                            60_000
                    )
            );
            NativeApp app = device.launch(
                    new MobileIosLaunchOptions(
                            System.getenv().getOrDefault("ALLWRIGHT_IOS_APP_PATH", DEFAULT_IOS_APP_PATH),
                            System.getenv("ALLWRIGHT_IOS_APP_ID"),
                            true,
                            120_000
                    )
            );

            app.getByRole("button", new RoleOptions().setName("Login").setExact(true)).click();
            app.getByLabel("Email", new TextOptions(true)).fill("allwright@example.com");
            app.getByLabel("Password", new TextOptions(true)).fill("not-a-real-password");
            app.getByRole("button", new RoleOptions().setName("Submit").setExact(true)).click();

            String alert = app.getByText(EXPECTED_ALERT, new TextOptions(true)).textContent();
            assertEquals(EXPECTED_ALERT, alert);
        } finally {
            Allwright.shutdown();
        }
    }
}
