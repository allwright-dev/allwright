package dev.allwright.examples;

import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.junit.jupiter.api.Assumptions.assumeTrue;

import dev.allwright.client.Allwright;
import dev.allwright.client.DesktopMacConnectOptions;
import dev.allwright.client.DesktopMacLaunchOptions;
import dev.allwright.client.MacApp;
import dev.allwright.client.MacDesktop;
import org.junit.jupiter.api.Test;

final class MacosBasicTest {
    @Test
    void macosBasic() {
        assumeTrue(
                "true".equalsIgnoreCase(System.getenv("ALLWRIGHT_RUN_MACOS_EXAMPLE")),
                "set ALLWRIGHT_RUN_MACOS_EXAMPLE=true to run this local example"
        );
        Allwright.setServerAddr(System.getenv().getOrDefault("ALLWRIGHT_SERVER_ADDR", "127.0.0.1:50051"));

        try {
            MacDesktop desktop = Allwright.desktop().mac().connect(new DesktopMacConnectOptions(
                    System.getenv("ALLWRIGHT_MAC_AGENT_ENDPOINT"), 30_000));
            MacApp app = desktop.launch(new DesktopMacLaunchOptions(
                    System.getenv().getOrDefault("ALLWRIGHT_MAC_APP_ID", "com.apple.calculator"),
                    false,
                    60_000));

            assertTrue(app.screenshot().length > 0, "macOS screenshot should not be empty");
        } finally {
            Allwright.shutdown();
        }
    }
}
