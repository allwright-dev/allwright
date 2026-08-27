export class AllwrightError extends Error {
  readonly debugDetails?: string;

  constructor(message: string, debugDetails?: string) {
    super(message);
    this.name = "AllwrightError";
    this.debugDetails = debugDetails;
  }
}

export function formatStreamError(raw: string): AllwrightError {
  const normalized = stripTransportPrefix(raw);
  const evaluated = extractEvaluateMessage(normalized);
  return createError(evaluated.userMessage, raw, evaluated.debugDetails);
}

export function formatActionError(action: string, raw: string, locator?: string): AllwrightError {
  const normalized = stripTransportPrefix(raw);
  const evaluated = extractEvaluateMessage(normalized);
  const locatorLabel = locator ? ` for locator ${formatLocator(locator)}` : "";
  const message = evaluated.userMessage
    ? `${capitalize(action)} failed${locatorLabel}: ${evaluated.userMessage}`
    : `${capitalize(action)} failed${locatorLabel}.`;
  return createError(message, raw, evaluated.debugDetails);
}

function createError(message: string, raw: string, debugDetails?: string): AllwrightError {
  if (debugEnabled()) {
    return new AllwrightError(`${message}\n\nDebug details: ${debugDetails ?? raw}`, debugDetails ?? raw);
  }
  return new AllwrightError(message, debugDetails ?? raw);
}

function extractEvaluateMessage(raw: string): { userMessage: string; debugDetails?: string } {
  const exceptionMarker = "Runtime.evaluate failed with exception details:";
  const mapperMarker = "mapper Runtime.evaluate raised exception details:";
  const marker = raw.includes(exceptionMarker)
    ? exceptionMarker
    : raw.includes(mapperMarker)
      ? mapperMarker
      : null;
  if (!marker) {
    return { userMessage: cleanupUserMessage(raw) };
  }

  const payload = raw.slice(raw.indexOf(marker) + marker.length).trim();
  const parsed = tryParseJson(payload);
  const message =
    pickString(parsed, ["exception", "message"]) ??
    pickPreviewProperty(parsed, "message") ??
    pickString(parsed, ["exception", "description"]) ??
    raw;
  return {
    userMessage: cleanupUserMessage(message),
    debugDetails: payload,
  };
}

function cleanupUserMessage(message: string): string {
  const compact = message.replace(/\s+/g, " ").trim();
  const invalidQuerySelector = compact.match(
    /Failed to execute 'querySelector(All)?' on 'Document': '(.+?)' is not a valid selector\.?/,
  );
  if (invalidQuerySelector) {
    return `invalid selector ${invalidQuerySelector[2]}`;
  }
  return compact
    .replace(/^SyntaxError:\s*/i, "")
    .replace(/^DOMException:\s*/i, "")
    .replace(/^Error:\s*/i, "");
}

function stripTransportPrefix(raw: string): string {
  return raw
    .replace(/^grpc stream error:\s*/i, "")
    .replace(/^\d+\s+INTERNAL:\s*/i, "")
    .trim();
}

function capitalize(value: string): string {
  return value.charAt(0).toUpperCase() + value.slice(1);
}

function formatLocator(locator: string): string {
  return JSON.stringify(locator.trim());
}

function debugEnabled(): boolean {
  const raw = process.env.ALLWRIGHT_DEBUG?.trim().toLowerCase();
  return raw === "1" || raw === "true" || raw === "yes";
}

function tryParseJson(value: string): unknown {
  try {
    return JSON.parse(value) as unknown;
  } catch {
    return null;
  }
}

function pickString(root: unknown, path: string[]): string | null {
  let current: unknown = root;
  for (const segment of path) {
    if (!current || typeof current !== "object" || !(segment in current)) {
      return null;
    }
    current = (current as Record<string, unknown>)[segment];
  }
  return typeof current === "string" && current.trim() ? current.trim() : null;
}

function pickPreviewProperty(root: unknown, name: string): string | null {
  const properties = pickUnknown(root, ["exception", "preview", "properties"]);
  if (!Array.isArray(properties)) {
    return null;
  }
  for (const property of properties) {
    if (
      property &&
      typeof property === "object" &&
      (property as Record<string, unknown>).name === name &&
      typeof (property as Record<string, unknown>).value === "string"
    ) {
      return ((property as Record<string, string>).value ?? "").trim() || null;
    }
  }
  return null;
}

function pickUnknown(root: unknown, path: string[]): unknown {
  let current: unknown = root;
  for (const segment of path) {
    if (!current || typeof current !== "object" || !(segment in current)) {
      return null;
    }
    current = (current as Record<string, unknown>)[segment];
  }
  return current;
}
