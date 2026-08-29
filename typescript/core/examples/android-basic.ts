import { mobile, shutdown } from "../dist/index.js";

function launchOptionsFromEnv() {
  const apkPath = "https://allwright.dev/Flights-debug.apk";
  const appId ="com.example.airticket"
  return {
    apkPath,
    appId,
  };
}

async function main(): Promise<void> {
  const device = await mobile.android.connect({});
  const app = await device.launch(launchOptionsFromEnv());
  try {
    await app.click('text=Account');
    await app.click('text=Login');
    await app.click('text=Sign Up')
  } finally {
    await shutdown();
  }
}

main().catch((error: unknown) => {
  console.error(error);
  process.exitCode = 1;
});
