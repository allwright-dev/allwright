#!/usr/bin/env node

import { spawn } from "node:child_process";
import { access, mkdir, writeFile } from "node:fs/promises";
import { createRequire } from "node:module";
import path from "node:path";
import process from "node:process";
import * as p from "@clack/prompts";

const VITEST_VERSION = "^3.2.4";
const TYPESCRIPT_VERSION = "^5.9.2";
const PACKAGE_VERSION = getPackageVersion();
const ALLWRIGHT_VERSION = `^${PACKAGE_VERSION}`;

type Language = "ts" | "js";
type SurfaceId = "web" | "mobile-android" | "mobile-ios" | "desktop-mac" | "desktop-windows";
type PackageManager = "bun" | "npm" | "pnpm" | "yarn";

const SURFACE_OPTIONS: Array<{ label: string; value: SurfaceId }> = [
  { label: "Web", value: "web" },
  { label: "Mobile Android", value: "mobile-android" },
  { label: "Mobile iOS", value: "mobile-ios" },
  { label: "Desktop macOS", value: "desktop-mac" },
  { label: "Desktop Windows", value: "desktop-windows" },
];

interface InitOptions {
  targetDir: string;
  language?: Language;
  surfaces: SurfaceId[];
  yes: boolean;
  force: boolean;
  install: boolean;
  packageManager?: PackageManager;
}

async function main(): Promise<void> {
  const options = parseArgs(process.argv.slice(2));
  const interactive = process.stdin.isTTY && process.stdout.isTTY;
  const language: Language =
    options.language ??
    (options.yes
      ? "ts"
      : await selectLanguage(interactive));

  const surfaces =
    options.surfaces.length > 0
      ? dedupeSurfaces(options.surfaces)
      : options.yes
        ? (["web"] satisfies SurfaceId[])
        : await selectSurfaces(interactive);

  const targetDir = path.resolve(process.cwd(), options.targetDir);
  const relativeTargetDir = path.relative(process.cwd(), targetDir) || ".";
  const files = buildProjectFiles({
    language,
    surfaces,
    packageName: inferPackageName(path.basename(targetDir) || "allwright-app"),
  });

  await ensureTargetDirectory(targetDir);
  const conflicts = await findConflicts(targetDir, Object.keys(files));
  if (conflicts.length > 0 && !options.force) {
    const proceed = options.yes || (await confirmOverwrite(interactive, relativeTargetDir, conflicts));
    if (!proceed) {
      throw new Error("Initialization cancelled.");
    }
  }

  await writeProjectFiles(targetDir, files);
  const packageManager = options.packageManager ?? (await detectPackageManager(targetDir));
  if (options.install) {
    await installDependencies(targetDir, packageManager);
  }
  printSummary(targetDir, relativeTargetDir, language, surfaces, packageManager, options.install);
}

function parseArgs(args: string[]): InitOptions {
  const options: InitOptions = {
    targetDir: ".",
    surfaces: [],
    yes: false,
    force: false,
    install: true,
  };

  for (let index = 0; index < args.length; index += 1) {
    const arg = args[index];
    switch (arg) {
      case "--yes":
      case "-y":
        options.yes = true;
        break;
      case "--force":
        options.force = true;
        break;
      case "--no-install":
        options.install = false;
        break;
      case "--package-manager": {
        const value = args[index + 1];
        if (!value) {
          throw new Error("Expected a package manager after --package-manager.");
        }
        assertUnset(options.packageManager, "package manager");
        options.packageManager = parsePackageManager(value);
        index += 1;
        break;
      }
      case "--typescript":
      case "--ts":
        assertUnset(options.language, "language");
        options.language = "ts";
        break;
      case "--javascript":
      case "--js":
        assertUnset(options.language, "language");
        options.language = "js";
        break;
      case "--web":
        options.surfaces.push("web");
        break;
      case "--mobile":
      case "--mobile-android":
        options.surfaces.push("mobile-android");
        break;
      case "--mobile-ios":
      case "--ios":
        options.surfaces.push("mobile-ios");
        break;
      case "--desktop-mac":
      case "--macos":
      case "--mac":
        options.surfaces.push("desktop-mac");
        break;
      case "--desktop-windows":
      case "--windows":
      case "--win":
        options.surfaces.push("desktop-windows");
        break;
      case "--both":
        options.surfaces.push("web", "mobile-android");
        break;
      case "--all":
        options.surfaces.push("web", "mobile-android", "mobile-ios", "desktop-mac", "desktop-windows");
        break;
      case "--surface": {
        const value = args[index + 1];
        if (!value) {
          throw new Error("Expected a surface id after --surface.");
        }
        options.surfaces.push(parseSurfaceId(value));
        index += 1;
        break;
      }
      default:
        if (arg.startsWith("-")) {
          throw new Error(`Unknown option: ${arg}`);
        }
        if (options.targetDir !== ".") {
          throw new Error(`Unexpected extra argument: ${arg}`);
        }
        options.targetDir = arg;
        break;
    }
  }

  return options;
}

function assertUnset<T>(value: T | undefined, kind: string): void {
  if (value !== undefined) {
    throw new Error(`Received more than one ${kind} option.`);
  }
}

function getPackageVersion(): string {
  const require = createRequire(import.meta.url);
  const packageJson = require("../package.json") as { version?: string };
  if (!packageJson.version) {
    throw new Error("Could not determine create-allwright package version.");
  }
  return packageJson.version;
}

async function selectLanguage(interactive: boolean): Promise<Language> {
  requireInteractive(interactive, "pass --typescript, --javascript, or --yes");
  const result = await p.select({
    message: "Which language would you like to use?",
    options: [
      { label: "TypeScript", value: "ts", hint: "recommended" },
      { label: "JavaScript", value: "js" },
    ],
  });
  return resolvePromptResult(result);
}

async function selectSurfaces(interactive: boolean): Promise<SurfaceId[]> {
  requireInteractive(interactive, "pass --surface <id> or --yes");
  const result = await p.multiselect({
    message: "Which surfaces would you like to test?",
    options: SURFACE_OPTIONS.map((surface) => ({
      ...surface,
      hint: surface.value === "web"
        ? "browser automation"
        : surface.value === "mobile-android"
          ? "Android app automation"
          : surface.value === "mobile-ios"
            ? "iOS app automation"
            : surface.value === "desktop-mac"
              ? "native macOS app automation"
              : "native Windows app automation",
    })),
    required: true,
  });
  return dedupeSurfaces(resolvePromptResult(result));
}

async function confirmOverwrite(
  interactive: boolean,
  relativeTargetDir: string,
  conflicts: string[],
): Promise<boolean> {
  if (!interactive) {
    return false;
  }

  const result = await p.confirm({
    message: `Overwrite ${conflicts.length} existing file${conflicts.length === 1 ? "" : "s"} in ${relativeTargetDir}?`,
    initialValue: false,
  });
  return resolvePromptResult(result);
}

function requireInteractive(interactive: boolean, hint: string): asserts interactive {
  if (!interactive) {
    throw new Error(`Run again in an interactive terminal, or ${hint}.`);
  }
}

function resolvePromptResult<T>(result: T | symbol): T {
  if (p.isCancel(result)) {
    p.cancel("Initialization cancelled.");
    process.exit(0);
  }
  return result;
}

async function ensureTargetDirectory(targetDir: string): Promise<void> {
  await mkdir(targetDir, { recursive: true });
}

async function findConflicts(targetDir: string, paths: string[]): Promise<string[]> {
  const conflicts: string[] = [];
  for (const relativePath of paths) {
    try {
      await access(path.join(targetDir, relativePath));
      conflicts.push(relativePath);
    } catch {
      // file does not exist yet
    }
  }
  return conflicts;
}

async function writeProjectFiles(targetDir: string, files: Record<string, string>): Promise<void> {
  for (const [relativePath, contents] of Object.entries(files)) {
    const destination = path.join(targetDir, relativePath);
    await mkdir(path.dirname(destination), { recursive: true });
    await writeFile(destination, contents, "utf8");
  }
}

function buildProjectFiles(input: {
  language: Language;
  surfaces: SurfaceId[];
  packageName: string;
}): Record<string, string> {
  const { language, surfaces, packageName } = input;
  const extension = language === "ts" ? "ts" : "js";
  const files: Record<string, string> = {
    ".gitignore": "node_modules/\ncoverage/\n.allwright/\n",
    "package.json": packageJsonContents(packageName, language),
    "allwright.config.yaml": allwrightConfigContents(surfaces),
    [`vitest.config.${extension}`]: vitestConfigContents(language),
    "README.md": generatedReadmeContents(surfaces),
  };

  if (language === "ts") {
    files["tsconfig.json"] = tsconfigContents();
  }

  if (hasSurface(surfaces, "web")) {
    files[`tests/web.spec.${extension}`] = webSpecContents(language);
  }

  if (hasSurface(surfaces, "mobile-android")) {
    files[`tests/mobile.spec.${extension}`] = androidSpecContents(language);
  }

  if (hasSurface(surfaces, "mobile-ios")) {
    files[`tests/ios.spec.${extension}`] = iosSpecContents(language);
  }

  if (hasSurface(surfaces, "desktop-mac")) {
    files[`tests/macos.spec.${extension}`] = macosSpecContents(language);
  }

  if (hasSurface(surfaces, "desktop-windows")) {
    files[`tests/windows.spec.${extension}`] = windowsSpecContents(language);
  }

  return files;
}

function packageJsonContents(packageName: string, language: Language): string {
  const devDependencies =
    language === "ts"
      ? `  "devDependencies": {
    "@allwright.dev/vitest": "${ALLWRIGHT_VERSION}",
    "@types/node": "^24.4.0",
    "typescript": "${TYPESCRIPT_VERSION}",
    "vitest": "${VITEST_VERSION}"
  }`
      : `  "devDependencies": {
    "@allwright.dev/vitest": "${ALLWRIGHT_VERSION}",
    "vitest": "${VITEST_VERSION}"
  }`;

  return `{
  "name": "${packageName}",
  "version": "0.0.0",
  "private": true,
  "type": "module",
  "scripts": {
    "test": "vitest run",
    "test:watch": "vitest"
  },
${devDependencies}
}
`;
}

function tsconfigContents(): string {
  return `{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ESNext",
    "moduleResolution": "Bundler",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "forceConsistentCasingInFileNames": true,
    "types": ["vitest/globals"]
  },
  "include": ["tests", "vitest.config.ts"]
}
`;
}

function vitestConfigContents(language: Language): string {
  const defineConfigImport =
    language === "ts"
      ? 'import { defineConfig } from "vitest/config";'
      : 'import { defineConfig } from "vitest/config";';

  return `import { allwrightVitestConfig } from "@allwright.dev/vitest/config";
${defineConfigImport}

export default allwrightVitestConfig(
  defineConfig({
    test: {
      globals: true,
      environment: "node",
    },
  }),
);
`;
}

function allwrightConfigContents(surfaces: SurfaceId[]): string {
  const lines = [
    "schemaVersion: 1",
    "",
    "server:",
    '  addr: "127.0.0.1:50051"',
  ];

  if (hasSurface(surfaces, "web")) {
    lines.push("");
    lines.push("web:");
    lines.push("  browser:");
    lines.push("    name: chromium");
  }

  if (hasSurface(surfaces, "mobile-android") || hasSurface(surfaces, "mobile-ios")) {
    lines.push("");
    lines.push("mobile:");
  }

  if (hasSurface(surfaces, "mobile-android")) {
    lines.push("  android:");
    lines.push("    app:");
    lines.push("      id: com.example.airticket");
    lines.push('      binary: "https://allwright.dev/Flights-debug.apk"');
  }

  if (hasSurface(surfaces, "mobile-ios")) {
    lines.push("  ios:");
    lines.push("    app:");
    lines.push("      id: com.sedinqa.Flights");
    lines.push('      binary: "https://allwright.dev/Flights-simulator.ipa"');
  }

  if (hasSurface(surfaces, "desktop-mac") || hasSurface(surfaces, "desktop-windows")) {
    lines.push("");
    lines.push("desktop:");
  }

  if (hasSurface(surfaces, "desktop-mac")) {
    lines.push("  mac:");
    lines.push("    app:");
    lines.push("      id: com.apple.calculator");
  }

  if (hasSurface(surfaces, "desktop-windows")) {
    lines.push("  windows:");
    lines.push("    app:");
    lines.push("      id: notepad.exe");
  }

  lines.push("");
  lines.push("expect:");
  lines.push("  timeoutMs: 7000");
  lines.push("  intervalMs: 100");

  return `${lines.join("\n")}\n`;
}

function webSpecContents(language: Language): string {
  const importLine =
    language === "ts"
      ? 'import { expect, test } from "@allwright.dev/vitest";'
      : 'import { expect, test } from "@allwright.dev/vitest";';

  return `${importLine}

const WEB_URL = "https://themoderninternet.vercel.app";
const ENTRY_SELECTOR =
  "xpath=//div[contains(@class,'card')][.//h2[normalize-space()='Form Inputs']]//button[normalize-space()='Visit page']";
const HEADING_SELECTOR = 'xpath=//h1[text()="Form Inputs"]';

test("opens the Form Inputs page", { timeout: 30_000 }, async ({ page }) => {
  await page.goto(WEB_URL);
  await page.click(ENTRY_SELECTOR);
  await expect(page.locator(HEADING_SELECTOR)).toHaveText("Form Inputs");
});
`;
}

function androidSpecContents(language: Language): string {
  const importLine =
    language === "ts"
      ? 'import { test } from "@allwright.dev/vitest";'
      : 'import { test } from "@allwright.dev/vitest";';

  return `${importLine}

test("opens the Android demo app and navigates to sign up", { timeout: 180_000 }, async ({ androidApp }) => {
  await androidApp.click("text=Account");
  await androidApp.click("text=Login");
  await androidApp.click("text=Sign Up");
});
`;
}

function iosSpecContents(language: Language): string {
  const importLine =
    language === "ts"
      ? 'import { expect, test } from "@allwright.dev/vitest";'
      : 'import { expect, test } from "@allwright.dev/vitest";';

  return `${importLine}

test("opens the iOS Flights demo app", { timeout: 180_000 }, async ({ iosApp }) => {
  await iosApp.locator("text=Login").click();
  await expect(iosApp.locator("text=Welcome back")).toBeVisible();
});
`;
}

function macosSpecContents(language: Language): string {
  const importLine =
    language === "ts"
      ? 'import { expect, test } from "@allwright.dev/vitest";'
      : 'import { expect, test } from "@allwright.dev/vitest";';

  return `${importLine}

test("opens Calculator through the macOS desktop agent", { timeout: 90_000 }, async ({ macosApp }) => {
  const screenshot = await macosApp.screenshot();
  expect(screenshot.byteLength).toBeGreaterThan(0);
});
`;
}

function windowsSpecContents(language: Language): string {
  const importLine =
    language === "ts"
      ? 'import { expect, test } from "@allwright.dev/vitest";'
      : 'import { expect, test } from "@allwright.dev/vitest";';

  return `${importLine}

test("opens Notepad through the Windows desktop agent", { timeout: 90_000 }, async ({ windowsApp }) => {
  const screenshot = await windowsApp.screenshot();
  expect(screenshot.byteLength).toBeGreaterThan(0);
});
`;
}

function generatedReadmeContents(surfaces: SurfaceId[]): string {
  const nextSteps = [
    "## Next steps",
    "",
    "1. Run `npm install`.",
  ];
  let step = 2;

  if (hasSurface(surfaces, "mobile-android")) {
    nextSteps.push(
      `${step++}. Start an Android emulator or connect a device with USB debugging enabled.`,
      `${step++}. Confirm it is visible with \`adb devices -l\`.`,
      `${step++}. The first Android run downloads and installs the Airticket demo app configured in \`allwright.config.yaml\`.`,
    );
  }

  if (hasSurface(surfaces, "mobile-ios")) {
    nextSteps.push(
      `${step++}. Start an iOS Simulator, or connect a registered physical device with Developer Mode and UI Automation enabled.`,
      `${step++}. The first Simulator run downloads and installs the Flights IPA configured in \`allwright.config.yaml\`; physical-device apps must use a signed device build. No manual app or agent installation is needed.`,
    );
  }

  if (hasSurface(surfaces, "desktop-mac")) {
    nextSteps.push(
      `${step++}. Run the suite on macOS 14 or newer with Xcode installed and UI Automation enabled for the process running Xcode.`,
      `${step++}. The first macOS run installs the desktop plugin, starts its bundled XCUITest runner, and launches Calculator by bundle identifier.`,
    );
  }

  if (hasSurface(surfaces, "desktop-windows")) {
    nextSteps.push(
      `${step++}. Run the suite from an interactive Windows 10 or newer desktop session.`,
      `${step++}. The first Windows run installs the desktop plugin, starts its bundled FlaUI UIA3 agent, and launches Notepad.`,
    );
  }

  nextSteps.push(`${step}. Run \`npm test\`.`);

  return `# allwright project

Scaffolded with \`npm init allwright\`.

This starter uses \`@allwright.dev/vitest\` so your test suite can drive the allwright engine with Playwright-style fixtures.

${nextSteps.join("\n")}
`;
}

function inferPackageName(input: string): string {
  const normalized = input
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9._-]+/g, "-")
    .replace(/^-+|-+$/g, "");

  return normalized || "allwright-app";
}

function hasSurface(surfaces: SurfaceId[], surface: SurfaceId): boolean {
  return surfaces.includes(surface);
}

function parseSurfaceId(input: string): SurfaceId {
  const normalized = input.trim().toLowerCase();
  if (normalized === "web") {
    return "web";
  }
  if (normalized === "mobile" || normalized === "mobile-android" || normalized === "android") {
    return "mobile-android";
  }
  if (normalized === "mobile-ios" || normalized === "ios") {
    return "mobile-ios";
  }
  if (
    normalized === "desktop-mac" ||
    normalized === "desktop-macos" ||
    normalized === "macos" ||
    normalized === "mac"
  ) {
    return "desktop-mac";
  }
  if (
    normalized === "desktop-windows" ||
    normalized === "desktop-win" ||
    normalized === "windows" ||
    normalized === "win"
  ) {
    return "desktop-windows";
  }
  throw new Error(`Unknown surface: ${input}`);
}

function parsePackageManager(input: string): PackageManager {
  const normalized = input.trim().toLowerCase();
  if (normalized === "bun" || normalized === "npm" || normalized === "pnpm" || normalized === "yarn") {
    return normalized;
  }
  throw new Error(`Unknown package manager: ${input}. Use bun, npm, pnpm, or yarn.`);
}

function dedupeSurfaces(surfaces: SurfaceId[]): SurfaceId[] {
  return dedupeStringValues(surfaces) as SurfaceId[];
}

function dedupeStringValues<T extends string>(values: T[]): T[] {
  return [...new Set(values)];
}

async function detectPackageManager(targetDir: string): Promise<PackageManager> {
  const lockfiles: Array<{ filename: string; packageManager: PackageManager }> = [
    { filename: "bun.lock", packageManager: "bun" },
    { filename: "bun.lockb", packageManager: "bun" },
    { filename: "pnpm-lock.yaml", packageManager: "pnpm" },
    { filename: "yarn.lock", packageManager: "yarn" },
    { filename: "package-lock.json", packageManager: "npm" },
  ];

  for (const lockfile of lockfiles) {
    try {
      await access(path.join(targetDir, lockfile.filename));
      return lockfile.packageManager;
    } catch {
      // Keep looking for a package manager lockfile.
    }
  }

  const userAgent = process.env.npm_config_user_agent ?? "";
  if (userAgent.startsWith("bun/")) {
    return "bun";
  }
  if (userAgent.startsWith("pnpm/")) {
    return "pnpm";
  }
  if (userAgent.startsWith("yarn/")) {
    return "yarn";
  }
  return "npm";
}

async function installDependencies(targetDir: string, packageManager: PackageManager): Promise<void> {
  p.log.step(`Installing dependencies with ${packageManager}...`);

  await new Promise<void>((resolve, reject) => {
    const child = spawn(packageManager, ["install"], {
      cwd: targetDir,
      stdio: "inherit",
    });

    child.once("error", (error) => {
      reject(
        new Error(
          `Could not run ${packageManager} install: ${error.message}. Use --no-install or choose another manager with --package-manager.`,
        ),
      );
    });
    child.once("close", (code) => {
      if (code === 0) {
        resolve();
      } else {
        reject(new Error(`${packageManager} install exited with code ${code ?? "unknown"}.`));
      }
    });
  });
}

function printSummary(
  absoluteTargetDir: string,
  relativeTargetDir: string,
  language: Language,
  surfaces: SurfaceId[],
  packageManager: PackageManager,
  installed: boolean,
): void {
  const displayTargetDir =
    relativeTargetDir === "." || (!relativeTargetDir.startsWith("..") && !path.isAbsolute(relativeTargetDir))
      ? relativeTargetDir
      : absoluteTargetDir;

  console.log("\nInitialized an allwright project.");
  console.log(`- Directory: ${displayTargetDir}`);
  console.log(`- Language: ${language === "ts" ? "TypeScript" : "JavaScript"}`);
  console.log(`- Surfaces: ${surfaces.map(formatSurfaceLabel).join(", ")}`);
  console.log(`- Package manager: ${packageManager}`);
  console.log("\nNext steps:");
  if (displayTargetDir !== ".") {
    console.log(`  cd ${displayTargetDir}`);
  }
  if (!installed) {
    console.log(`  ${packageManager} install`);
  }
  if (hasSurface(surfaces, "mobile-android")) {
    console.log("  adb devices -l");
  }
  if (hasSurface(surfaces, "mobile-ios")) {
    console.log("  xcrun simctl list devices available");
  }
  if (hasSurface(surfaces, "desktop-mac")) {
    console.log("  xcodebuild -version");
  }
  console.log(`${packageManager === "npm" ? "  npm test" : `  ${packageManager} test`}`);
}

function formatSurfaceLabel(surface: SurfaceId): string {
  return SURFACE_OPTIONS.find((option) => option.value === surface)?.label ?? surface;
}

void main().catch((error) => {
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = 1;
});
