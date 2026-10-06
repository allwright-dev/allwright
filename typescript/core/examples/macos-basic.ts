import { desktop, shutdown } from "../dist/index.js";

async function main(): Promise<void> {
  const session = await desktop.mac.connect({
    agentEndpoint: process.env.ALLWRIGHT_MAC_AGENT_ENDPOINT,
    timeoutMs: 30_000,
  });

  try {
    const app = await session.launch({
      appId: process.env.ALLWRIGHT_MAC_APP_ID ?? "com.apple.calculator",
      timeoutMs: 60_000,
    });
    const screenshot = await app.screenshot();
    if (screenshot.byteLength === 0) {
      throw new Error("macOS screenshot should not be empty");
    }
    console.log(`[ts-macos-basic] captured ${screenshot.byteLength} bytes`);
  } finally {
    await shutdown();
  }
}

main().catch((error: unknown) => {
  console.error(error);
  process.exitCode = 1;
});
