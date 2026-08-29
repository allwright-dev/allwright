package dev.allwright.examples;

import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.junit.jupiter.api.Assumptions.assumeTrue;

import dev.allwright.client.Allwright;
import dev.allwright.client.Browser;
import dev.allwright.client.Page;
import dev.allwright.client.TextResult;
import dev.allwright.client.WaitForSelectorOptions;
import org.junit.jupiter.api.Test;

final class WebBasicTest {
    private static final String DEFAULT_WEB_URL = "https://themoderninternet.vercel.app";
    private static final String DEFAULT_WEB_ENTRY_SELECTOR =
            "xpath=//div[contains(@class,'card')][.//h2[normalize-space()='Form Inputs']]//button[normalize-space()='Visit page']";
    private static final String DEFAULT_WEB_HEADING_SELECTOR = "xpath=//h1[text()=\"Form Inputs\"]";
    private static final String DEFAULT_WEB_HEADING_TEXT = "Form Inputs";

    @Test
    void webBasic() {
        assumeTrue(
                "true".equalsIgnoreCase(System.getenv("ALLWRIGHT_RUN_WEB_EXAMPLE")),
                "set ALLWRIGHT_RUN_WEB_EXAMPLE=true to run this local example"
        );

        Allwright.setServerAddr(System.getenv().getOrDefault("ALLWRIGHT_SERVER_ADDR", "127.0.0.1:50051"));

        try (Browser browser = Allwright.firefox().launch()) {
            Page page = browser.page();
            page.goTo(System.getenv().getOrDefault("ALLWRIGHT_WEB_URL", DEFAULT_WEB_URL));
            page.click(System.getenv().getOrDefault("ALLWRIGHT_WEB_ENTRY_SELECTOR", DEFAULT_WEB_ENTRY_SELECTOR));
            page.waitForSelector(
                    System.getenv().getOrDefault("ALLWRIGHT_WEB_HEADING_SELECTOR", DEFAULT_WEB_HEADING_SELECTOR),
                    new WaitForSelectorOptions(10_000, true)
            );
            TextResult heading = page.textContent(
                    System.getenv().getOrDefault("ALLWRIGHT_WEB_HEADING_SELECTOR", DEFAULT_WEB_HEADING_SELECTOR)
            );
            assertTrue(
                    heading.text().contains(
                            System.getenv().getOrDefault("ALLWRIGHT_WEB_HEADING_TEXT", DEFAULT_WEB_HEADING_TEXT)
                    )
            );
        } finally {
            Allwright.shutdown();
        }
    }
}
