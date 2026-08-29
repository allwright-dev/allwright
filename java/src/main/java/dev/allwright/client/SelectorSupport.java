package dev.allwright.client;

final class SelectorSupport {
    private SelectorSupport() {}

    static String normalizeSelectorForTransport(String selector) {
        String trimmed = selector == null ? "" : selector.trim();
        if (isNormalizedTransportSelector(trimmed)) {
            return trimmed;
        }
        SelectorTransport parsed = parseSelectorForTransport(selector);
        return parsed.kind() + "=" + quoteJson(parsed.body());
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
        String trimmed = selector == null ? "" : selector.trim();
        String lowered = trimmed.toLowerCase();
        if (lowered.startsWith("xpath=") || lowered.startsWith("xpath:")) {
            return new SelectorTransport("xpath", decodeSelectorBody(trimmed.substring(6)));
        }
        if (lowered.startsWith("css=") || lowered.startsWith("css:")) {
            return new SelectorTransport("css", decodeSelectorBody(trimmed.substring(4)));
        }
        if (
                trimmed.startsWith("//")
                        || trimmed.startsWith(".//")
                        || trimmed.startsWith("../")
                        || trimmed.startsWith("/")
                        || trimmed.startsWith("(")
        ) {
            return new SelectorTransport("xpath", trimmed);
        }
        return new SelectorTransport("css", trimmed);
    }

    private static SelectorPrefix parseExplicitSelectorPrefix(String selector) {
        String lowered = selector.toLowerCase();
        if (lowered.startsWith("xpath=") || lowered.startsWith("xpath:")) {
            return new SelectorPrefix("xpath", 6);
        }
        if (lowered.startsWith("css=") || lowered.startsWith("css:")) {
            return new SelectorPrefix("css", 4);
        }
        return null;
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

    private static boolean isNormalizedTransportSelector(String selector) {
        if (selector == null || selector.isBlank()) {
            return false;
        }
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

    private static String decodeSelectorBody(String body) {
        String candidate = body == null ? "" : body.trim();
        if (candidate.length() >= 2 && candidate.charAt(0) == '"' && candidate.charAt(candidate.length() - 1) == '"') {
            try {
                return unescapeJsonString(candidate.substring(1, candidate.length() - 1));
            } catch (IllegalArgumentException ignored) {
                return candidate;
            }
        }
        return candidate;
    }

    static String quoteJson(String value) {
        StringBuilder builder = new StringBuilder(value.length() + 2);
        builder.append('"');
        for (int index = 0; index < value.length(); index++) {
            char ch = value.charAt(index);
            switch (ch) {
                case '"' -> builder.append("\\\"");
                case '\\' -> builder.append("\\\\");
                case '\b' -> builder.append("\\b");
                case '\f' -> builder.append("\\f");
                case '\n' -> builder.append("\\n");
                case '\r' -> builder.append("\\r");
                case '\t' -> builder.append("\\t");
                default -> {
                    if (ch < 0x20) {
                        builder.append(String.format("\\u%04x", (int) ch));
                    } else {
                        builder.append(ch);
                    }
                }
            }
        }
        builder.append('"');
        return builder.toString();
    }

    static String unescapeJsonString(String value) {
        StringBuilder builder = new StringBuilder(value.length());
        for (int index = 0; index < value.length(); index++) {
            char ch = value.charAt(index);
            if (ch != '\\') {
                builder.append(ch);
                continue;
            }
            if (index + 1 >= value.length()) {
                throw new IllegalArgumentException("unterminated escape");
            }
            char escaped = value.charAt(++index);
            switch (escaped) {
                case '"', '\\', '/' -> builder.append(escaped);
                case 'b' -> builder.append('\b');
                case 'f' -> builder.append('\f');
                case 'n' -> builder.append('\n');
                case 'r' -> builder.append('\r');
                case 't' -> builder.append('\t');
                case 'u' -> {
                    if (index + 4 >= value.length()) {
                        throw new IllegalArgumentException("invalid unicode escape");
                    }
                    String hex = value.substring(index + 1, index + 5);
                    builder.append((char) Integer.parseInt(hex, 16));
                    index += 4;
                }
                default -> throw new IllegalArgumentException("unsupported escape");
            }
        }
        return builder.toString();
    }

    private record SelectorTransport(String kind, String body) {}

    private record SelectorPrefix(String kind, int prefixLength) {}
}
