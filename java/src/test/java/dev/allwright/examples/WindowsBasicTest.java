package dev.allwright.examples;

import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.junit.jupiter.api.Assumptions.assumeTrue;

import dev.allwright.client.Allwright;
import dev.allwright.client.DesktopWindowsConnectOptions;
import dev.allwright.client.DesktopWindowsLaunchOptions;
import dev.allwright.client.WindowsApp;
import dev.allwright.client.WindowsDesktop;
import org.junit.jupiter.api.Test;

final class WindowsBasicTest {
    @Test
    void windowsBasic() {
        assumeTrue(
                "true".equalsIgnoreCase(System.getenv("ALLWRIGHT_RUN_WINDOWS_EXAMPLE")),
                "set ALLWRIGHT_RUN_WINDOWS_EXAMPLE=true to run this local example"
        );
        Allwright.setServerAddr(System.getenv().getOrDefault("ALLWRIGHT_SERVER_ADDR", "127.0.0.1:50051"));

        try {
            WindowsDesktop desktop = Allwright.desktop().windows().connect(new DesktopWindowsConnectOptions(
                    System.getenv("ALLWRIGHT_WINDOWS_AGENT_ENDPOINT"), 30_000));
            WindowsApp app = desktop.launch(new DesktopWindowsLaunchOptions(
                    System.getenv().getOrDefault("ALLWRIGHT_WINDOWS_APP_ID", "notepad.exe"),
                    false,
                    60_000));

            assertTrue(app.screenshot().length > 0, "Windows screenshot should not be empty");
        } finally {
            Allwright.shutdown();
        }
    }
}
