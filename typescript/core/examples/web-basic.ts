import { chromium, shutdown } from "../dist/index.js";
async function main(): Promise<void> {
  const browser = await chromium.launch();
  try {
    const page = browser.page();
    await page.goto("https://themoderninternet.vercel.app");
    await page.click("//*[@data-slot='card' and .//*[text()='Form Inputs']]//button");
    await page.waitForSelector("//h1[text()='Form Inputs']",{
      visible:true
    });
  } finally {
    await browser.close().catch(() => undefined);
    await shutdown();
  }
}

main().catch((error: unknown) => {
  console.error(error);
  process.exitCode = 1;
});
