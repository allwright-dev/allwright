import { mobile, shutdown } from "../dist/index.js";

async function main(): Promise<void> {
  const appPath = process.env.ALLWRIGHT_IOS_APP_PATH ?? "https://allwright.dev/Flights-simulator.ipa";
  const device = await mobile.ios.connect({
    agentEndpoint: process.env.ALLWRIGHT_IOS_AGENT_ENDPOINT ?? "http://127.0.0.1:8100",
  });
  const app = await device.launch({
    appPath,
    appId: process.env.ALLWRIGHT_IOS_APP_ID,
    stopBeforeLaunch: true,
  });

  try {
    await app.getByRole("button", { name: "Login", exact: true }).click();
    await app.getByLabel("Email", { exact: true }).fill("allwright@example.com");
    await app.getByLabel("Password", { exact: true }).fill("not-a-real-password");
    await app.getByRole("button", { name: "Submit", exact: true }).click();

    const alert = await app
      .getByText("No account found. Please sign up first.", { exact: true })
      .textContent();
    console.log(alert.text);
  } finally {
    await shutdown();
  }
}

main().catch((error: unknown) => {
  console.error(error);
  process.exitCode = 1;
});
