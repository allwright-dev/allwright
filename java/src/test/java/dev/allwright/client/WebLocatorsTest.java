package dev.allwright.client;
import org.junit.jupiter.api.Test;
import java.util.regex.Pattern;
import static org.junit.jupiter.api.Assertions.*;

class WebLocatorsTest {
    @Test void quotedChainsAndFiltersSurviveNormalization() {
        Page page = new Page(null, "browser", "page");
        Locator inner = page.getByRole("button", new RoleOptions().setName(Pattern.compile("^Save \"now\"$", Pattern.CASE_INSENSITIVE)).setPressed(false));
        Locator locator = page.locator("article").filter(new LocatorFilterOptions().setHas(inner).setVisible(false).setHasNotText("xpath=\"trap\"")).getByTestId("a\"b").last();
        assertEquals(locator.selector(), SelectorSupport.normalizeSelectorForTransport(locator.selector()));
        assertTrue(inner.selector().contains("pressed\\\":false"));
        assertTrue(locator.selector().startsWith("css=\"article\" aw="));
    }
    @Test void invalidScopeAndLevelFailLocally() {
        Page page = new Page(null, "browser", "one"), other = new Page(null, "browser", "two");
        assertThrows(IllegalArgumentException.class, () -> page.locator("li").filter(new LocatorFilterOptions().setHas(other.getByText("other"))));
        assertThrows(IllegalArgumentException.class, () -> page.getByRole("heading", new RoleOptions().setLevel(0)));
    }
    @Test void exclusionIsImmutableAndRejectsOtherPages() {
        Page page = new Page(null, "browser", "one"), other = new Page(null, "browser", "two");
        Locator buttons = page.getByRole("button");
        Locator remaining = buttons.not(page.getByText("Cancel")).first();
        assertTrue(remaining.selector().contains("exclude"));
        assertFalse(buttons.selector().contains("exclude"));
        assertEquals(remaining.selector(), SelectorSupport.normalizeSelectorForTransport(remaining.selector()));
        assertThrows(IllegalArgumentException.class, () -> buttons.not(other.getByText("Cancel")));
    }
}
