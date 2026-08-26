package dev.allwright.client;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

class AllwrightTest {
    @TempDir
    Path tempDir;

    @Test
    void findConfigFileSearchesParentDirectories() throws IOException {
        Path configFile = tempDir.resolve("allwright.config.yaml");
        Files.writeString(configFile, "schemaVersion: 1\n");

        Path nestedDir = Files.createDirectories(tempDir.resolve("apps/java/smoke"));

        assertEquals(configFile, Allwright.findConfigFile(nestedDir));
    }

    @Test
    void findConfigFileReturnsNullWhenMissing() throws IOException {
        Path nestedDir = Files.createDirectories(tempDir.resolve("apps/java/missing"));

        assertNull(Allwright.findConfigFile(nestedDir));
    }

    @Test
    void resolveConfigMergesTopLevelAndSuiteOverrides() throws IOException {
        Path configFile = tempDir.resolve("allwright.config.yaml");
        Files.writeString(
                configFile,
                """
                schemaVersion: 1
                server:
                  addr: 127.0.0.1:6000
                browser:
                  name: chromium
                  binary: /opt/chrome
                  launchOptions:
                    timeoutMs: 1200
                expect:
                  timeoutMs: 3000
                  intervalMs: 250
                suites:
                  firefox-smoke:
                    browser:
                      name: firefox
                      binary: /opt/firefox
                      launchOptions:
                        timeoutMs: 2400
                    expect:
                      intervalMs: 100
                """
        );

        Allwright.ResolvedConfig resolved = Allwright.resolveConfig(
                new Allwright.ResolveConfigOptions(tempDir, configFile, "firefox-smoke")
        );

        assertEquals(configFile, resolved.configFilePath());
        assertEquals("firefox-smoke", resolved.suiteName());
        assertEquals("127.0.0.1:6000", resolved.serverAddr());
        assertEquals("firefox", resolved.browserName());
        assertEquals("/opt/firefox", resolved.browserBinary());
        assertEquals("/opt/firefox", resolved.launchOptions().browserBinary());
        assertEquals(2400, resolved.launchOptions().timeoutMs());
        assertNotNull(resolved.expect());
        assertEquals(3000, resolved.expect().timeoutMs());
        assertEquals(100, resolved.expect().intervalMs());
    }

    @Test
    void resolveConfigRejectsUnknownSuite() throws IOException {
        Path configFile = tempDir.resolve("allwright.config.yaml");
        Files.writeString(configFile, "schemaVersion: 1\n");

        Allwright.AllwrightException error = assertThrows(
                Allwright.AllwrightException.class,
                () -> Allwright.resolveConfig(new Allwright.ResolveConfigOptions(tempDir, configFile, "missing"))
        );

        assertEquals(
                "allwright config suite \"missing\" was not found in " + configFile,
                error.getMessage()
        );
    }
}
