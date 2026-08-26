export type SelectorFlavor = "css" | "xpath";

export function parseSelectorForTransport(selector: string): { flavor: SelectorFlavor; body: string } {
  const trimmed = selector.trim();
  const lower = trimmed.toLowerCase();
  if (lower.startsWith("xpath=") || lower.startsWith("xpath:")) {
    return { flavor: "xpath", body: trimmed.slice(6).trim() };
  }
  if (lower.startsWith("css=") || lower.startsWith("css:")) {
    return { flavor: "css", body: trimmed.slice(4).trim() };
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
