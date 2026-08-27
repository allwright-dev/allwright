export type SelectorFlavor = "css" | "xpath";

const SELECTOR_PREFIXES = ["xpath=", "xpath:", "css=", "css:"] as const;

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
): { flavor: SelectorFlavor; prefixLength: number } | null {
  const lowered = selector.toLowerCase();
  if (lowered.startsWith("xpath=") || lowered.startsWith("xpath:")) {
    return { flavor: "xpath", prefixLength: 6 };
  }
  if (lowered.startsWith("css=") || lowered.startsWith("css:")) {
    return { flavor: "css", prefixLength: 4 };
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

export function parseSelectorForTransport(selector: string): { flavor: SelectorFlavor; body: string } {
  const trimmed = selector.trim();
  const lower = trimmed.toLowerCase();
  if (lower.startsWith("xpath=") || lower.startsWith("xpath:")) {
    return { flavor: "xpath", body: decodeSelectorBody(trimmed.slice(6)) };
  }
  if (lower.startsWith("css=") || lower.startsWith("css:")) {
    return { flavor: "css", body: decodeSelectorBody(trimmed.slice(4)) };
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

export function normalizeSelectorForTransport(selector: string): string {
  const trimmed = selector.trim();
  if (isNormalizedTransportSelector(trimmed)) {
    return trimmed;
  }
  const parsed = parseSelectorForTransport(selector);
  return `${parsed.flavor}=${JSON.stringify(parsed.body)}`;
}

export function chainSelectorForTransport(parent: string, child: string): string {
  const parentSelector = normalizeSelectorForTransport(parent);
  const childSelector = normalizeSelectorForTransport(child);
  if (!parentSelector) {
    return childSelector;
  }
  if (!childSelector) {
    return parentSelector;
  }
  return `${parentSelector} ${childSelector}`;
}
