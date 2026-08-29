package dev.allwright.client;

final class AndroidSelectorSupport {
    private AndroidSelectorSupport() {}

    private static final String[] UIAUTOMATOR_SELECTOR_KEYS = {
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
            "instance"
    };

    static String normalizeSelectorForTransport(String selector) {
        String trimmed = selector == null ? "" : selector.trim();
        if (trimmed.isEmpty()) {
            return "";
        }
        if (isNormalizedTransportSelector(trimmed)) {
            return trimmed;
        }
        SelectorTransport parsed = parseSelectorForTransport(trimmed);
        return parsed.kind() + "=" + SelectorSupport.quoteJson(parsed.body());
    }

    static String chainSelectorForTransport(String parent, String child) {
        String normalizedParent = parent == null || parent.trim().isEmpty()
                ? ""
                : normalizeSelectorForTransport(parent);
        String normalizedChild = child == null || child.trim().isEmpty()
                ? ""
                : normalizeSelectorForTransport(child);
        if (normalizedParent.isEmpty()) {
            return normalizedChild;
        }
        if (normalizedChild.isEmpty()) {
            return normalizedParent;
        }
        return normalizedParent + " " + normalizedChild;
    }

    private static SelectorTransport parseSelectorForTransport(String selector) {
        SelectorPrefix explicit = parseExplicitSelectorPrefix(selector);
        if (explicit != null) {
            return new SelectorTransport(explicit.kind(), decodeSelectorBody(selector.substring(explicit.prefixLength())));
        }
        int uiAutomatorPrefix = parseUiAutomatorSelectorPrefix(selector);
        if (uiAutomatorPrefix > 0) {
            return new SelectorTransport("uia", selector.substring(0, uiAutomatorPrefix - 1) + "=" + selector.substring(uiAutomatorPrefix));
        }
        if (
                selector.startsWith("//")
                        || selector.startsWith(".//")
                        || selector.startsWith("../")
                        || selector.startsWith("/")
                        || selector.startsWith("(")
        ) {
            return new SelectorTransport("xpath", selector);
        }
        return new SelectorTransport("css", selector);
    }

    private static SelectorPrefix parseExplicitSelectorPrefix(String selector) {
        String lowered = selector.toLowerCase();
        if (lowered.startsWith("xpath=") || lowered.startsWith("xpath:")) {
            return new SelectorPrefix("xpath", 6);
        }
        if (lowered.startsWith("css=") || lowered.startsWith("css:")) {
            return new SelectorPrefix("css", 4);
        }
        if (lowered.startsWith("uia=") || lowered.startsWith("uia:")) {
            return new SelectorPrefix("uia", 4);
        }
        return null;
    }

    private static int parseUiAutomatorSelectorPrefix(String selector) {
        for (int index = 0; index < selector.length(); index++) {
            char ch = selector.charAt(index);
            if (ch != '=' && ch != ':') {
                continue;
            }
            String key = selector.substring(0, index).trim().toLowerCase();
            for (String candidate : UIAUTOMATOR_SELECTOR_KEYS) {
                if (candidate.equals(key)) {
                    return index + 1;
                }
            }
            return -1;
        }
        return -1;
    }

    private static boolean isNormalizedTransportSelector(String selector) {
        int index = 0;
        while (index < selector.length()) {
            SelectorPrefix prefix = parseExplicitSelectorPrefix(selector.substring(index));
            if (prefix == null) {
                return false;
            }
            index += prefix.prefixLength();

            int jsonEnd = findJsonStringEnd(selector.substring(index));
            if (jsonEnd < 0) {
                return false;
            }
            index += jsonEnd;
            if (index == selector.length()) {
                return true;
            }

            int whitespaceStart = index;
            while (index < selector.length() && Character.isWhitespace(selector.charAt(index))) {
                index++;
            }
            if (index == whitespaceStart) {
                return false;
            }
            if (parseExplicitSelectorPrefix(selector.substring(index)) == null) {
                return false;
            }
        }
        return true;
    }

    private static int findJsonStringEnd(String value) {
        if (!value.startsWith("\"")) {
            return -1;
        }
        boolean escaped = false;
        for (int index = 1; index < value.length(); index++) {
            char ch = value.charAt(index);
            if (escaped) {
                escaped = false;
                continue;
            }
            if (ch == '\\') {
                escaped = true;
                continue;
            }
            if (ch == '"') {
                return index + 1;
            }
        }
        return -1;
    }

    private static String decodeSelectorBody(String body) {
        String candidate = body == null ? "" : body.trim();
        if (candidate.length() >= 2 && candidate.charAt(0) == '"' && candidate.charAt(candidate.length() - 1) == '"') {
            try {
                return SelectorSupport.unescapeJsonString(candidate.substring(1, candidate.length() - 1));
            } catch (IllegalArgumentException ignored) {
                return candidate;
            }
        }
        return candidate;
    }

    private record SelectorTransport(String kind, String body) {}

    private record SelectorPrefix(String kind, int prefixLength) {}
}
