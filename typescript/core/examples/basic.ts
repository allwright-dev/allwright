import { firefox } from "../dist/index.js";

async function main(): Promise<void> {
  const browser = await firefox.launch({
    browserBinary: process.env.ALLWRIGHT_BROWSER_BINARY,
  });

  try {
    const page = browser.page();
    await page.goto("https://example.com");
    const heading = await page.textContent("h1");
    console.log(`[core-example] h1=${JSON.stringify(heading.text)}`);
  } finally {
    await browser.close();
  }
}

main().catch((error: unknown) => {
  console.error(error);
  process.exitCode = 1;
});
