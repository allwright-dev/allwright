import { ping, resolveConfig, setServerAddr, shutdown } from "@allwright.dev/core";

export async function setup(): Promise<void> {
  const config = resolveConfig();
  if (config.serverAddr) {
    setServerAddr(config.serverAddr);
  }
  await ping();
}

export async function teardown(): Promise<void> {
  await shutdown();
}
