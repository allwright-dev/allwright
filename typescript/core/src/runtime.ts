import { fileURLToPath } from "node:url";

import grpc from "@grpc/grpc-js";
import protoLoader from "@grpc/proto-loader";

import { ensureRuntimeReady, shutdownManagedServer } from "./bootstrap.js";
import { EventQueue } from "./types.js";
import type {
  BrowserSessionEvent,
  BrowserSessionStream,
  Browser,
  BrowserKind,
  EngineProtoRoot,
  EngineServiceClientShape,
  LaunchOptions,
  PageHandle,
  ResolveConfigOptions,
  ResolvedAllwrightConfig,
  RuntimeClient,
} from "./types.js";

const DEFAULT_SERVER_ADDR = "127.0.0.1:50051";
const SERVER_ADDR_ENV_VAR = "ALLWRIGHT_SERVER_ADDR";
const PROTO_ROOT = fileURLToPath(new URL("../proto/", import.meta.url));
const ENGINE_PROTO_PATH = fileURLToPath(new URL("../proto/engine/v1/engine.proto", import.meta.url));

let runtimePromise: Promise<RuntimeClient> | null = null;
let serverAddrOverride: string | null = null;

export function setServerAddr(serverAddr: string): void {
  serverAddrOverride = normalizeServerAddr(serverAddr);
  runtimePromise = null;
  void shutdownManagedServer();
}

export async function shutdown(): Promise<void> {
  if (!runtimePromise) {
    return;
  }
  const runtime = await runtimePromise;
  runtime.client.close();
  runtimePromise = null;
  await shutdownManagedServer();
}

export async function ping(): Promise<string> {
  const runtime = await getRuntime();
  return new Promise<string>((resolve, reject) => {
    runtime.client.Ping({}, (error, response) => {
      if (error) {
        reject(new Error(`ping engine server: ${error.message}`));
        return;
      }
      resolve(response.message ?? "");
    });
  });
}

export async function getRuntime(): Promise<RuntimeClient> {
  if (!runtimePromise) {
    runtimePromise = createRuntime();
  }
  return runtimePromise;
}

export async function createPageHandle(runtime: RuntimeClient): Promise<PageHandle> {
  const stream = runtime.client.TabSession();
  const queue = bindStreamQueue(stream);
  return {
    stream,
    queue,
    closed: false,
  };
}

export async function createBrowserSessionHandle(
  runtime: RuntimeClient,
): Promise<{ stream: BrowserSessionStream; queue: EventQueue<BrowserSessionEvent> }> {
  const stream = runtime.client.BrowserSession();
  const queue = bindStreamQueue(stream);
  return { stream, queue };
}

export async function launchConfiguredBrowser(
  config: ResolvedAllwrightConfig,
  launchBrowserKind: (browserKind: BrowserKind, options?: LaunchOptions) => Promise<Browser>,
): Promise<Browser> {
  return launchBrowserKind(config.browserName, {
    ...config.launchOptions,
    browserBinary: config.browserBinary ?? config.launchOptions.browserBinary,
  });
}

export function normalizeServerAddr(raw: string): string {
  const trimmed = raw.trim();
  if (trimmed.startsWith("dns:") || trimmed.startsWith("unix:")) {
    return trimmed;
  }
  if (trimmed.includes("://")) {
    const parsed = new URL(trimmed);
    return parsed.host;
  }
  return trimmed;
}

export function resolveLaunchBrowserArgs(
  browserKindOrOptions: BrowserKind | ResolveConfigOptions | undefined,
): browserKindOrOptions is undefined | ResolveConfigOptions {
  return browserKindOrOptions === undefined || typeof browserKindOrOptions !== "string";
}

async function createRuntime(): Promise<RuntimeClient> {
  const serverAddr = configuredServerAddr();
  await ensureRuntimeReady(serverAddr);
  const loaded = protoLoader.loadSync(ENGINE_PROTO_PATH, {
    includeDirs: [PROTO_ROOT],
    keepCase: false,
    longs: String,
    enums: String,
    defaults: true,
    oneofs: true,
  });
  const proto = grpc.loadPackageDefinition(loaded) as unknown as EngineProtoRoot;
  const ClientCtor = proto.allwright.engine.v1.EngineService;
  const client = new ClientCtor(
    serverAddr,
    grpc.credentials.createInsecure(),
  ) as unknown as EngineServiceClientShape;
  return { client };
}

function configuredServerAddr(): string {
  if (serverAddrOverride) {
    return serverAddrOverride;
  }
  return normalizeServerAddr(process.env[SERVER_ADDR_ENV_VAR] ?? DEFAULT_SERVER_ADDR);
}

function bindStreamQueue<TEvent>(stream: grpc.ClientReadableStream<TEvent>): EventQueue<TEvent> {
  const queue = new EventQueue<TEvent>();
  stream.on("data", (event) => {
    queue.push(event);
  });
  stream.on("error", (error) => {
    queue.fail(new Error(`grpc stream error: ${error.message}`));
  });
  stream.on("end", () => {
    queue.fail(new Error("grpc stream ended"));
  });
  return queue;
}
