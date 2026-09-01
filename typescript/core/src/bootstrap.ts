import fs from "node:fs";
import net from "node:net";
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
const DEFAULT_RELEASE_VERSION = "0.0.58";
const STARTUP_TIMEOUT_MS = 20_000;
const PING_TIMEOUT_MS = 1_000;
const PROTO_ROOT = fileURLToPath(new URL("../proto/", import.meta.url));
const ENGINE_PROTO_PATH = fileURLToPath(new URL("../proto/engine/v1/engine.proto", import.meta.url));

let managedServer: ReturnType<typeof spawn> | null = null;
let managedServerAddr: string | null = null;
let managedServerBaseAddr: string | null = null;
let managedServerStdout = "";
let managedServerStderr = "";
let managedServerSpawnError: string | null = null;

type PingClient = {
  Ping(
    request: object,
    metadata: grpc.Metadata,
    options: grpc.CallOptions,
    callback: (error: grpc.ServiceError | null, response: { message?: string; version?: string }) => void,
  ): void;
  close(): void;
};

export async function ensureRuntimeReady(serverAddr: string): Promise<string> {
  const expectedVersion = expectedRuntimeVersion();
  const status = await pingServer(serverAddr);
  if (status) {
    if (status.version === expectedVersion) {
      return serverAddr;
    }
    if (!isLocalServerAddr(serverAddr)) {
      throw new Error(
        `allwright server at ${serverAddr} is running version ${displayVersion(status.version)} but this client expects ${expectedVersion}`,
      );
    }
  }

  if (!isLocalServerAddr(serverAddr)) {
    throw new Error(
      `allwright could not reach engine server at ${serverAddr}. Automatic startup is only supported for local addresses.`,
    );
  }

  if (managedServer && !managedServer.killed && managedServer.exitCode === null && managedServerBaseAddr === serverAddr && managedServerAddr) {
    return waitForServer(managedServerAddr, expectedVersion);
  }
  if (managedServer && !managedServer.killed && managedServer.exitCode === null) {
    managedServer.kill("SIGTERM");
    managedServer = null;
    managedServerAddr = null;
    managedServerBaseAddr = null;
    managedServerStdout = "";
    managedServerStderr = "";
    managedServerSpawnError = null;
  }

  let resolvedServerAddr = serverAddr;
  if (status && status.version !== expectedVersion) {
    resolvedServerAddr = await allocateManagedServerAddr(serverAddr);
  }

  const cliPath = await ensureCliAvailable(expectedVersion);
  managedServer = spawn(cliPath, ["serve", "--listen-addr", cliListenAddr(resolvedServerAddr)], {
    stdio: ["ignore", "pipe", "pipe"],
  });
  managedServerStdout = "";
  managedServerStderr = "";
  managedServerSpawnError = null;
  managedServer.stdout?.on("data", (chunk: Buffer | string) => {
    managedServerStdout = appendManagedServerOutput(managedServerStdout, chunk);
  });
  managedServer.stderr?.on("data", (chunk: Buffer | string) => {
    managedServerStderr = appendManagedServerOutput(managedServerStderr, chunk);
  });
  managedServer.on("error", (error) => {
    managedServerSpawnError = error.message;
  });
  managedServerAddr = resolvedServerAddr;
  managedServerBaseAddr = serverAddr;

  return waitForServer(resolvedServerAddr, expectedVersion);
}

export async function shutdownManagedServer(): Promise<void> {
  if (managedServer && managedServer.exitCode === null && !managedServer.killed) {
    managedServer.kill("SIGTERM");
  }
  managedServer = null;
  managedServerAddr = null;
  managedServerBaseAddr = null;
  managedServerStdout = "";
  managedServerStderr = "";
  managedServerSpawnError = null;
}

async function waitForServer(serverAddr: string, expectedVersion: string): Promise<string> {
  const deadline = Date.now() + STARTUP_TIMEOUT_MS;
  while (Date.now() < deadline) {
    if (managedServer && managedServer.exitCode !== null) {
      const details = formatManagedServerFailure(managedServer.exitCode, managedServer.signalCode);
      await shutdownManagedServer();
      throw new Error(`allwright server exited before becoming ready at ${serverAddr}${details}`);
    }
    if (managedServerSpawnError) {
      const details = formatManagedServerFailure(null, null);
      await shutdownManagedServer();
      throw new Error(`failed to start allwright server at ${serverAddr}${details}`);
    }
    const status = await pingServer(serverAddr);
    if (status?.version === expectedVersion) {
      return serverAddr;
    }
    await new Promise((resolve) => setTimeout(resolve, 250));
  }
  const details = formatManagedServerFailure(managedServer?.exitCode ?? null, managedServer?.signalCode ?? null);
  await shutdownManagedServer();
  throw new Error(`timed out waiting for allwright server at ${serverAddr} to become ready with version ${expectedVersion}${details}`);
}

async function pingServer(serverAddr: string): Promise<{ version: string } | null> {
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

  return await new Promise<{ version: string } | null>((resolve) => {
    client.Ping({}, new grpc.Metadata(), { deadline: new Date(Date.now() + PING_TIMEOUT_MS) }, (error, response) => {
      client.close();
      if (error) {
        resolve(null);
        return;
      }
      resolve({ version: normalizeReleaseVersion(response.version ?? "") });
    });
  });
}

async function ensureCliAvailable(expectedVersion: string): Promise<string> {
  const envPath = process.env[ALLWRIGHT_CLI_PATH_ENV_VAR]?.trim();
  if (envPath && isFile(envPath) && cliVersionMatches(envPath, expectedVersion)) {
    return envPath;
  }

  const bundled = path.join(allwrightHome(), "bin", cliFilename());
  if (isFile(bundled) && cliVersionMatches(bundled, expectedVersion)) {
    return bundled;
  }

  const repoLocal = repoLocalCliPath();
  if (repoLocal && cliVersionMatches(repoLocal, expectedVersion)) {
    return repoLocal;
  }

  const fromPath = resolveFromPath(cliFilename());
  if (fromPath && cliVersionMatches(fromPath, expectedVersion)) {
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
  const extractRoot = fs.mkdtempSync(path.join(os.tmpdir(), "allwright-cli-"));
  if (archivePath.endsWith(".zip")) {
    const result = spawnSync("powershell", [
      "-NoProfile",
      "-Command",
      `Expand-Archive -Path '${archivePath.replaceAll("'", "''")}' -DestinationPath '${extractRoot.replaceAll("'", "''")}' -Force`,
    ], { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] });
    if (result.error) {
      fs.rmSync(extractRoot, { recursive: true, force: true });
      throw new Error(`failed to extract allwright CLI zip archive: ${result.error.message}`);
    }
    if (result.status !== 0) {
      const details = [result.stdout, result.stderr].map((value) => value?.trim()).filter(Boolean).join("\n");
      fs.rmSync(extractRoot, { recursive: true, force: true });
      throw new Error(details ? `failed to extract allwright CLI zip archive:\n${details}` : "failed to extract allwright CLI zip archive");
    }
    const extracted = findExtractedCli(extractRoot);
    if (!extracted) {
      fs.rmSync(extractRoot, { recursive: true, force: true });
      throw new Error(`allwright CLI zip archive did not contain bin/${cliFilename()}`);
    }
    fs.copyFileSync(extracted, cliPath);
    fs.rmSync(extractRoot, { recursive: true, force: true });
    return;
  }

  const result = spawnSync("tar", [
    "-xzf",
    archivePath,
    "-C",
    extractRoot,
  ], { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] });
  if (result.error) {
    fs.rmSync(extractRoot, { recursive: true, force: true });
    throw new Error(`failed to extract allwright CLI tar archive: ${result.error.message}`);
  }
  if (result.status !== 0) {
    const details = [result.stdout, result.stderr].map((value) => value?.trim()).filter(Boolean).join("\n");
    fs.rmSync(extractRoot, { recursive: true, force: true });
    throw new Error(details ? `failed to extract allwright CLI tar archive:\n${details}` : "failed to extract allwright CLI tar archive");
  }
  const extracted = findExtractedCli(extractRoot);
  if (!extracted) {
    fs.rmSync(extractRoot, { recursive: true, force: true });
    throw new Error(`allwright CLI archive did not contain bin/${cliFilename()}`);
  }
  fs.copyFileSync(extracted, cliPath);
  fs.rmSync(extractRoot, { recursive: true, force: true });
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

function expectedRuntimeVersion(): string {
  return normalizeReleaseVersion(process.env[ALLWRIGHT_VERSION_ENV_VAR]?.trim() || DEFAULT_RELEASE_VERSION);
}

function cliListenAddr(serverAddr: string): string {
  return serverAddr.replace(/^https?:\/\//, "");
}

function isLocalServerAddr(serverAddr: string): boolean {
  const host = parseServerHost(serverAddr);
  return host === "127.0.0.1" || host === "localhost" || host === "::1";
}

async function allocateManagedServerAddr(serverAddr: string): Promise<string> {
  const host = localBindingHost(serverAddr);
  const port = await new Promise<number>((resolve, reject) => {
    const server = net.createServer();
    server.unref();
    server.on("error", reject);
    server.listen(0, host, () => {
      const address = server.address();
      if (!address || typeof address === "string") {
        server.close(() => reject(new Error("failed to reserve a local port for allwright")));
        return;
      }
      const { port } = address;
      server.close((error) => {
        if (error) {
          reject(error);
          return;
        }
        resolve(port);
      });
    });
  });
  return host.includes(":") ? `[${host}]:${port}` : `${host}:${port}`;
}

function localBindingHost(serverAddr: string): string {
  const host = parseServerHost(serverAddr);
  return host === "::1" ? "::1" : "127.0.0.1";
}

function displayVersion(version: string): string {
  return version || "unknown";
}

function repoLocalCliPath(): string | null {
  const repoRoot = fileURLToPath(new URL("../../..", import.meta.url));
  for (const candidate of [
    path.join(repoRoot, "target", "debug", cliFilename()),
    path.join(repoRoot, "target", "release", cliFilename()),
  ]) {
    if (isFile(candidate)) {
      return candidate;
    }
  }
  return null;
}

function cliVersionMatches(cliPath: string, expectedVersion: string): boolean {
  const result = spawnSync(cliPath, ["--version"], { stdio: ["ignore", "pipe", "ignore"], encoding: "utf8" });
  if (result.status !== 0) {
    return false;
  }
  for (const token of result.stdout.split(/\s+/)) {
    if (/^\d/.test(token)) {
      return normalizeReleaseVersion(token) === expectedVersion;
    }
  }
  return false;
}

function parseServerHost(serverAddr: string): string {
  const listenAddr = cliListenAddr(serverAddr);
  const ipv6Match = listenAddr.match(/^\[([^\]]+)\]/);
  if (ipv6Match) {
    return ipv6Match[1] ?? "";
  }
  return listenAddr.split(":")[0] ?? "";
}

function allwrightHome(): string {
  return process.env[ALLWRIGHT_HOME_ENV_VAR]?.trim() || path.join(os.homedir(), ".allwright");
}

function cliFilename(): string {
  return process.platform === "win32" ? "allwright.exe" : "allwright";
}

function appendManagedServerOutput(current: string, chunk: Buffer | string): string {
  const next = `${current}${typeof chunk === "string" ? chunk : chunk.toString("utf8")}`;
  return next.length > 8_000 ? next.slice(-8_000) : next;
}

function formatManagedServerFailure(exitCode: number | null, signalCode: NodeJS.Signals | null): string {
  const parts: string[] = [];
  if (managedServerSpawnError) {
    parts.push(`spawn error: ${managedServerSpawnError}`);
  }
  if (exitCode !== null) {
    parts.push(`exit code: ${exitCode}`);
  }
  if (signalCode) {
    parts.push(`signal: ${signalCode}`);
  }
  if (managedServerStdout.trim()) {
    parts.push(`stdout:\n${managedServerStdout.trim()}`);
  }
  if (managedServerStderr.trim()) {
    parts.push(`stderr:\n${managedServerStderr.trim()}`);
  }
  return parts.length > 0 ? `\n${parts.join("\n")}` : "";
}

function findExtractedCli(extractRoot: string): string | null {
  const queue = [extractRoot];
  while (queue.length > 0) {
    const current = queue.shift();
    if (!current) {
      continue;
    }
    for (const entry of fs.readdirSync(current, { withFileTypes: true })) {
      const entryPath = path.join(current, entry.name);
      if (entry.isDirectory()) {
        queue.push(entryPath);
        continue;
      }
      if (entry.isFile() && entry.name === cliFilename() && path.basename(path.dirname(entryPath)) === "bin") {
        return entryPath;
      }
    }
  }
  return null;
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
