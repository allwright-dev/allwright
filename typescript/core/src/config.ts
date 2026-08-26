import fs from "node:fs";
import path from "node:path";

import type {
  AllwrightConfig,
  LaunchOptions,
  ResolveConfigOptions,
  ResolvedAllwrightConfig,
} from "./types.js";

const CONFIG_FILENAMES = [
  "allwright.config.yaml",
  "allwright.config.yml",
  "allwright.config.json",
  ".allwright/config.yaml",
  ".allwright/config.yml",
  ".allwright/config.json",
] as const;

export function findConfigFile(startDir = process.cwd()): string | null {
  let currentDir = path.resolve(startDir);

  while (true) {
    for (const filename of CONFIG_FILENAMES) {
      const candidate = path.join(currentDir, filename);
      if (fs.existsSync(candidate) && fs.statSync(candidate).isFile()) {
        return candidate;
      }
    }

    const parentDir = path.dirname(currentDir);
    if (parentDir === currentDir) {
      return null;
    }
    currentDir = parentDir;
  }
}

export function loadConfigFile(configFile: string): AllwrightConfig {
  const resolved = path.resolve(configFile);
  const raw = fs.readFileSync(resolved, "utf8");
  const parsed = parseConfigContents(raw, resolved);
  validateConfigShape(parsed, resolved);
  return parsed as AllwrightConfig;
}

export function resolveConfig(options: ResolveConfigOptions = {}): ResolvedAllwrightConfig {
  const configFilePath =
    options.configFile ? path.resolve(options.configFile) : findConfigFile(options.cwd);
  const fileConfig = configFilePath ? loadConfigFile(configFilePath) : {};
  const suiteName = options.suite?.trim() || null;
  const suiteConfig = suiteName ? fileConfig.suites?.[suiteName] : undefined;

  if (suiteName && !suiteConfig) {
    throw new Error(
      `allwright config suite "${suiteName}" was not found in ${configFilePath ?? "the resolved config file"}`,
    );
  }

  const serverAddr = suiteConfig?.server?.addr ?? fileConfig.server?.addr;
  const browserName = suiteConfig?.browser?.name ?? fileConfig.browser?.name ?? "chromium";
  const browserBinary = suiteConfig?.browser?.binary ?? fileConfig.browser?.binary;
  const launchOptions = mergeLaunchOptions(
    fileConfig.browser?.launchOptions,
    suiteConfig?.browser?.launchOptions,
  );
  const expect = {
    ...(fileConfig.expect ?? {}),
    ...(suiteConfig?.expect ?? {}),
  };

  return {
    configFilePath,
    suiteName,
    serverAddr,
    browserName,
    browserBinary,
    launchOptions: browserBinary ? { ...launchOptions, browserBinary } : launchOptions,
    expect,
  };
}

function mergeLaunchOptions(
  base?: LaunchOptions,
  override?: LaunchOptions,
): LaunchOptions {
  return {
    ...(base ?? {}),
    ...(override ?? {}),
  };
}

function validateConfigShape(value: unknown, source: string): void {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw new Error(`allwright config ${source} must contain a top-level object`);
  }

  const config = value as Record<string, unknown>;
  if (config.schemaVersion !== undefined && config.schemaVersion !== 1) {
    throw new Error(
      `allwright config ${source} has unsupported schemaVersion ${String(config.schemaVersion)}; expected 1`,
    );
  }

  const browserName =
    (config.browser as { name?: unknown } | undefined)?.name;
  if (browserName !== undefined && browserName !== "chromium" && browserName !== "firefox") {
    throw new Error(
      `allwright config ${source} has unsupported browser.name ${String(browserName)}; use "chromium" or "firefox"`,
    );
  }
}

function parseConfigContents(raw: string, source: string): unknown {
  const extension = path.extname(source).toLowerCase();
  if (extension === ".json") {
    return JSON.parse(raw) as unknown;
  }
  if (extension === ".yaml" || extension === ".yml") {
    return parseSimpleYaml(raw, source);
  }
  throw new Error(
    `unsupported allwright config file extension ${extension || "<none>"} for ${source}`,
  );
}

function parseSimpleYaml(raw: string, source: string): unknown {
  const root: Record<string, unknown> = {};
  const stack: Array<{ indent: number; value: Record<string, unknown> }> = [
    { indent: -1, value: root },
  ];

  for (const [index, originalLine] of raw.split(/\r?\n/).entries()) {
    const lineNumber = index + 1;
    const line = stripYamlComment(originalLine);
    if (!line.trim()) {
      continue;
    }

    const indent = countLeadingSpaces(line);
    if (indent % 2 !== 0) {
      throw new Error(
        `invalid YAML indentation in ${source}:${lineNumber}; use multiples of 2 spaces`,
      );
    }

    while (stack.length > 1 && indent <= stack[stack.length - 1]!.indent) {
      stack.pop();
    }

    const current = stack[stack.length - 1]!;
    const trimmed = line.trim();
    const separatorIndex = trimmed.indexOf(":");
    if (separatorIndex <= 0) {
      throw new Error(`invalid YAML mapping in ${source}:${lineNumber}`);
    }

    const key = trimmed.slice(0, separatorIndex).trim();
    const rawValue = trimmed.slice(separatorIndex + 1).trim();
    if (!key) {
      throw new Error(`empty YAML key in ${source}:${lineNumber}`);
    }

    if (!rawValue) {
      const child: Record<string, unknown> = {};
      current.value[key] = child;
      stack.push({ indent, value: child });
      continue;
    }

    current.value[key] = parseYamlScalar(rawValue, source, lineNumber);
  }

  return root;
}

function stripYamlComment(line: string): string {
  let inSingleQuote = false;
  let inDoubleQuote = false;

  for (let index = 0; index < line.length; index += 1) {
    const char = line[index];
    if (char === "'" && !inDoubleQuote) {
      inSingleQuote = !inSingleQuote;
      continue;
    }
    if (char === "\"" && !inSingleQuote) {
      inDoubleQuote = !inDoubleQuote;
      continue;
    }
    if (char === "#" && !inSingleQuote && !inDoubleQuote) {
      return line.slice(0, index);
    }
  }

  return line;
}

function countLeadingSpaces(line: string): number {
  let count = 0;
  while (count < line.length && line[count] === " ") {
    count += 1;
  }
  return count;
}

function parseYamlScalar(value: string, source: string, lineNumber: number): unknown {
  if ((value.startsWith("\"") && value.endsWith("\"")) || (value.startsWith("'") && value.endsWith("'"))) {
    return value.slice(1, -1);
  }
  if (value === "true") {
    return true;
  }
  if (value === "false") {
    return false;
  }
  if (value === "null") {
    return null;
  }
  if (/^-?\d+$/.test(value)) {
    return Number.parseInt(value, 10);
  }
  if (/^-?\d+\.\d+$/.test(value)) {
    return Number.parseFloat(value);
  }
  if (value.startsWith("[") || value.startsWith("{")) {
    throw new Error(
      `unsupported YAML collection syntax in ${source}:${lineNumber}; use nested mappings instead`,
    );
  }
  return value;
}
