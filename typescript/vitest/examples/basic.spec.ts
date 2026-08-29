import { expect, test } from "../dist/index.js";

const WEB_URL = "https://themoderninternet.vercel.app";
const ENTRY_SELECTOR =
  "xpath=//div[contains(@class,'card')][.//h2[normalize-space()='Form Inputs']]//button[normalize-space()='Visit page']";
const HEADING_SELECTOR = 'xpath=//h1[text()="Form Inputs"]';

test.use({
  allwright: {
    suite: "firefox-dev",
  },
});

test("opens the Form Inputs page", async ({ page }) => {
  await page.goto(WEB_URL);
  await page.click(ENTRY_SELECTOR);
  await expect(page.locator(HEADING_SELECTOR)).toHaveText("Form Inputs");
});
