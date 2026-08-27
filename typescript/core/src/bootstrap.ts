import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawn, spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

import grpc from "@grpc/grpc-js";
import protoLoader from "@grpc/proto-loader";

const ALLWRIGHT_AUTO_INSTALL_ENV_VAR = "ALLWRIGHT_AUTO_INSTALL";
const ALLWRIGHT_CLI_PATH_ENV_VAR = "ALLWRIGHT_CLI_PATH";
const ALLWRIGHT_HOME_ENV_VAR = "ALLWRIGHT_HOME";
const ALLWRIGHT_REPOSITORY_ENV_VAR = "ALLWRIGHT_REPOSITORY";
const ALLWRIGHT_VERSION_ENV_VAR = "ALLWRIGHT_VERSION";
const DEFAULT_RELEASE_REPOSITORY = "allwright-dev/allwright";
const DEFAULT_RELEASE_VERSION = "0.0.1";
const STARTUP_TIMEOUT_MS = 20_000;
const PING_TIMEOUT_MS = 1_000;
const PROTO_ROOT = fileURLToPath(new URL("../proto/", import.meta.url));
const ENGINE_PROTO_PATH = fileURLToPath(new URL("../proto/engine/v1/engine.proto", import.meta.url));

let managedServer: ReturnType<typeof spawn> | null = null;
let managedServerAddr: string | null = null;

type PingClient = {
  Ping(
    request: object,
    metadata: grpc.Metadata,
    options: grpc.CallOptions,
    callback: (error: grpc.ServiceError | null, response: { message?: string }) => void,
  ): void;
  close(): void;
};

export async function ensureRuntimeReady(serverAddr: string): Promise<void> {
  if (await pingServer(serverAddr)) {
    return;
  }

  if (!isLocalServerAddr(serverAddr)) {
    throw new Error(
      `allwright could not reach engine server at ${serverAddr}. Automatic startup is only supported for local addresses.`,
    );
  }

  if (managedServer && !managedServer.killed && managedServer.exitCode === null && managedServerAddr === serverAddr) {
    await waitForServer(serverAddr);
    return;
  }

  const cliPath = await ensureCliAvailable();
  ensureWebPlugin(cliPath);

  managedServer = spawn(cliPath, ["serve", "--listen-addr", cliListenAddr(serverAddr)], {
    stdio: "ignore",
  });
  managedServerAddr = serverAddr;

  await waitForServer(serverAddr);
}

export async function shutdownManagedServer(): Promise<void> {
  if (managedServer && managedServer.exitCode === null && !managedServer.killed) {
    managedServer.kill("SIGTERM");
  }
  managedServer = null;
  managedServerAddr = null;
}

async function waitForServer(serverAddr: string): Promise<void> {
  const deadline = Date.now() + STARTUP_TIMEOUT_MS;
  while (Date.now() < deadline) {
    if (await pingServer(serverAddr)) {
      return;
    }
    await new Promise((resolve) => setTimeout(resolve, 250));
  }
  await shutdownManagedServer();
  throw new Error(`timed out waiting for allwright server at ${serverAddr} to become ready`);
}

async function pingServer(serverAddr: string): Promise<boolean> {
  const loaded = protoLoader.loadSync(ENGINE_PROTO_PATH, {
    includeDirs: [PROTO_ROOT],
    keepCase: false,
    longs: String,
    enums: String,
    defaults: true,
    oneofs: true,
  });
  const proto = grpc.loadPackageDefinition(loaded) as unknown as {
    allwright: { engine: { v1: { EngineService: new (addr: string, creds: grpc.ChannelCredentials) => PingClient } } };
  };
  const client = new proto.allwright.engine.v1.EngineService(
    serverAddr,
    grpc.credentials.createInsecure(),
  );

  return await new Promise<boolean>((resolve) => {
    client.Ping({}, new grpc.Metadata(), { deadline: new Date(Date.now() + PING_TIMEOUT_MS) }, (error) => {
      client.close();
      resolve(!error);
    });
  });
}

async function ensureCliAvailable(): Promise<string> {
  const envPath = process.env[ALLWRIGHT_CLI_PATH_ENV_VAR]?.trim();
  if (envPath && isFile(envPath)) {
    return envPath;
  }

  const bundled = path.join(allwrightHome(), "bin", cliFilename());
  if (isFile(bundled)) {
    return bundled;
  }

  const fromPath = resolveFromPath(cliFilename());
  if (fromPath) {
    return fromPath;
  }

  if (!autoInstallEnabled()) {
    throw new Error("allwright CLI was not found. Install it first or set ALLWRIGHT_CLI_PATH.");
  }

  return await installCli();
}

async function installCli(): Promise<string> {
  const installDir = path.join(allwrightHome(), "bin");
  fs.mkdirSync(installDir, { recursive: true });
  const cliPath = path.join(installDir, cliFilename());
  const versionTag = await resolveReleaseTag();
  const assetName = cliAssetName(versionTag);
  const assetPath = path.join(os.tmpdir(), assetName);
  const response = await fetch(
    `https://github.com/${releaseRepository()}/releases/download/${versionTag}/${assetName}`,
    { headers: { "user-agent": `allwright-ts/${DEFAULT_RELEASE_VERSION}` } },
  );
  if (!response.ok) {
    throw new Error(`failed to download allwright CLI asset ${assetName}: ${response.status} ${response.statusText}`);
  }
  fs.writeFileSync(assetPath, Buffer.from(await response.arrayBuffer()));
  extractCliArchive(assetPath, cliPath);
  fs.chmodSync(cliPath, 0o755);
  fs.rmSync(assetPath, { force: true });
  return cliPath;
}

function ensureWebPlugin(cliPath: string): void {
  const pluginPath = path.join(allwrightHome(), "plugins", "web", "lib", webPluginFilename());
  if (isFile(pluginPath)) {
    return;
  }

  const version = process.env[ALLWRIGHT_VERSION_ENV_VAR]?.trim() || DEFAULT_RELEASE_VERSION;
  const result = spawnSync(cliPath, ["plugin", "install", "web", "--version", normalizeReleaseVersion(version)], {
    stdio: "ignore",
  });
  if (result.status !== 0 || !isFile(pluginPath)) {
    throw new Error("allwright attempted to install the `web` plugin automatically, but the install did not complete successfully");
  }
}

async function resolveReleaseTag(): Promise<string> {
  const version = process.env[ALLWRIGHT_VERSION_ENV_VAR]?.trim() || DEFAULT_RELEASE_VERSION;
  if (version !== "latest") {
    return normalizeReleaseTag(version);
  }
  const response = await fetch(`https://api.github.com/repos/${releaseRepository()}/releases/latest`, {
    headers: { "user-agent": `allwright-ts/${DEFAULT_RELEASE_VERSION}` },
  });
  if (!response.ok) {
    throw new Error(`failed to resolve latest allwright release: ${response.status} ${response.statusText}`);
  }
  const payload = (await response.json()) as { tag_name?: string };
  if (!payload.tag_name?.trim()) {
    throw new Error("latest allwright release metadata did not include tag_name");
  }
  return payload.tag_name;
}

function extractCliArchive(archivePath: string, cliPath: string): void {
  if (archivePath.endsWith(".zip")) {
    const result = spawnSync("powershell", [
      "-NoProfile",
      "-Command",
      `Expand-Archive -Path '${archivePath.replaceAll("'", "''")}' -DestinationPath '${path.dirname(cliPath).replaceAll("'", "''")}' -Force`,
    ], { stdio: "ignore" });
    if (result.status !== 0) {
      throw new Error("failed to extract allwright CLI zip archive");
    }
    const extracted = path.join(path.dirname(cliPath), "bin", cliFilename());
    fs.copyFileSync(extracted, cliPath);
    fs.rmSync(path.join(path.dirname(cliPath), "bin"), { recursive: true, force: true });
    return;
  }

  const result = spawnSync("tar", [
    "-xzf",
    archivePath,
    "-C",
    path.dirname(cliPath),
    `bin/${cliFilename()}`,
  ], { stdio: "ignore" });
  if (result.status !== 0) {
    throw new Error("failed to extract allwright CLI tar archive");
  }
  const extracted = path.join(path.dirname(cliPath), "bin", cliFilename());
  fs.copyFileSync(extracted, cliPath);
  fs.rmSync(path.join(path.dirname(cliPath), "bin"), { recursive: true, force: true });
}

function cliAssetName(versionTag: string): string {
  const targets = new Map<string, string>([
    ["darwin/arm64", "aarch64-apple-darwin"],
    ["darwin/x64", "x86_64-apple-darwin"],
    ["linux/arm64", "aarch64-unknown-linux-gnu"],
    ["linux/x64", "x86_64-unknown-linux-gnu"],
    ["win32/arm64", "aarch64-pc-windows-msvc"],
    ["win32/x64", "x86_64-pc-windows-msvc"],
  ]);
  const target = targets.get(`${process.platform}/${process.arch}`);
  if (!target) {
    throw new Error(`automatic allwright CLI install is not supported on ${process.platform}/${process.arch}`);
  }
  const extension = process.platform === "win32" ? "zip" : "tar.gz";
  return `allwright-${versionTag}-${target}.${extension}`;
}

function normalizeReleaseTag(version: string): string {
  return version.startsWith("v") ? version : `v${version}`;
}

function normalizeReleaseVersion(version: string): string {
  return version.replace(/^v/, "");
}

function cliListenAddr(serverAddr: string): string {
  return serverAddr.replace(/^https?:\/\//, "");
}

function isLocalServerAddr(serverAddr: string): boolean {
  const host = cliListenAddr(serverAddr).split(":")[0]?.replace(/^\[|\]$/g, "") ?? "";
  return host === "127.0.0.1" || host === "localhost" || host === "::1";
}

function allwrightHome(): string {
  return process.env[ALLWRIGHT_HOME_ENV_VAR]?.trim() || path.join(os.homedir(), ".allwright");
}

function cliFilename(): string {
  return process.platform === "win32" ? "allwright.exe" : "allwright";
}

function webPluginFilename(): string {
  if (process.platform === "darwin") {
    return "liballwright_surface_web.dylib";
  }
  if (process.platform === "win32") {
    return "allwright_surface_web.dll";
  }
  return "liballwright_surface_web.so";
}

function autoInstallEnabled(): boolean {
  const raw = process.env[ALLWRIGHT_AUTO_INSTALL_ENV_VAR]?.trim().toLowerCase();
  return raw !== "0" && raw !== "false" && raw !== "no";
}

function resolveFromPath(filename: string): string | null {
  for (const entry of (process.env.PATH ?? "").split(path.delimiter)) {
    if (!entry) {
      continue;
    }
    const candidate = path.join(entry, filename);
    if (isFile(candidate)) {
      return candidate;
    }
  }
  return null;
}

function releaseRepository(): string {
  return process.env[ALLWRIGHT_REPOSITORY_ENV_VAR]?.trim() || DEFAULT_RELEASE_REPOSITORY;
}

function isFile(candidate: string): boolean {
  try {
    return fs.statSync(candidate).isFile();
  } catch {
    return false;
  }
}
