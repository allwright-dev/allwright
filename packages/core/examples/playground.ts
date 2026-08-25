import readline from "node:readline/promises";
import { stdin as input, stdout as output } from "node:process";

import { chromium, setServerAddr, shutdown } from "../dist/index.js";

async function main(): Promise<void> {
  const args = parseArgs(process.argv.slice(2));
  setServerAddr(args.serverAddr);

  console.log(
    `[ts-playground] launching chrome with chromeBinary=${JSON.stringify(args.chromeBinary)} via singleton TypeScript client runtime`,
  );

  const browser = await chromium.launch({
    chromeBinary: args.chromeBinary ?? undefined,
  });
  const initialTab = browser.page();
  console.log(
    `[${browser.sessionId}] chrome launched: ${browser.browserName} (${browser.launchNote}) cdp=${browser.cdpWebSocketURL} user_data_dir=${browser.userDataDir} initial_tab_session_id=${initialTab.sessionId}`,
  );

  const initialNavigation = await initialTab.navigate(args.navigateUrl);
  console.log(
    `[${initialTab.sessionId}] tab navigated: ${initialNavigation.url} (${initialNavigation.note})`,
  );
  console.log(
    `[${initialTab.sessionId}] chromium-bidi injected: bidi_session_id=${initialNavigation.bidiSessionId} mapper_target_id=${initialNavigation.mapperTargetId} mapper_session_id=${initialNavigation.mapperSessionId} package_version=${initialNavigation.packageVersion}`,
  );

  if (args.clickSelector) {
    const click = await initialTab.click(args.clickSelector);
    console.log(
      `[${initialTab.sessionId}] element clicked: selector=${click.selector} (${click.note}) bidi_session_id=${click.bidiSessionId}`,
    );
  }

  const tabs = [initialTab];
  for (let index = 0; index < args.tabs; index += 1) {
    const tabNumber = index + 2;
    const tab = await browser.newPage();
    console.log(`[${browser.sessionId}] tab opened: ${tab.sessionId} (requested additional tab ${tabNumber})`);
    const navigation = await tab.navigate(args.navigateUrl);
    console.log(`[${tab.sessionId}] tab navigated: ${navigation.url} (${navigation.note})`);
    if (args.clickSelector) {
      const click = await tab.click(args.clickSelector);
      console.log(
        `[${tab.sessionId}] element clicked: selector=${click.selector} (${click.note}) bidi_session_id=${click.bidiSessionId}`,
      );
    }
    tabs.push(tab);
  }

  const rl = readline.createInterface({ input, output });
  await rl.question("[ts-playground] Press Enter to close the tabs and browser session...");
  rl.close();

  for (const tab of tabs.slice(1)) {
    await tab.close();
    console.log(`[${tab.sessionId}] tab session closed`);
  }
  await initialTab.close();
  console.log(`[${initialTab.sessionId}] tab session closed`);

  await browser.close();
  console.log(`[${browser.sessionId}] session closed`);
}

interface ParsedArgs {
  serverAddr: string;
  chromeBinary: string | null;
  navigateUrl: string;
  clickSelector: string | null;
  tabs: number;
}

function parseArgs(argv: string[]): ParsedArgs {
  let serverAddr = "127.0.0.1:50051";
  let chromeBinary: string | null = null;
  let navigateUrl = "https://example.com";
  let clickSelector: string | null = null;
  let tabs = 3;

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    const next = argv[index + 1];
    if (arg === "--server-addr" && next) {
      serverAddr = next;
      index += 1;
      continue;
    }
    if (arg === "--chrome-binary" && next) {
      chromeBinary = next;
      index += 1;
      continue;
    }
    if (arg === "--navigate-url" && next) {
      navigateUrl = next;
      index += 1;
      continue;
    }
    if (arg === "--click-selector" && next) {
      clickSelector = next;
      index += 1;
      continue;
    }
    if (arg === "--tabs" && next) {
      tabs = Number.parseInt(next, 10);
      index += 1;
      continue;
    }
  }

  return {
    serverAddr,
    chromeBinary,
    navigateUrl,
    clickSelector,
    tabs,
  };
}

main()
  .catch(async (error: unknown) => {
    console.error(error);
    process.exitCode = 1;
  })
  .finally(async () => {
    await shutdown();
  });
