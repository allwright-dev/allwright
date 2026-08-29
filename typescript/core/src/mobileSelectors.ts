const UIAUTOMATOR_SELECTOR_KEYS = new Set([
  "text",
  "textcontains",
  "textmatches",
  "textstartswith",
  "classname",
  "classnamematches",
  "description",
  "desc",
  "descriptioncontains",
  "desccontains",
  "descriptionmatches",
  "descmatches",
  "descriptionstartswith",
  "descstartswith",
  "checkable",
  "checked",
  "clickable",
  "longclickable",
  "scrollable",
  "enabled",
  "focusable",
  "focused",
  "selected",
  "packagename",
  "package",
  "packagenamematches",
  "resourceid",
  "resourceidmatches",
  "index",
  "instance",
]);

const SELECTOR_PREFIXES = ["xpath=", "xpath:", "css=", "css:", "uia=", "uia:"] as const;

type MobileSelectorFlavor = "css" | "xpath" | "uia";

function decodeSelectorBody(body: string): string {
  const candidate = body.trim();
  if (candidate.startsWith("\"") && candidate.endsWith("\"")) {
    try {
      return JSON.parse(candidate) as string;
    } catch {
      return candidate;
    }
  }
  return candidate;
}

function parseExplicitSelectorPrefix(
  selector: string,
): { flavor: MobileSelectorFlavor; prefixLength: number } | null {
  const lowered = selector.toLowerCase();
  if (lowered.startsWith("xpath=") || lowered.startsWith("xpath:")) {
    return { flavor: "xpath", prefixLength: 6 };
  }
  if (lowered.startsWith("css=") || lowered.startsWith("css:")) {
    return { flavor: "css", prefixLength: 4 };
  }
  if (lowered.startsWith("uia=") || lowered.startsWith("uia:")) {
    return { flavor: "uia", prefixLength: 4 };
  }
  return null;
}

function findJsonStringEnd(value: string): number | null {
  if (!value.startsWith("\"")) {
    return null;
  }
  let escaped = false;
  for (let index = 1; index < value.length; index += 1) {
    const char = value[index];
    if (escaped) {
      escaped = false;
      continue;
    }
    if (char === "\\") {
      escaped = true;
      continue;
    }
    if (char === "\"") {
      return index + 1;
    }
  }
  return null;
}

function isNormalizedTransportSelector(selector: string): boolean {
  const trimmed = selector.trim();
  if (!trimmed) {
    return false;
  }

  let index = 0;
  while (index < trimmed.length) {
    const prefix = parseExplicitSelectorPrefix(trimmed.slice(index));
    if (!prefix) {
      return false;
    }

    index += prefix.prefixLength;
    const remainder = trimmed.slice(index);
    if (!remainder.startsWith("\"")) {
      return false;
    }

    const jsonEnd = findJsonStringEnd(remainder);
    if (!jsonEnd) {
      return false;
    }

    index += jsonEnd;
    const tail = trimmed.slice(index);
    if (!tail) {
      return true;
    }
    if (!/^\s+/.test(tail)) {
      return false;
    }

    const nextIndex = index + tail.match(/^\s+/)?.[0].length!;
    const nextSegment = trimmed.slice(nextIndex).toLowerCase();
    if (!SELECTOR_PREFIXES.some((prefixValue) => nextSegment.startsWith(prefixValue))) {
      return false;
    }
    index = nextIndex;
  }

  return true;
}

function parseUiAutomatorSelectorPrefix(selector: string): number | null {
  const separatorIndex = selector.search(/[=:]/);
  if (separatorIndex <= 0) {
    return null;
  }
  const key = selector.slice(0, separatorIndex).trim().toLowerCase();
  if (!UIAUTOMATOR_SELECTOR_KEYS.has(key)) {
    return null;
  }
  return separatorIndex + 1;
}

function parseSelectorForTransport(selector: string): { flavor: MobileSelectorFlavor; body: string } {
  const trimmed = selector.trim();
  const explicit = parseExplicitSelectorPrefix(trimmed);
  if (explicit) {
    return { flavor: explicit.flavor, body: decodeSelectorBody(trimmed.slice(explicit.prefixLength)) };
  }
  const uiAutomatorPrefix = parseUiAutomatorSelectorPrefix(trimmed);
  if (uiAutomatorPrefix) {
    return { flavor: "uia", body: `${trimmed.slice(0, uiAutomatorPrefix - 1)}=${trimmed.slice(uiAutomatorPrefix)}` };
  }
  if (
    trimmed.startsWith("//") ||
    trimmed.startsWith(".//") ||
    trimmed.startsWith("../") ||
    trimmed.startsWith("/") ||
    trimmed.startsWith("(")
  ) {
    return { flavor: "xpath", body: trimmed };
  }
  return { flavor: "css", body: trimmed };
}

export function normalizeMobileSelectorForTransport(selector: string): string {
  const trimmed = selector.trim();
  if (!trimmed) {
    return "";
  }
  if (isNormalizedTransportSelector(trimmed)) {
    return trimmed;
  }
  const parsed = parseSelectorForTransport(selector);
  return `${parsed.flavor}=${JSON.stringify(parsed.body)}`;
}

export function chainMobileSelectorForTransport(parent: string, child: string): string {
  const parentSelector = normalizeMobileSelectorForTransport(parent);
  const childSelector = normalizeMobileSelectorForTransport(child);
  if (!parentSelector) {
    return childSelector;
  }
  if (!childSelector) {
    return parentSelector;
  }
  return `${parentSelector} ${childSelector}`;
}
