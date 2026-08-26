import readline from "node:readline/promises";
import { stdin as input, stdout as output } from "node:process";

import { chromium, firefox, setServerAddr, shutdown, type BrowserKind } from "../dist/index.js";

async function main(): Promise<void> {
  const args = parseArgs(process.argv.slice(2));
  setServerAddr(args.serverAddr);

  console.log(
    `[ts-playground] launching ${args.browser} with browserBinary=${JSON.stringify(args.browserBinary)} via singleton TypeScript client runtime`,
  );

  const browserType = args.browser === "firefox" ? firefox : chromium;
  const browser = await browserType.launch({
    browserBinary: args.browserBinary ?? undefined,
  });
  const initialTab = browser.page();
  logBrowserLaunch(browser, initialTab.sessionId);

  const initialNavigation = await initialTab.navigate(args.navigateUrl);
  console.log(
    `[${initialTab.sessionId}] tab navigated: ${initialNavigation.url} (${initialNavigation.note})`,
  );
  logNavigationAutomation(initialTab.sessionId, initialNavigation);

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
    logNavigationAutomation(tab.sessionId, navigation);
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
  browser: BrowserKind;
  browserBinary: string | null;
  navigateUrl: string;
  clickSelector: string | null;
  tabs: number;
}

function parseArgs(argv: string[]): ParsedArgs {
  let serverAddr = "127.0.0.1:50051";
  let browser: BrowserKind = "chromium";
  let browserBinary: string | null = null;
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
    if (arg === "--browser" && next && (next === "chromium" || next === "firefox")) {
      browser = next;
      index += 1;
      continue;
    }
    if (arg === "--browser-binary" && next) {
      browserBinary = next;
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
    browser,
    browserBinary,
    navigateUrl,
    clickSelector,
    tabs,
  };
}

function logBrowserLaunch(browser: { sessionId: string; browserName: string; launchNote: string; cdpWebSocketURL: string; userDataDir: string }, initialTabSessionId: string): void {
  const cdpPart = browser.cdpWebSocketURL ? ` cdp=${browser.cdpWebSocketURL}` : "";
  console.log(
    `[${browser.sessionId}] browser launched: ${browser.browserName} (${browser.launchNote})${cdpPart} user_data_dir=${browser.userDataDir} initial_tab_session_id=${initialTabSessionId}`,
  );
}

function logNavigationAutomation(
  tabSessionId: string,
  navigation: { bidiSessionId: string; mapperTargetId: string; mapperSessionId: string; packageVersion: string },
): void {
  console.log(
    `[${tabSessionId}] automation session: bidi_session_id=${navigation.bidiSessionId} mapper_target_id=${navigation.mapperTargetId} mapper_session_id=${navigation.mapperSessionId} package_version=${navigation.packageVersion}`,
  );
}

main()
  .catch(async (error: unknown) => {
    console.error(error);
    process.exitCode = 1;
  })
  .finally(async () => {
    await shutdown();
  });
