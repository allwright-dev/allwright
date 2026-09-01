import { fileURLToPath } from "node:url";

import type { UserConfig } from "vitest/config";

const allwrightGlobalSetup = fileURLToPath(new URL("./global-setup.js", import.meta.url));

export function allwrightVitestConfig<T extends UserConfig>(config: T): T {
  const configuredGlobalSetup = config.test?.globalSetup;
  const globalSetup = [
    allwrightGlobalSetup,
    ...(Array.isArray(configuredGlobalSetup)
      ? configuredGlobalSetup
      : configuredGlobalSetup
        ? [configuredGlobalSetup]
        : []),
  ];

  return {
    ...config,
    test: {
      ...config.test,
      globalSetup,
    },
  } as T;
}
